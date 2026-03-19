mod types;

pub use types::*;

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;
use anyhow::Result;

pub struct Storage {
    base_path: PathBuf,
    session_file: Arc<RwLock<Option<File>>>,
    message_file: Arc<RwLock<Option<File>>>,
    process_file: Arc<RwLock<Option<File>>>,
    activity_file: Arc<RwLock<Option<File>>>,
    task_file: Arc<RwLock<Option<File>>>,
    tool_result_file: Arc<RwLock<Option<File>>>,
}

impl Storage {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            base_path,
            session_file: Arc::new(RwLock::new(None)),
            message_file: Arc::new(RwLock::new(None)),
            process_file: Arc::new(RwLock::new(None)),
            activity_file: Arc::new(RwLock::new(None)),
            task_file: Arc::new(RwLock::new(None)),
            tool_result_file: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn init(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.base_path).await?;
        
        let session_path = self.base_path.join("sessions.jsonl");
        let message_path = self.base_path.join("messages.jsonl");
        let process_path = self.base_path.join("processes.jsonl");
        let activity_path = self.base_path.join("activities.jsonl");
        let task_path = self.base_path.join("tasks.jsonl");
        let tool_result_path = self.base_path.join("tool_results.jsonl");

        *self.session_file.write().await = Some(
            OpenOptions::new().create(true).append(true).open(&session_path).await?
        );
        *self.message_file.write().await = Some(
            OpenOptions::new().create(true).append(true).open(&message_path).await?
        );
        *self.process_file.write().await = Some(
            OpenOptions::new().create(true).append(true).open(&process_path).await?
        );
        *self.activity_file.write().await = Some(
            OpenOptions::new().create(true).append(true).open(&activity_path).await?
        );
        *self.task_file.write().await = Some(
            OpenOptions::new().create(true).append(true).open(&task_path).await?
        );
        *self.tool_result_file.write().await = Some(
            OpenOptions::new().create(true).append(true).open(&tool_result_path).await?
        );

        Ok(())
    }

    pub async fn save_session(&self, session: &StoredSession) -> Result<()> {
        let mut file = self.session_file.write().await;
        if let Some(f) = file.as_mut() {
            let line = serde_json::to_string(session)? + "\n";
            f.write_all(line.as_bytes()).await?;
            f.flush().await?;
        }
        Ok(())
    }

    pub async fn save_message(&self, message: &StoredMessage) -> Result<()> {
        let mut file = self.message_file.write().await;
        if let Some(f) = file.as_mut() {
            let line = serde_json::to_string(message)? + "\n";
            f.write_all(line.as_bytes()).await?;
            f.flush().await?;
        }
        Ok(())
    }

    pub async fn save_process(&self, process: &StoredProcess) -> Result<()> {
        let mut file = self.process_file.write().await;
        if let Some(f) = file.as_mut() {
            let line = serde_json::to_string(process)? + "\n";
            f.write_all(line.as_bytes()).await?;
            f.flush().await?;
        }
        Ok(())
    }

    pub async fn save_activity(&self, activity: &StoredActivity) -> Result<()> {
        let mut file = self.activity_file.write().await;
        if let Some(f) = file.as_mut() {
            let line = serde_json::to_string(activity)? + "\n";
            f.write_all(line.as_bytes()).await?;
            f.flush().await?;
        }
        Ok(())
    }

    pub async fn save_task(&self, task: &StoredTask) -> Result<()> {
        let mut file = self.task_file.write().await;
        if let Some(f) = file.as_mut() {
            let line = serde_json::to_string(task)? + "\n";
            f.write_all(line.as_bytes()).await?;
            f.flush().await?;
        }
        Ok(())
    }

    pub async fn load_sessions(&self) -> Result<Vec<StoredSession>> {
        let path = self.base_path.join("sessions.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = tokio::fs::read_to_string(&path).await?;
        let mut sessions = Vec::new();
        for line in content.lines() {
            if !line.is_empty() {
                if let Ok(session) = serde_json::from_str::<StoredSession>(line) {
                    sessions.push(session);
                }
            }
        }
        Ok(sessions)
    }

    pub async fn load_messages(&self, session_id: Option<&str>) -> Result<Vec<StoredMessage>> {
        let path = self.base_path.join("messages.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = tokio::fs::read_to_string(&path).await?;
        let mut messages = Vec::new();
        for line in content.lines() {
            if !line.is_empty() {
                if let Ok(msg) = serde_json::from_str::<StoredMessage>(line) {
                    if session_id.is_none() || msg.session_id == session_id.unwrap() {
                        messages.push(msg);
                    }
                }
            }
        }
        Ok(messages)
    }

    pub async fn load_processes(&self) -> Result<Vec<StoredProcess>> {
        let path = self.base_path.join("processes.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = tokio::fs::read_to_string(&path).await?;
        let mut processes = Vec::new();
        for line in content.lines() {
            if !line.is_empty() {
                if let Ok(p) = serde_json::from_str::<StoredProcess>(line) {
                    processes.push(p);
                }
            }
        }
        Ok(processes)
    }

    pub async fn load_activities(&self, session_id: Option<&str>) -> Result<Vec<StoredActivity>> {
        let path = self.base_path.join("activities.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = tokio::fs::read_to_string(&path).await?;
        let mut activities = Vec::new();
        for line in content.lines() {
            if !line.is_empty() {
                if let Ok(a) = serde_json::from_str::<StoredActivity>(line) {
                    if session_id.is_none() || a.session_id.as_deref() == session_id {
                        activities.push(a);
                    }
                }
            }
        }
        Ok(activities)
    }

    pub async fn load_tasks(&self) -> Result<Vec<StoredTask>> {
        let path = self.base_path.join("tasks.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = tokio::fs::read_to_string(&path).await?;
        let mut tasks = Vec::new();
        for line in content.lines() {
            if !line.is_empty() {
                if let Ok(t) = serde_json::from_str::<StoredTask>(line) {
                    tasks.push(t);
                }
            }
        }
        Ok(tasks)
    }

    pub async fn read_process_output(&self, process_id: &str, stream: &str) -> Result<String> {
        let path = self.base_path.join("process_outputs").join(format!("{}_{}.txt", process_id, stream));
        if path.exists() {
            Ok(tokio::fs::read_to_string(&path).await?)
        } else {
            Ok(String::new())
        }
    }
    
    pub async fn save_tool_result(&self, result: &StoredToolResult) -> Result<()> {
        let mut file = self.tool_result_file.write().await;
        if let Some(f) = file.as_mut() {
            let line = serde_json::to_string(result)? + "\n";
            f.write_all(line.as_bytes()).await?;
            f.flush().await?;
        }
        Ok(())
    }
    
    pub async fn get_messages(&self, session_id: &str) -> Result<Vec<StoredMessage>> {
        self.load_messages(Some(session_id)).await
    }
    
    pub async fn get_tool_results(&self, session_id: &str) -> Result<Vec<StoredToolResult>> {
        let path = self.base_path.join("tool_results.jsonl");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let content = tokio::fs::read_to_string(&path).await?;
        let mut results = Vec::new();
        for line in content.lines() {
            if !line.is_empty() {
                if let Ok(r) = serde_json::from_str::<StoredToolResult>(line) {
                    results.push(r);
                }
            }
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_storage_new() {
        let dir = tempdir().unwrap();
        let storage = Storage::new(dir.path().to_path_buf());
        let guard = storage.session_file.read().await;
        assert!(guard.is_none());
    }

    #[tokio::test]
    async fn test_storage_init() {
        let dir = tempdir().unwrap();
        let storage = Storage::new(dir.path().to_path_buf());
        storage.init().await.unwrap();
        
        let session_guard = storage.session_file.read().await;
        let message_guard = storage.message_file.read().await;
        let process_guard = storage.process_file.read().await;
        assert!(session_guard.is_some());
        assert!(message_guard.is_some());
        assert!(process_guard.is_some());
    }

    #[tokio::test]
    async fn test_storage_load_empty() {
        let dir = tempdir().unwrap();
        let storage = Storage::new(dir.path().to_path_buf());
        storage.init().await.unwrap();
        
        let sessions = storage.load_sessions().await.unwrap();
        assert!(sessions.is_empty());
        
        let messages = storage.load_messages(None).await.unwrap();
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_storage_save_and_load_session() {
        let dir = tempdir().unwrap();
        let storage = Storage::new(dir.path().to_path_buf());
        storage.init().await.unwrap();
        
        let session = StoredSession {
            id: "test-session".to_string(),
            state: "active".to_string(),
            purpose: "test".to_string(),
            created_at: chrono::Utc::now(),
            started_at: None,
            finished_at: None,
            task_id: None,
            query_id: None,
            message_count: 0,
        };
        
        storage.save_session(&session).await.unwrap();
        
        let loaded = storage.load_sessions().await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "test-session");
    }

    #[tokio::test]
    async fn test_storage_save_and_load_message() {
        let dir = tempdir().unwrap();
        let storage = Storage::new(dir.path().to_path_buf());
        storage.init().await.unwrap();
        
        let message = StoredMessage {
            id: "msg-1".to_string(),
            session_id: "test-session".to_string(),
            role: MessageRole::User,
            content: "Hello".to_string(),
            timestamp: chrono::Utc::now(),
            tool_calls: None,
        };
        
        storage.save_message(&message).await.unwrap();
        
        let loaded = storage.load_messages(Some("test-session")).await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].content, "Hello");
    }

    #[tokio::test]
    async fn test_storage_save_and_load_process() {
        let dir = tempdir().unwrap();
        let storage = Storage::new(dir.path().to_path_buf());
        storage.init().await.unwrap();
        
        let process = StoredProcess {
            id: "proc-1".to_string(),
            command: "ls".to_string(),
            args: vec![],
            cwd: "/tmp".to_string(),
            status: "completed".to_string(),
            exit_code: Some(0),
            started_at: chrono::Utc::now(),
            finished_at: Some(chrono::Utc::now()),
            session_id: "session-1".to_string(),
            stdout_size: 100,
            stderr_size: 0,
        };
        
        storage.save_process(&process).await.unwrap();
        
        let loaded = storage.load_processes().await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].command, "ls");
    }

    #[tokio::test]
    async fn test_storage_save_and_load_task() {
        let dir = tempdir().unwrap();
        let storage = Storage::new(dir.path().to_path_buf());
        storage.init().await.unwrap();
        
        let task = StoredTask {
            id: "task-1".to_string(),
            content: "Test task".to_string(),
            status: "pending".to_string(),
            priority: "medium".to_string(),
            session_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            completed_at: None,
        };
        
        storage.save_task(&task).await.unwrap();
        
        let loaded = storage.load_tasks().await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].content, "Test task");
    }

    #[tokio::test]
    async fn test_storage_save_and_load_activity() {
        let dir = tempdir().unwrap();
        let storage = Storage::new(dir.path().to_path_buf());
        storage.init().await.unwrap();
        
        let activity = StoredActivity {
            id: "act-1".to_string(),
            session_id: Some("session-1".to_string()),
            activity_type: ActivityType::FileRead,
            description: "Reading file".to_string(),
            details: serde_json::json!({}),
            timestamp: chrono::Utc::now(),
        };
        
        storage.save_activity(&activity).await.unwrap();
        
        let loaded = storage.load_activities(Some("session-1")).await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].description, "Reading file");
    }

    #[tokio::test]
    async fn test_storage_read_process_output_nonexistent() {
        let dir = tempdir().unwrap();
        let storage = Storage::new(dir.path().to_path_buf());
        storage.init().await.unwrap();
        
        let output = storage.read_process_output("nonexistent", "stdout").await.unwrap();
        assert!(output.is_empty());
    }

    #[tokio::test]
    async fn test_storage_get_messages() {
        let dir = tempdir().unwrap();
        let storage = Storage::new(dir.path().to_path_buf());
        storage.init().await.unwrap();
        
        let message = StoredMessage {
            id: "msg-2".to_string(),
            session_id: "session-1".to_string(),
            role: MessageRole::User,
            content: "Test message".to_string(),
            timestamp: chrono::Utc::now(),
            tool_calls: None,
        };
        
        storage.save_message(&message).await.unwrap();
        
        let loaded = storage.get_messages("session-1").await.unwrap();
        assert_eq!(loaded.len(), 1);
    }

    #[tokio::test]
    async fn test_storage_save_tool_result() {
        let dir = tempdir().unwrap();
        let storage = Storage::new(dir.path().to_path_buf());
        storage.init().await.unwrap();
        
        let result = StoredToolResult {
            tool_call_id: "call-1".to_string(),
            output: "success".to_string(),
            success: true,
        };
        
        storage.save_tool_result(&result).await.unwrap();
    }
}