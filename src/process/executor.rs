use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader, AsyncWriteExt};
use tokio::fs::File;
use crate::error::{Error, Result};
use super::types::{Process, ProcessStatus};
use chrono::Utc;

const OUTPUT_LIMIT: u64 = 10 * 1024 * 1024;

pub struct Executor {
    run_dir: PathBuf,
}

impl Executor {
    pub fn new(run_dir: PathBuf) -> Self {
        Self { run_dir }
    }

    pub async fn execute(
        &self,
        id: &str,
        command: &str,
        args: &[String],
        cwd: &str,
        env: &std::collections::HashMap<String, String>,
        session_id: &str,
    ) -> Result<Process> {
        let process_dir = self.run_dir.join("processes").join(id);
        tokio::fs::create_dir_all(&process_dir).await?;

        let stdout_path = process_dir.join("stdout.log").display().to_string();
        let stderr_path = process_dir.join("stderr.log").display().to_string();

        let mut cmd = tokio::process::Command::new(command);
        cmd.args(args)
            .current_dir(cwd)
            .envs(env)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let started_at = Utc::now();

        let mut child = cmd.spawn()
            .map_err(|e| Error::Process(format!("Failed to spawn process: {}", e)))?;

        let pid = child.id();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let stdout_file = File::create(&stdout_path).await
            .map_err(|e| Error::Process(format!("Failed to create stdout file: {}", e)))?;
        let stderr_file = File::create(&stderr_path).await
            .map_err(|e| Error::Process(format!("Failed to create stderr file: {}", e)))?;

        let stdout_handle = tokio::spawn(capture_stdout(
            stdout, stdout_file, OUTPUT_LIMIT, process_dir.clone()
        ));

        let stderr_handle = tokio::spawn(capture_stderr(
            stderr, stderr_file, OUTPUT_LIMIT, process_dir.clone()
        ));

        let status = child.wait().await
            .map_err(|e| Error::Process(format!("Failed to wait for process: {}", e)))?;

        let (exit_code, final_status) = if status.success() {
            (Some(0), ProcessStatus::Completed)
        } else {
            let code = status.code();
            (code, ProcessStatus::Failed)
        };

        let finished_at = Utc::now();

        stdout_handle.abort();
        stderr_handle.abort();

        let (stdout_size, stderr_size) = self.get_file_sizes(&stdout_path, &stderr_path).await;
        let output_limit_reached = stdout_size >= OUTPUT_LIMIT || stderr_size >= OUTPUT_LIMIT;

        Ok(Process {
            id: id.to_string(),
            command: command.to_string(),
            args: args.to_vec(),
            cwd: cwd.to_string(),
            env: env.clone(),
            status: final_status,
            pid,
            exit_code,
            started_at,
            finished_at: Some(finished_at),
            session_id: session_id.to_string(),
            stdout_path,
            stderr_path,
            stdout_size,
            stderr_size,
            output_limit_reached,
        })
    }

    async fn get_file_sizes(&self, stdout_path: &str, stderr_path: &str) -> (u64, u64) {
        let stdout_size = tokio::fs::metadata(stdout_path).await
            .map(|m| m.len())
            .unwrap_or(0);
        let stderr_size = tokio::fs::metadata(stderr_path).await
            .map(|m| m.len())
            .unwrap_or(0);
        (stdout_size, stderr_size)
    }

    pub async fn kill(&self, pid: u32) -> Result<()> {
        let status = tokio::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()
            .await
            .map_err(|e| Error::Process(format!("Failed to kill process: {}", e)))?;

        if !status.success() {
            tracing::warn!("Failed to kill process {}", pid);
        }

        Ok(())
    }
}

async fn capture_stdout(
    reader: Option<tokio::process::ChildStdout>,
    file: File,
    limit: u64,
    process_dir: PathBuf,
) {
    if let Some(reader) = reader {
        let mut reader = BufReader::new(reader).lines();
        let mut file = file;
        let mut total_size: u64 = 0;
        let mut limit_reached = false;

        while let Some(line) = reader.next_line().await.ok().flatten() {
            if limit_reached {
                continue;
            }

            let line_with_newline = format!("{}\n", line);
            let line_size = line_with_newline.len() as u64;

            if total_size + line_size > limit {
                let warning = format!("\n\n[OUTPUT LIMIT REACHED: {}MB]\n", limit / 1024 / 1024);
                let _ = file.write_all(warning.as_bytes()).await;
                limit_reached = true;

                let marker_file = process_dir.join("stdout_limit_reached");
                let _ = tokio::fs::write(&marker_file, "true").await;
                continue;
            }

            if file.write_all(line_with_newline.as_bytes()).await.is_err() {
                break;
            }
            total_size += line_size;
        }
    }
}

async fn capture_stderr(
    reader: Option<tokio::process::ChildStderr>,
    file: File,
    limit: u64,
    process_dir: PathBuf,
) {
    if let Some(reader) = reader {
        let mut reader = BufReader::new(reader).lines();
        let mut file = file;
        let mut total_size: u64 = 0;
        let mut limit_reached = false;

        while let Some(line) = reader.next_line().await.ok().flatten() {
            if limit_reached {
                continue;
            }

            let line_with_newline = format!("{}\n", line);
            let line_size = line_with_newline.len() as u64;

            if total_size + line_size > limit {
                let warning = format!("\n\n[OUTPUT LIMIT REACHED: {}MB]\n", limit / 1024 / 1024);
                let _ = file.write_all(warning.as_bytes()).await;
                limit_reached = true;

                let marker_file = process_dir.join("stderr_limit_reached");
                let _ = tokio::fs::write(&marker_file, "true").await;
                continue;
            }

            if file.write_all(line_with_newline.as_bytes()).await.is_err() {
                break;
            }
            total_size += line_size;
        }
    }
}