use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::broadcast;
use crate::error::{Error, Result};
use crate::protocol::{ProcessEvent, AgentEvent};
use super::types::{Process, ProcessStatus, ProcessStats, RunRequest, ReadOutputRequest, StreamType};
use super::executor::Executor;

pub struct ProcessManager {
    processes: Arc<RwLock<HashMap<String, Process>>>,
    executor: Executor,
    event_tx: broadcast::Sender<AgentEvent>,
}

impl ProcessManager {
    pub fn new(run_dir: PathBuf, event_tx: broadcast::Sender<AgentEvent>) -> Self {
        Self {
            processes: Arc::new(RwLock::new(HashMap::new())),
            executor: Executor::new(run_dir),
            event_tx,
        }
    }

    pub async fn run(
        &self,
        request: RunRequest,
        session_id: &str,
    ) -> Result<Process> {
        let id = uuid::Uuid::new_v4().to_string();
        let cwd = request.cwd.as_deref().unwrap_or("/workspace");
        let env = request.env.unwrap_or_default();

        let process = self.executor.execute(
            &id,
            &request.command,
            &request.args,
            cwd,
            &env,
            session_id,
        ).await?;

        let status = process.status;
        let process_id = process.id.clone();
        let exit_code = process.exit_code;

        self.processes.write().await.insert(id.clone(), process.clone());

        let _ = self.event_tx.send(AgentEvent::ProcessNotification {
            process_id: id.clone(),
            event: ProcessEvent::Started,
            output_preview: None,
        });

        if status != ProcessStatus::Running {
            let event = match status {
                ProcessStatus::Completed => ProcessEvent::Completed { exit_code: exit_code.unwrap_or(0) },
                ProcessStatus::Failed => ProcessEvent::Failed { error: format!("Exit code: {:?}", exit_code) },
                _ => ProcessEvent::Failed { error: "Unknown status".into() },
            };
            
            let _ = self.event_tx.send(AgentEvent::ProcessNotification {
                process_id,
                event,
                output_preview: None,
            });
        }

        Ok(process)
    }

    pub async fn kill(&self, id: &str) -> Result<()> {
        let mut processes = self.processes.write().await;
        
        if let Some(process) = processes.get_mut(id) {
            if process.status == ProcessStatus::Running {
                if let Some(pid) = process.pid {
                    self.executor.kill(pid).await?;
                }
                process.status = ProcessStatus::Killed;
                process.finished_at = Some(chrono::Utc::now());
            }
        }

        Ok(())
    }

    pub async fn get(&self, id: &str) -> Option<Process> {
        self.processes.read().await.get(id).cloned()
    }

    pub async fn list(&self, status: Option<ProcessStatus>) -> Vec<Process> {
        let processes = self.processes.read().await;
        
        match status {
            Some(s) => processes.values()
                .filter(|p| p.status == s)
                .cloned()
                .collect(),
            None => processes.values().cloned().collect(),
        }
    }

    pub async fn stats(&self) -> ProcessStats {
        let processes = self.processes.read().await;
        
        let running = processes.values().filter(|p| p.status == ProcessStatus::Running).count();
        let completed = processes.values().filter(|p| p.status == ProcessStatus::Completed).count();
        let failed = processes.values().filter(|p| p.status == ProcessStatus::Failed).count();
        let killed = processes.values().filter(|p| p.status == ProcessStatus::Killed).count();
        let total = processes.len();

        ProcessStats {
            running,
            completed,
            failed,
            killed,
            total,
        }
    }

    pub async fn read_output(
        &self,
        id: &str,
        request: ReadOutputRequest,
    ) -> Result<String> {
        let process = self.get(id).await
            .ok_or_else(|| Error::NotFound(format!("Process {} not found", id)))?;

        let path = match request.stream {
            StreamType::Stdout => process.stdout_path.clone(),
            StreamType::Stderr => process.stderr_path.clone(),
            StreamType::Both => {
                let stdout = self.read_file_with_options(&process.stdout_path, &request).await?;
                let stderr = self.read_file_with_options(&process.stderr_path, &request).await?;
                return Ok(format!("=== STDOUT ===\n{}\n\n=== STDERR ===\n{}", stdout, stderr));
            }
        };

        self.read_file_with_options(&path, &request).await
    }

    async fn read_file_with_options(
        &self,
        path: &str,
        request: &ReadOutputRequest,
    ) -> Result<String> {
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| Error::Io(e))?;

        let lines: Vec<&str> = content.lines().collect();

        let filtered: Vec<&str> = if let Some(ref grep) = request.grep {
            lines.iter()
                .filter(|line| line.contains(grep))
                .copied()
                .collect()
        } else {
            lines
        };

        let start = match request.offset {
            Some(offset) if offset < 0 => {
                let abs = (-offset) as usize;
                if abs > filtered.len() { 0 } else { filtered.len() - abs }
            }
            Some(offset) => offset as usize,
            None => 0,
        };

        let limit = request.limit.unwrap_or(100);
        let end = std::cmp::min(start + limit, filtered.len());

        Ok(filtered[start..end].join("\n"))
    }

    pub async fn cleanup_completed(&self, max_age_hours: u64) -> Result<usize> {
        let mut processes = self.processes.write().await;
        let now = chrono::Utc::now();
        let mut removed = 0;

        let to_remove: Vec<String> = processes
            .iter()
            .filter(|(_, p)| {
                p.status != ProcessStatus::Running &&
                p.finished_at.map(|f| {
                    let age = now - f;
                    age.num_hours() >= max_age_hours as i64
                }).unwrap_or(false)
            })
            .map(|(id, _)| id.clone())
            .collect();

        for id in to_remove {
            processes.remove(&id);
            removed += 1;
        }

        Ok(removed)
    }
}