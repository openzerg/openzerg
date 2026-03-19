use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub purpose: SessionPurpose,
    pub state: SessionState,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub task_id: Option<String>,
    pub query_id: Option<String>,
    pub current_activity: String,
    pub message_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionPurpose {
    Main,
    Query,
    Task,
    Remind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Idle,
    Generating,
    WaitingTool,
    Thinking,
    Completed,
    Failed,
    Cancelled,
}

impl Session {
    pub fn new(id: String, purpose: SessionPurpose) -> Self {
        Self {
            id,
            purpose,
            state: SessionState::Idle,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
            task_id: None,
            query_id: None,
            current_activity: "Created".to_string(),
            message_count: 0,
        }
    }

    pub fn summary(&self) -> SessionSummary {
        SessionSummary {
            id: self.id.clone(),
            purpose: format!("{:?}", self.purpose),
            state: format!("{:?}", self.state),
            current_activity: self.current_activity.clone(),
            message_count: self.message_count,
        }
    }

    pub fn duration(&self) -> Option<std::time::Duration> {
        self.finished_at.map(|end| {
            (end - self.created_at)
                .to_std()
                .unwrap_or(std::time::Duration::ZERO)
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub purpose: String,
    pub state: String,
    pub current_activity: String,
    pub message_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let session = Session::new("test-id".to_string(), SessionPurpose::Main);
        assert_eq!(session.id, "test-id");
        assert_eq!(session.purpose, SessionPurpose::Main);
        assert_eq!(session.state, SessionState::Idle);
        assert_eq!(session.current_activity, "Created");
        assert_eq!(session.message_count, 0);
    }

    #[test]
    fn test_session_summary() {
        let session = Session::new("test-id".to_string(), SessionPurpose::Query);
        let summary = session.summary();
        assert_eq!(summary.id, "test-id");
        assert_eq!(summary.purpose, "Query");
        assert_eq!(summary.state, "Idle");
    }

    #[test]
    fn test_session_purpose_variants() {
        assert_eq!(SessionPurpose::Main, SessionPurpose::Main);
        assert_ne!(SessionPurpose::Main, SessionPurpose::Query);
        assert_ne!(SessionPurpose::Task, SessionPurpose::Remind);
    }

    #[test]
    fn test_session_state_variants() {
        assert_eq!(SessionState::Idle, SessionState::Idle);
        assert_ne!(SessionState::Idle, SessionState::Generating);
        assert_ne!(SessionState::Thinking, SessionState::Completed);
    }

    #[test]
    fn test_session_duration_no_finish() {
        let session = Session::new("test-id".to_string(), SessionPurpose::Main);
        assert!(session.duration().is_none());
    }

    #[test]
    fn test_session_serialization() {
        let session = Session::new("test-id".to_string(), SessionPurpose::Task);
        let json = serde_json::to_string(&session).unwrap();
        assert!(json.contains("test-id"));
        assert!(json.contains("Task"));
    }

    #[test]
    fn test_session_deserialization() {
        let json = r#"{"id":"x","purpose":"Main","state":"Idle","created_at":"2024-01-01T00:00:00Z","started_at":null,"finished_at":null,"task_id":null,"query_id":null,"current_activity":"test","message_count":0}"#;
        let session: Session = serde_json::from_str(json).unwrap();
        assert_eq!(session.id, "x");
        assert_eq!(session.purpose, SessionPurpose::Main);
    }

    #[test]
    fn test_session_summary_serialization() {
        let summary = SessionSummary {
            id: "test".to_string(),
            purpose: "Main".to_string(),
            state: "Idle".to_string(),
            current_activity: "test".to_string(),
            message_count: 5,
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("5"));
    }
}
