use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Process {
    pub id: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub env: HashMap<String, String>,
    pub status: ProcessStatus,
    pub pid: Option<u32>,
    pub exit_code: Option<i32>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub session_id: String,
    pub stdout_path: String,
    pub stderr_path: String,
    pub stdout_size: u64,
    pub stderr_size: u64,
    pub output_limit_reached: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessStatus {
    Running,
    Completed,
    Failed,
    Killed,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessStats {
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
    pub killed: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRequest {
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: Option<HashMap<String, String>>,
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadOutputRequest {
    pub stream: StreamType,
    pub offset: Option<i64>,
    pub limit: Option<usize>,
    pub grep: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamType {
    Stdout,
    Stderr,
    Both,
}

impl Process {
    pub fn duration(&self) -> Option<std::time::Duration> {
        self.finished_at.map(|end| {
            (end - self.started_at)
                .to_std()
                .unwrap_or(std::time::Duration::ZERO)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_status_variants() {
        assert_eq!(ProcessStatus::Running, ProcessStatus::Running);
        assert_ne!(ProcessStatus::Running, ProcessStatus::Completed);
        assert_ne!(ProcessStatus::Failed, ProcessStatus::Killed);
    }

    #[test]
    fn test_process_status_serialization() {
        let status = ProcessStatus::Completed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"Completed\"");
    }

    #[test]
    fn test_process_status_deserialization() {
        let json = "\"Running\"";
        let status: ProcessStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status, ProcessStatus::Running);
    }

    #[test]
    fn test_stream_type_variants() {
        assert_eq!(StreamType::Stdout, StreamType::Stdout);
        assert_ne!(StreamType::Stdout, StreamType::Stderr);
        assert_ne!(StreamType::Both, StreamType::Stdout);
    }

    #[test]
    fn test_process_duration_no_finish() {
        let process = Process {
            id: "proc-1".to_string(),
            command: "ls".to_string(),
            args: vec![],
            cwd: "/tmp".to_string(),
            env: HashMap::new(),
            status: ProcessStatus::Running,
            pid: Some(1234),
            exit_code: None,
            started_at: Utc::now(),
            finished_at: None,
            session_id: "session-1".to_string(),
            stdout_path: "/tmp/out".to_string(),
            stderr_path: "/tmp/err".to_string(),
            stdout_size: 0,
            stderr_size: 0,
            output_limit_reached: false,
        };
        assert!(process.duration().is_none());
    }

    #[test]
    fn test_process_duration_with_finish() {
        let now = Utc::now();
        let later = now + chrono::Duration::seconds(10);
        let process = Process {
            id: "proc-1".to_string(),
            command: "ls".to_string(),
            args: vec![],
            cwd: "/tmp".to_string(),
            env: HashMap::new(),
            status: ProcessStatus::Completed,
            pid: Some(1234),
            exit_code: Some(0),
            started_at: now,
            finished_at: Some(later),
            session_id: "session-1".to_string(),
            stdout_path: "/tmp/out".to_string(),
            stderr_path: "/tmp/err".to_string(),
            stdout_size: 100,
            stderr_size: 0,
            output_limit_reached: false,
        };
        let duration = process.duration().unwrap();
        assert!(duration.as_secs() >= 10);
    }

    #[test]
    fn test_process_stats_serialization() {
        let stats = ProcessStats {
            running: 1,
            completed: 5,
            failed: 2,
            killed: 0,
            total: 8,
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("running"));
        assert!(json.contains("5"));
    }
}
