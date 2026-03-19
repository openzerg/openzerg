use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use chrono::Utc;
use tokio::io::AsyncWriteExt;
use crate::error::{Error, Result};
use super::types::{Process, ProcessStatus};

const DEFAULT_MEMORY_MAX: &str = "512M";
const DEFAULT_TASKS_MAX: u32 = 100;
const OUTPUT_LIMIT: u64 = 10 * 1024 * 1024;

const OPENZERG_SLICE: &str = r#"[Unit]
Description=Slice for OpenZerg agent processes
DefaultDependencies=no
Before=slices.target

[Slice]
MemoryMax=2G
TasksMax=500
CPUQuota=80%
"#;

pub struct SystemdExecutor {
    run_dir: PathBuf,
    slice: String,
}

impl SystemdExecutor {
    pub fn new(run_dir: PathBuf) -> Self {
        Self {
            run_dir,
            slice: "openzerg.slice".to_string(),
        }
    }
    
    pub async fn ensure_slice(&self) -> Result<()> {
        let slice_path = "/etc/systemd/system/openzerg.slice";
        
        if !std::path::Path::new(slice_path).exists() {
            tokio::fs::write(slice_path, OPENZERG_SLICE).await
                .map_err(|e| Error::Process(format!("Failed to write slice file: {}", e)))?;
            
            let status = tokio::process::Command::new("systemctl")
                .args(["daemon-reload"])
                .status()
                .await
                .map_err(|e| Error::Process(format!("Failed to reload systemd: {}", e)))?;
            
            if !status.success() {
                tracing::warn!("systemctl daemon-reload failed");
            }
        }
        
        Ok(())
    }
    
    pub async fn execute(
        &self,
        command: &str,
        workdir: &PathBuf,
        env: &HashMap<String, String>,
        timeout_ms: u64,
        session_id: &str,
    ) -> Result<Process> {
        let process_id = uuid::Uuid::new_v4().to_string();
        let unit_name = format!("openzerg-{}.scope", process_id);
        
        let output_dir = self.run_dir.join(&process_id);
        tokio::fs::create_dir_all(&output_dir).await
            .map_err(|e| Error::Process(format!("Failed to create output dir: {}", e)))?;
        
        let stdout_path = output_dir.join("stdout.log");
        let stderr_path = output_dir.join("stderr.log");
        let metadata_path = output_dir.join("metadata.json");
        
        let timeout_secs = timeout_ms / 1000 + 30;
        
        let mut cmd = tokio::process::Command::new("systemd-run");
        cmd.args([
            "--scope",
            "--unit", &unit_name,
            "--slice", &self.slice,
            "-p", "KillMode=control-group",
            "-p", &format!("MemoryMax={}", DEFAULT_MEMORY_MAX),
            "-p", &format!("TasksMax={}", DEFAULT_TASKS_MAX),
            "-p", &format!("TimeoutStopSec={}s", timeout_secs),
            "--working-directory", &workdir.display().to_string(),
        ]);
        
        for (k, v) in env {
            cmd.arg("--setenv").arg(format!("{}={}", k, v));
        }
        
        cmd.args(["--", "sh", "-c", command]);
        
        let stdout_file = tokio::fs::File::create(&stdout_path).await
            .map_err(|e| Error::Process(format!("Failed to create stdout file: {}", e)))?;
        let stderr_file = tokio::fs::File::create(&stderr_path).await
            .map_err(|e| Error::Process(format!("Failed to create stderr file: {}", e)))?;
        
        cmd.stdout(Stdio::from(stdout_file.into_std().await));
        cmd.stderr(Stdio::from(stderr_file.into_std().await));
        
        let started_at = Utc::now();
        
        let status = cmd.status().await
            .map_err(|e| Error::Process(format!("Failed to execute command: {}", e)))?;
        
        let finished_at = Utc::now();
        
        let stdout_size = tokio::fs::metadata(&stdout_path).await
            .map(|m| m.len())
            .unwrap_or(0);
        let stderr_size = tokio::fs::metadata(&stderr_path).await
            .map(|m| m.len())
            .unwrap_or(0);
        
        let (exit_code, final_status) = if status.success() {
            (Some(0), ProcessStatus::Completed)
        } else {
            let code = status.code();
            (code, ProcessStatus::Failed)
        };
        
        let metadata = serde_json::json!({
            "process_id": process_id,
            "unit_name": unit_name,
            "command": command,
            "workdir": workdir.display().to_string(),
            "started_at": started_at.to_rfc3339(),
            "finished_at": finished_at.to_rfc3339(),
            "exit_code": exit_code,
            "stdout_size": stdout_size,
            "stderr_size": stderr_size,
        });
        
        tokio::fs::write(&metadata_path, serde_json::to_string_pretty(&metadata).unwrap()).await.ok();
        
        Ok(Process {
            id: process_id,
            command: command.to_string(),
            args: vec![],
            cwd: workdir.display().to_string(),
            env: env.clone(),
            status: final_status,
            pid: None,
            exit_code,
            started_at,
            finished_at: Some(finished_at),
            session_id: session_id.to_string(),
            stdout_path: stdout_path.display().to_string(),
            stderr_path: stderr_path.display().to_string(),
            stdout_size,
            stderr_size,
            output_limit_reached: stdout_size >= OUTPUT_LIMIT || stderr_size >= OUTPUT_LIMIT,
        })
    }
    
    pub async fn kill(&self, unit_name: &str) -> Result<()> {
        let status = tokio::process::Command::new("systemctl")
            .args(["stop", unit_name])
            .status()
            .await
            .map_err(|e| Error::Process(format!("Failed to stop unit: {}", e)))?;
        
        if !status.success() {
            tracing::warn!("Failed to stop unit {}", unit_name);
        }
        
        Ok(())
    }
    
    pub async fn is_active(&self, unit_name: &str) -> bool {
        tokio::process::Command::new("systemctl")
            .args(["is-active", "--quiet", unit_name])
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }
    
    pub async fn get_unit_name(&self, process_id: &str) -> String {
        format!("openzerg-{}.scope", process_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systemd_executor_new() {
        let executor = SystemdExecutor::new(PathBuf::from("/tmp"));
        assert!(true);
    }

    #[test]
    fn test_slice_content() {
        assert!(OPENZERG_SLICE.contains("OpenZerg"));
        assert!(OPENZERG_SLICE.contains("MemoryMax=2G"));
    }

    #[test]
    fn test_default_memory_max() {
        assert_eq!(DEFAULT_MEMORY_MAX, "512M");
    }

    #[test]
    fn test_default_tasks_max() {
        assert_eq!(DEFAULT_TASKS_MAX, 100);
    }

    #[test]
    fn test_output_limit() {
        assert_eq!(OUTPUT_LIMIT, 10 * 1024 * 1024);
    }

    #[tokio::test]
    async fn test_get_unit_name() {
        let executor = SystemdExecutor::new(PathBuf::from("/tmp"));
        let unit_name = executor.get_unit_name("test-id").await;
        assert_eq!(unit_name, "openzerg-test-id.scope");
    }
}