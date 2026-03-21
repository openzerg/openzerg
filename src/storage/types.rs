use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

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
    pub system_prompt: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct StoredSessionRow {
    pub id: String,
    pub purpose: String,
    pub state: String,
    pub created_at: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub task_id: Option<String>,
    pub query_id: Option<String>,
    pub message_count: i32,
    pub system_prompt: String,
}

impl From<StoredSessionRow> for StoredSession {
    fn from(row: StoredSessionRow) -> Self {
        Self {
            id: row.id,
            purpose: row.purpose,
            state: row.state,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            started_at: row.started_at.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|d| d.with_timezone(&Utc))
            }),
            finished_at: row.finished_at.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|d| d.with_timezone(&Utc))
            }),
            task_id: row.task_id,
            query_id: row.query_id,
            message_count: row.message_count as usize,
            system_prompt: row.system_prompt,
        }
    }
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

#[derive(Debug, Clone, FromRow)]
pub struct StoredMessageRow {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub timestamp: String,
    pub tool_calls: Option<String>,
}

impl From<StoredMessageRow> for StoredMessage {
    fn from(row: StoredMessageRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            role: MessageRole::from_str(&row.role),
            content: row.content,
            timestamp: DateTime::parse_from_rfc3339(&row.timestamp)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            tool_calls: row.tool_calls.and_then(|s| serde_json::from_str(&s).ok()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

impl MessageRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::System => "System",
            MessageRole::User => "User",
            MessageRole::Assistant => "Assistant",
            MessageRole::Tool => "Tool",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "System" => MessageRole::System,
            "Assistant" => MessageRole::Assistant,
            "Tool" => MessageRole::Tool,
            _ => MessageRole::User,
        }
    }
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

#[derive(Debug, Clone, FromRow)]
pub struct StoredToolResultRow {
    pub tool_call_id: String,
    pub output: String,
    pub success: i32,
}

impl From<StoredToolResultRow> for StoredToolResult {
    fn from(row: StoredToolResultRow) -> Self {
        Self {
            tool_call_id: row.tool_call_id,
            output: row.output,
            success: row.success != 0,
        }
    }
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

#[derive(Debug, Clone, FromRow)]
pub struct StoredProcessRow {
    pub id: String,
    pub command: String,
    pub args: String,
    pub cwd: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub session_id: String,
    pub stdout_size: i64,
    pub stderr_size: i64,
}

impl From<StoredProcessRow> for StoredProcess {
    fn from(row: StoredProcessRow) -> Self {
        Self {
            id: row.id,
            command: row.command,
            args: serde_json::from_str(&row.args).unwrap_or_default(),
            cwd: row.cwd,
            status: row.status,
            exit_code: row.exit_code,
            started_at: DateTime::parse_from_rfc3339(&row.started_at)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            finished_at: row.finished_at.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|d| d.with_timezone(&Utc))
            }),
            session_id: row.session_id,
            stdout_size: row.stdout_size as u64,
            stderr_size: row.stderr_size as u64,
        }
    }
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

#[derive(Debug, Clone, FromRow)]
pub struct StoredActivityRow {
    pub id: String,
    pub session_id: Option<String>,
    pub activity_type: String,
    pub description: String,
    pub details: String,
    pub timestamp: String,
}

impl From<StoredActivityRow> for StoredActivity {
    fn from(row: StoredActivityRow) -> Self {
        Self {
            id: row.id,
            session_id: row.session_id,
            activity_type: ActivityType::from_str(&row.activity_type),
            description: row.description,
            details: serde_json::from_str(&row.details).unwrap_or(serde_json::json!({})),
            timestamp: DateTime::parse_from_rfc3339(&row.timestamp)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        }
    }
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

impl ActivityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivityType::FileRead => "FileRead",
            ActivityType::FileWrite => "FileWrite",
            ActivityType::FileEdit => "FileEdit",
            ActivityType::ProcessStart => "ProcessStart",
            ActivityType::ProcessEnd => "ProcessEnd",
            ActivityType::ToolCall => "ToolCall",
            ActivityType::TaskCreate => "TaskCreate",
            ActivityType::TaskUpdate => "TaskUpdate",
            ActivityType::Message => "Message",
            ActivityType::Thinking => "Thinking",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "FileWrite" => ActivityType::FileWrite,
            "FileEdit" => ActivityType::FileEdit,
            "ProcessStart" => ActivityType::ProcessStart,
            "ProcessEnd" => ActivityType::ProcessEnd,
            "ToolCall" => ActivityType::ToolCall,
            "TaskCreate" => ActivityType::TaskCreate,
            "TaskUpdate" => ActivityType::TaskUpdate,
            "Message" => ActivityType::Message,
            "Thinking" => ActivityType::Thinking,
            _ => ActivityType::FileRead,
        }
    }
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

#[derive(Debug, Clone, FromRow)]
pub struct StoredTaskRow {
    pub id: String,
    pub content: String,
    pub status: String,
    pub priority: String,
    pub session_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

impl From<StoredTaskRow> for StoredTask {
    fn from(row: StoredTaskRow) -> Self {
        Self {
            id: row.id,
            content: row.content,
            status: row.status,
            priority: row.priority,
            session_id: row.session_id,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&row.updated_at)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            completed_at: row.completed_at.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|d| d.with_timezone(&Utc))
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredProvider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: Option<i32>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<i32>,
    pub extra_params: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct StoredProviderRow {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: Option<i32>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<i32>,
    pub extra_params: Option<String>,
    pub is_active: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<StoredProviderRow> for StoredProvider {
    fn from(row: StoredProviderRow) -> Self {
        Self {
            id: row.id,
            name: row.name,
            base_url: row.base_url,
            api_key: row.api_key,
            model: row.model,
            max_tokens: row.max_tokens,
            temperature: row.temperature,
            top_p: row.top_p,
            top_k: row.top_k,
            extra_params: row.extra_params,
            is_active: row.is_active != 0,
            created_at: DateTime::parse_from_rfc3339(&row.created_at)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&row.updated_at)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
        }
    }
}
