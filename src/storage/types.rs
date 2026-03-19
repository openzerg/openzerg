use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSession {
    pub id: String,
    pub purpose: String,
    pub state: String,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub task_id: Option<String>,
    pub query_id: Option<String>,
    pub message_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredMessage {
    pub id: String,
    pub session_id: String,
    pub role: MessageRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub tool_calls: Option<Vec<StoredToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToolResult {
    pub tool_call_id: String,
    pub output: String,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredProcess {
    pub id: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
    pub session_id: String,
    pub stdout_size: u64,
    pub stderr_size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredActivity {
    pub id: String,
    pub session_id: Option<String>,
    pub activity_type: ActivityType,
    pub description: String,
    pub details: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityType {
    FileRead,
    FileWrite,
    FileEdit,
    ProcessStart,
    ProcessEnd,
    ToolCall,
    TaskCreate,
    TaskUpdate,
    Message,
    Thinking,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTask {
    pub id: String,
    pub content: String,
    pub status: String,
    pub priority: String,
    pub session_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stored_session_serialization() {
        let session = StoredSession {
            id: "s1".to_string(),
            purpose: "test".to_string(),
            state: "active".to_string(),
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
            task_id: None,
            query_id: None,
            message_count: 0,
        };
        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("s1"));
    }

    #[test]
    fn test_stored_message_serialization() {
        let msg = StoredMessage {
            id: "m1".to_string(),
            session_id: "s1".to_string(),
            role: MessageRole::User,
            content: "hello".to_string(),
            timestamp: Utc::now(),
            tool_calls: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("hello"));
    }

    #[test]
    fn test_message_role_serialization() {
        let role = MessageRole::Assistant;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"Assistant\"");
    }

    #[test]
    fn test_message_role_deserialization() {
        let json = "\"User\"";
        let role: MessageRole = serde_json::from_str(json).unwrap();
        assert!(matches!(role, MessageRole::User));
    }

    #[test]
    fn test_stored_tool_call_serialization() {
        let call = StoredToolCall {
            id: "c1".to_string(),
            name: "read".to_string(),
            arguments: "{}".to_string(),
        };
        let json = serde_json::to_string(&call).unwrap();
        assert!(json.contains("read"));
    }

    #[test]
    fn test_stored_tool_result_serialization() {
        let result = StoredToolResult {
            tool_call_id: "c1".to_string(),
            output: "success".to_string(),
            success: true,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("success"));
    }

    #[test]
    fn test_stored_process_serialization() {
        let process = StoredProcess {
            id: "p1".to_string(),
            command: "ls".to_string(),
            args: vec!["-la".to_string()],
            cwd: "/tmp".to_string(),
            status: "completed".to_string(),
            exit_code: Some(0),
            started_at: Utc::now(),
            finished_at: Some(Utc::now()),
            session_id: "s1".to_string(),
            stdout_size: 100,
            stderr_size: 0,
        };
        let json = serde_json::to_string(&process).unwrap();
        assert!(json.contains("ls"));
    }

    #[test]
    fn test_stored_activity_serialization() {
        let activity = StoredActivity {
            id: "a1".to_string(),
            session_id: Some("s1".to_string()),
            activity_type: ActivityType::FileRead,
            description: "reading file".to_string(),
            details: serde_json::json!({}),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&activity).unwrap();
        assert!(json.contains("reading"));
    }

    #[test]
    fn test_activity_type_serialization() {
        let at = ActivityType::ToolCall;
        let json = serde_json::to_string(&at).unwrap();
        assert_eq!(json, "\"ToolCall\"");
    }

    #[test]
    fn test_stored_task_serialization() {
        let task = StoredTask {
            id: "t1".to_string(),
            content: "do something".to_string(),
            status: "pending".to_string(),
            priority: "high".to_string(),
            session_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
        };
        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("do something"));
    }
}
