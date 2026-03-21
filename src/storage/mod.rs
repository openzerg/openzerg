mod types;

pub use types::*;

use std::path::PathBuf;
use std::sync::Arc;
use std::str::FromStr;
use tokio::sync::OnceCell;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use anyhow::Result;

pub struct Storage {
    pool: OnceCell<SqlitePool>,
    base_path: PathBuf,
}

impl Storage {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            pool: OnceCell::new(),
            base_path,
        }
    }

    pub fn base_path(&self) -> &PathBuf {
        &self.base_path
    }

    pub async fn init(&self) -> Result<()> {
        tokio::fs::create_dir_all(&self.base_path).await?;
        
        let db_path = self.base_path.join("openzerg.db");
        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());
        
        let options = SqliteConnectOptions::from_str(&db_url)?
            .create_if_missing(true)
            .foreign_keys(true);
        
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        
        self.run_migrations(&pool).await?;
        
        self.pool.set(pool).map_err(|_| anyhow::anyhow!("Pool already initialized"))?;
        
        Ok(())
    }

    async fn run_migrations(&self, pool: &SqlitePool) -> Result<()> {
        sqlx::query(r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                purpose TEXT NOT NULL,
                state TEXT NOT NULL,
                created_at TEXT NOT NULL,
                started_at TEXT,
                finished_at TEXT,
                task_id TEXT,
                query_id TEXT,
                message_count INTEGER DEFAULT 0
            );
            
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                tool_calls TEXT,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );
            
            CREATE TABLE IF NOT EXISTS processes (
                id TEXT PRIMARY KEY,
                command TEXT NOT NULL,
                args TEXT NOT NULL,
                cwd TEXT NOT NULL,
                status TEXT NOT NULL,
                exit_code INTEGER,
                started_at TEXT NOT NULL,
                finished_at TEXT,
                session_id TEXT NOT NULL,
                stdout_size INTEGER DEFAULT 0,
                stderr_size INTEGER DEFAULT 0,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );
            
            CREATE TABLE IF NOT EXISTS activities (
                id TEXT PRIMARY KEY,
                session_id TEXT,
                activity_type TEXT NOT NULL,
                description TEXT NOT NULL,
                details TEXT NOT NULL,
                timestamp TEXT NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                status TEXT NOT NULL,
                priority TEXT NOT NULL,
                session_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT
            );
            
            CREATE TABLE IF NOT EXISTS tool_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                tool_call_id TEXT NOT NULL,
                session_id TEXT,
                output TEXT NOT NULL,
                success INTEGER NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS providers (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                base_url TEXT NOT NULL,
                api_key TEXT NOT NULL,
                model TEXT NOT NULL,
                max_tokens INTEGER,
                temperature REAL,
                top_p REAL,
                top_k INTEGER,
                extra_params TEXT,
                is_active INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            
            CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
            CREATE INDEX IF NOT EXISTS idx_processes_session ON processes(session_id);
            CREATE INDEX IF NOT EXISTS idx_activities_session ON activities(session_id);
            CREATE INDEX IF NOT EXISTS idx_tasks_session ON tasks(session_id);
            CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);
        "#)
        .execute(pool)
        .await?;
        
        Ok(())
    }

    fn pool(&self) -> &SqlitePool {
        self.pool.get().expect("Storage not initialized")
    }

    pub async fn save_session(&self, session: &StoredSession) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO sessions (id, purpose, state, created_at, started_at, finished_at, task_id, query_id, message_count)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                state = excluded.state,
                started_at = excluded.started_at,
                finished_at = excluded.finished_at,
                task_id = excluded.task_id,
                query_id = excluded.query_id,
                message_count = excluded.message_count
        "#)
        .bind(&session.id)
        .bind(&session.purpose)
        .bind(&session.state)
        .bind(session.created_at.to_rfc3339())
        .bind(session.started_at.map(|t| t.to_rfc3339()))
        .bind(session.finished_at.map(|t| t.to_rfc3339()))
        .bind(&session.task_id)
        .bind(&session.query_id)
        .bind(session.message_count as i32)
        .execute(self.pool())
        .await?;
        
        Ok(())
    }

    pub async fn update_session_state(&self, id: &str, state: &str) -> Result<()> {
        sqlx::query("UPDATE sessions SET state = ? WHERE id = ?")
            .bind(state)
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn finish_session(&self, id: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE sessions SET state = 'Completed', finished_at = ? WHERE id = ?")
            .bind(&now)
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn load_sessions(&self) -> Result<Vec<StoredSession>> {
        let rows: Vec<StoredSessionRow> = sqlx::query_as("SELECT * FROM sessions ORDER BY created_at DESC")
            .fetch_all(self.pool())
            .await?;
        
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn get_session(&self, id: &str) -> Result<Option<StoredSession>> {
        let row: Option<StoredSessionRow> = sqlx::query_as("SELECT * FROM sessions WHERE id = ?")
            .bind(id)
            .fetch_optional(self.pool())
            .await?;
        
        Ok(row.map(|r| r.into()))
    }

    pub async fn delete_session(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM messages WHERE session_id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn save_message(&self, message: &StoredMessage) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO messages (id, session_id, role, content, timestamp, tool_calls)
            VALUES (?, ?, ?, ?, ?, ?)
        "#)
        .bind(&message.id)
        .bind(&message.session_id)
        .bind(message.role.as_str())
        .bind(&message.content)
        .bind(message.timestamp.to_rfc3339())
        .bind(message.tool_calls.as_ref().map(|tc| serde_json::to_string(tc).unwrap_or_default()))
        .execute(self.pool())
        .await?;
        
        sqlx::query("UPDATE sessions SET message_count = message_count + 1 WHERE id = ?")
            .bind(&message.session_id)
            .execute(self.pool())
            .await?;
        
        Ok(())
    }

    pub async fn load_messages(&self, session_id: Option<&str>) -> Result<Vec<StoredMessage>> {
        let rows: Vec<StoredMessageRow> = if let Some(sid) = session_id {
            sqlx::query_as("SELECT * FROM messages WHERE session_id = ? ORDER BY timestamp ASC")
                .bind(sid)
                .fetch_all(self.pool())
                .await?
        } else {
            sqlx::query_as("SELECT * FROM messages ORDER BY timestamp ASC")
                .fetch_all(self.pool())
                .await?
        };
        
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn get_messages(&self, session_id: &str) -> Result<Vec<StoredMessage>> {
        self.load_messages(Some(session_id)).await
    }

    pub async fn save_process(&self, process: &StoredProcess) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO processes (id, command, args, cwd, status, exit_code, started_at, finished_at, session_id, stdout_size, stderr_size)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                exit_code = excluded.exit_code,
                finished_at = excluded.finished_at,
                stdout_size = excluded.stdout_size,
                stderr_size = excluded.stderr_size
        "#)
        .bind(&process.id)
        .bind(&process.command)
        .bind(serde_json::to_string(&process.args)?)
        .bind(&process.cwd)
        .bind(&process.status)
        .bind(process.exit_code)
        .bind(process.started_at.to_rfc3339())
        .bind(process.finished_at.map(|t| t.to_rfc3339()))
        .bind(&process.session_id)
        .bind(process.stdout_size as i64)
        .bind(process.stderr_size as i64)
        .execute(self.pool())
        .await?;
        
        Ok(())
    }

    pub async fn load_processes(&self) -> Result<Vec<StoredProcess>> {
        let rows: Vec<StoredProcessRow> = sqlx::query_as("SELECT * FROM processes ORDER BY started_at DESC")
            .fetch_all(self.pool())
            .await?;
        
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn save_activity(&self, activity: &StoredActivity) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO activities (id, session_id, activity_type, description, details, timestamp)
            VALUES (?, ?, ?, ?, ?, ?)
        "#)
        .bind(&activity.id)
        .bind(&activity.session_id)
        .bind(activity.activity_type.as_str())
        .bind(&activity.description)
        .bind(serde_json::to_string(&activity.details)?)
        .bind(activity.timestamp.to_rfc3339())
        .execute(self.pool())
        .await?;
        
        Ok(())
    }

    pub async fn load_activities(&self, session_id: Option<&str>) -> Result<Vec<StoredActivity>> {
        let rows: Vec<StoredActivityRow> = if let Some(sid) = session_id {
            sqlx::query_as("SELECT * FROM activities WHERE session_id = ? ORDER BY timestamp DESC")
                .bind(sid)
                .fetch_all(self.pool())
                .await?
        } else {
            sqlx::query_as("SELECT * FROM activities ORDER BY timestamp DESC")
                .fetch_all(self.pool())
                .await?
        };
        
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn save_task(&self, task: &StoredTask) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO tasks (id, content, status, priority, session_id, created_at, updated_at, completed_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                priority = excluded.priority,
                session_id = excluded.session_id,
                updated_at = excluded.updated_at,
                completed_at = excluded.completed_at
        "#)
        .bind(&task.id)
        .bind(&task.content)
        .bind(&task.status)
        .bind(&task.priority)
        .bind(&task.session_id)
        .bind(task.created_at.to_rfc3339())
        .bind(task.updated_at.to_rfc3339())
        .bind(task.completed_at.map(|t| t.to_rfc3339()))
        .execute(self.pool())
        .await?;
        
        Ok(())
    }

    pub async fn load_tasks(&self) -> Result<Vec<StoredTask>> {
        let rows: Vec<StoredTaskRow> = sqlx::query_as("SELECT * FROM tasks ORDER BY created_at DESC")
            .fetch_all(self.pool())
            .await?;
        
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn save_tool_result(&self, session_id: &str, result: &StoredToolResult) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO tool_results (tool_call_id, session_id, output, success)
            VALUES (?, ?, ?, ?)
        "#)
        .bind(&result.tool_call_id)
        .bind(session_id)
        .bind(&result.output)
        .bind(result.success as i32)
        .execute(self.pool())
        .await?;
        
        Ok(())
    }

    pub async fn get_tool_results(&self, session_id: &str) -> Result<Vec<StoredToolResult>> {
        let rows: Vec<StoredToolResultRow> = sqlx::query_as(
            "SELECT tool_call_id, output, success FROM tool_results WHERE session_id = ?"
        )
            .bind(session_id)
            .fetch_all(self.pool())
            .await?;
        
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn read_process_output(&self, process_id: &str, stream: &str) -> Result<String> {
        let path = self.base_path.join("process_outputs").join(format!("{}_{}.txt", process_id, stream));
        if path.exists() {
            Ok(tokio::fs::read_to_string(&path).await?)
        } else {
            Ok(String::new())
        }
    }

    pub async fn save_provider(&self, provider: &StoredProvider) -> Result<()> {
        sqlx::query(r#"
            INSERT INTO providers (id, name, base_url, api_key, model, max_tokens, temperature, top_p, top_k, extra_params, is_active, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                base_url = excluded.base_url,
                api_key = excluded.api_key,
                model = excluded.model,
                max_tokens = excluded.max_tokens,
                temperature = excluded.temperature,
                top_p = excluded.top_p,
                top_k = excluded.top_k,
                extra_params = excluded.extra_params,
                is_active = excluded.is_active,
                updated_at = excluded.updated_at
        "#)
        .bind(&provider.id)
        .bind(&provider.name)
        .bind(&provider.base_url)
        .bind(&provider.api_key)
        .bind(&provider.model)
        .bind(provider.max_tokens)
        .bind(provider.temperature)
        .bind(provider.top_p)
        .bind(provider.top_k)
        .bind(&provider.extra_params)
        .bind(provider.is_active as i32)
        .bind(provider.created_at.to_rfc3339())
        .bind(provider.updated_at.to_rfc3339())
        .execute(self.pool())
        .await?;
        
        Ok(())
    }

    pub async fn load_providers(&self) -> Result<Vec<StoredProvider>> {
        let rows: Vec<StoredProviderRow> = sqlx::query_as("SELECT * FROM providers ORDER BY created_at ASC")
            .fetch_all(self.pool())
            .await?;
        
        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    pub async fn get_active_provider(&self) -> Result<Option<StoredProvider>> {
        let row: Option<StoredProviderRow> = sqlx::query_as("SELECT * FROM providers WHERE is_active = 1 LIMIT 1")
            .fetch_optional(self.pool())
            .await?;
        
        Ok(row.map(|r| r.into()))
    }

    pub async fn set_active_provider(&self, id: &str) -> Result<()> {
        sqlx::query("UPDATE providers SET is_active = 0")
            .execute(self.pool())
            .await?;
        sqlx::query("UPDATE providers SET is_active = 1 WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }

    pub async fn delete_provider(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM providers WHERE id = ?")
            .bind(id)
            .execute(self.pool())
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_storage_init() {
        let dir = tempdir().unwrap();
        let storage = Storage::new(dir.path().to_path_buf());
        storage.init().await.unwrap();
    }

    #[tokio::test]
    async fn test_save_and_load_session() {
        let dir = tempdir().unwrap();
        let storage = Storage::new(dir.path().to_path_buf());
        storage.init().await.unwrap();
        
        let session = StoredSession {
            id: "test-session".to_string(),
            purpose: "Query".to_string(),
            state: "Idle".to_string(),
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
    async fn test_save_and_load_message() {
        let dir = tempdir().unwrap();
        let storage = Storage::new(dir.path().to_path_buf());
        storage.init().await.unwrap();
        
        let session = StoredSession {
            id: "s1".to_string(),
            purpose: "Query".to_string(),
            state: "Idle".to_string(),
            created_at: chrono::Utc::now(),
            started_at: None,
            finished_at: None,
            task_id: None,
            query_id: None,
            message_count: 0,
        };
        storage.save_session(&session).await.unwrap();
        
        let message = StoredMessage {
            id: "m1".to_string(),
            session_id: "s1".to_string(),
            role: MessageRole::User,
            content: "Hello".to_string(),
            timestamp: chrono::Utc::now(),
            tool_calls: None,
        };
        
        storage.save_message(&message).await.unwrap();
        
        let loaded = storage.get_messages("s1").await.unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].content, "Hello");
    }
}