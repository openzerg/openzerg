use crate::protocol::Priority;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: Priority,
    pub session_id: Option<String>,
    pub parent_task: Option<String>,
    pub subtasks: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub deadline: Option<DateTime<Utc>>,
    pub result: Option<TaskResult>,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Assigned,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub success: bool,
    pub summary: String,
    pub details: Option<serde_json::Value>,
}

impl Task {
    pub fn new(id: String, title: String, description: String, priority: Priority) -> Self {
        Self {
            id,
            title,
            description,
            status: TaskStatus::Pending,
            priority,
            session_id: None,
            parent_task: None,
            subtasks: Vec::new(),
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            deadline: None,
            result: None,
            context: None,
        }
    }

    pub fn summary(&self) -> TaskSummary {
        TaskSummary {
            id: self.id.clone(),
            title: self.title.clone(),
            status: format!("{:?}", self.status),
            priority: format!("{:?}", self.priority),
            session_id: self.session_id.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSummary {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    pub session_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_new() {
        let task = Task::new(
            "task-1".to_string(),
            "Test Task".to_string(),
            "Description".to_string(),
            Priority::Medium,
        );
        assert_eq!(task.id, "task-1");
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.priority, Priority::Medium);
        assert!(task.session_id.is_none());
        assert!(task.parent_task.is_none());
        assert!(task.subtasks.is_empty());
    }

    #[test]
    fn test_task_summary() {
        let task = Task::new(
            "task-1".to_string(),
            "Test Task".to_string(),
            "Description".to_string(),
            Priority::High,
        );
        let summary = task.summary();
        assert_eq!(summary.id, "task-1");
        assert_eq!(summary.title, "Test Task");
        assert_eq!(summary.status, "Pending");
        assert_eq!(summary.priority, "High");
    }

    #[test]
    fn test_task_status_variants() {
        assert_eq!(TaskStatus::Pending, TaskStatus::Pending);
        assert_ne!(TaskStatus::Pending, TaskStatus::InProgress);
        assert_ne!(TaskStatus::Completed, TaskStatus::Failed);
    }

    #[test]
    fn test_task_status_serialization() {
        let status = TaskStatus::InProgress;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"InProgress\"");
    }

    #[test]
    fn test_task_result_serialization() {
        let result = TaskResult {
            success: true,
            summary: "Task completed".to_string(),
            details: Some(serde_json::json!({"key": "value"})),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("completed"));
    }

    #[test]
    fn test_task_serialization() {
        let task = Task::new(
            "t1".to_string(),
            "Title".to_string(),
            "Desc".to_string(),
            Priority::Low,
        );
        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("t1"));
        assert!(json.contains("Title"));
    }

    #[test]
    fn test_task_summary_serialization() {
        let summary = TaskSummary {
            id: "t1".to_string(),
            title: "Test".to_string(),
            status: "Pending".to_string(),
            priority: "High".to_string(),
            session_id: Some("s1".to_string()),
        };
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("t1"));
        assert!(json.contains("s1"));
    }
}
