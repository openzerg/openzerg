use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::error::Result;
use crate::protocol::Priority;
use super::types::{Task, TaskStatus, TaskResult, TaskSummary};

pub struct TaskManager {
    tasks: Arc<RwLock<HashMap<String, Task>>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add(&self, task: Task) -> Result<()> {
        let id = task.id.clone();
        self.tasks.write().await.insert(id, task);
        Ok(())
    }

    pub async fn get(&self, id: &str) -> Option<Task> {
        self.tasks.read().await.get(id).cloned()
    }

    pub async fn assign(&self, id: &str, session_id: &str) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        
        if let Some(task) = tasks.get_mut(id) {
            task.session_id = Some(session_id.to_string());
            task.status = TaskStatus::Assigned;
        }
        
        Ok(())
    }

    pub async fn start(&self, id: &str) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        
        if let Some(task) = tasks.get_mut(id) {
            task.status = TaskStatus::InProgress;
            task.started_at = Some(chrono::Utc::now());
        }
        
        Ok(())
    }

    pub async fn complete(&self, id: &str, result: TaskResult) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        
        if let Some(task) = tasks.get_mut(id) {
            task.status = TaskStatus::Completed;
            task.completed_at = Some(chrono::Utc::now());
            task.result = Some(result);
        }
        
        Ok(())
    }

    pub async fn fail(&self, id: &str, error: &str) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        
        if let Some(task) = tasks.get_mut(id) {
            task.status = TaskStatus::Failed;
            task.completed_at = Some(chrono::Utc::now());
            task.result = Some(TaskResult {
                success: false,
                summary: error.to_string(),
                details: None,
            });
        }
        
        Ok(())
    }

    pub async fn cancel(&self, id: &str) -> Result<()> {
        let mut tasks = self.tasks.write().await;
        
        if let Some(task) = tasks.get_mut(id) {
            task.status = TaskStatus::Cancelled;
            task.completed_at = Some(chrono::Utc::now());
        }
        
        Ok(())
    }

    pub async fn list(&self, status: Option<TaskStatus>) -> Vec<Task> {
        let tasks = self.tasks.read().await;
        
        match status {
            Some(s) => tasks.values().filter(|t| t.status == s).cloned().collect(),
            None => tasks.values().cloned().collect(),
        }
    }

    pub async fn list_summaries(&self, status: Option<TaskStatus>) -> Vec<TaskSummary> {
        let tasks = self.tasks.read().await;
        
        let filtered: Vec<_> = match status {
            Some(s) => tasks.values().filter(|t| t.status == s).collect(),
            None => tasks.values().collect(),
        };
        
        filtered.iter().map(|t| t.summary()).collect()
    }

    pub async fn get_session_tasks(&self, session_id: &str) -> Vec<Task> {
        let tasks = self.tasks.read().await;
        tasks.values()
            .filter(|t| t.session_id.as_deref() == Some(session_id))
            .cloned()
            .collect()
    }

    pub async fn get_pending(&self) -> Vec<Task> {
        self.list(Some(TaskStatus::Pending)).await
    }

    pub async fn get_active(&self) -> Vec<Task> {
        let tasks = self.tasks.read().await;
        tasks.values()
            .filter(|t| matches!(t.status, TaskStatus::Assigned | TaskStatus::InProgress))
            .cloned()
            .collect()
    }

    pub async fn remove(&self, id: &str) -> Result<()> {
        self.tasks.write().await.remove(id);
        Ok(())
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_manager_new() {
        let manager = TaskManager::new();
        let tasks = manager.list(None).await;
        assert!(tasks.is_empty());
    }

    #[tokio::test]
    async fn test_task_manager_default() {
        let manager = TaskManager::default();
        let tasks = manager.list(None).await;
        assert!(tasks.is_empty());
    }

    #[tokio::test]
    async fn test_add_task() {
        let manager = TaskManager::new();
        let task = Task::new("t1".to_string(), "Test".to_string(), "Desc".to_string(), Priority::Medium);
        
        manager.add(task).await.unwrap();
        let retrieved = manager.get("t1").await.unwrap();
        assert_eq!(retrieved.title, "Test");
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let manager = TaskManager::new();
        let task = manager.get("nonexistent").await;
        assert!(task.is_none());
    }

    #[tokio::test]
    async fn test_assign_task() {
        let manager = TaskManager::new();
        let task = Task::new("t1".to_string(), "Test".to_string(), "Desc".to_string(), Priority::High);
        manager.add(task).await.unwrap();
        
        manager.assign("t1", "session-1").await.unwrap();
        let retrieved = manager.get("t1").await.unwrap();
        assert_eq!(retrieved.status, TaskStatus::Assigned);
        assert_eq!(retrieved.session_id, Some("session-1".to_string()));
    }

    #[tokio::test]
    async fn test_start_task() {
        let manager = TaskManager::new();
        let task = Task::new("t1".to_string(), "Test".to_string(), "Desc".to_string(), Priority::Low);
        manager.add(task).await.unwrap();
        
        manager.start("t1").await.unwrap();
        let retrieved = manager.get("t1").await.unwrap();
        assert_eq!(retrieved.status, TaskStatus::InProgress);
        assert!(retrieved.started_at.is_some());
    }

    #[tokio::test]
    async fn test_complete_task() {
        let manager = TaskManager::new();
        let task = Task::new("t1".to_string(), "Test".to_string(), "Desc".to_string(), Priority::Medium);
        manager.add(task).await.unwrap();
        
        let result = TaskResult {
            success: true,
            summary: "Done".to_string(),
            details: None,
        };
        manager.complete("t1", result).await.unwrap();
        
        let retrieved = manager.get("t1").await.unwrap();
        assert_eq!(retrieved.status, TaskStatus::Completed);
        assert!(retrieved.completed_at.is_some());
        assert!(retrieved.result.is_some());
    }

    #[tokio::test]
    async fn test_fail_task() {
        let manager = TaskManager::new();
        let task = Task::new("t1".to_string(), "Test".to_string(), "Desc".to_string(), Priority::Urgent);
        manager.add(task).await.unwrap();
        
        manager.fail("t1", "Something went wrong").await.unwrap();
        let retrieved = manager.get("t1").await.unwrap();
        assert_eq!(retrieved.status, TaskStatus::Failed);
        assert!(retrieved.result.unwrap().summary.contains("wrong"));
    }

    #[tokio::test]
    async fn test_cancel_task() {
        let manager = TaskManager::new();
        let task = Task::new("t1".to_string(), "Test".to_string(), "Desc".to_string(), Priority::Medium);
        manager.add(task).await.unwrap();
        
        manager.cancel("t1").await.unwrap();
        let retrieved = manager.get("t1").await.unwrap();
        assert_eq!(retrieved.status, TaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_list_by_status() {
        let manager = TaskManager::new();
        let t1 = Task::new("t1".to_string(), "A".to_string(), "".to_string(), Priority::Medium);
        let t2 = Task::new("t2".to_string(), "B".to_string(), "".to_string(), Priority::Medium);
        manager.add(t1).await.unwrap();
        manager.add(t2).await.unwrap();
        manager.complete("t1", TaskResult { success: true, summary: "".to_string(), details: None }).await.unwrap();
        
        let pending = manager.list(Some(TaskStatus::Pending)).await;
        let completed = manager.list(Some(TaskStatus::Completed)).await;
        
        assert_eq!(pending.len(), 1);
        assert_eq!(completed.len(), 1);
    }

    #[tokio::test]
    async fn test_list_summaries() {
        let manager = TaskManager::new();
        let task = Task::new("t1".to_string(), "Test Task".to_string(), "Desc".to_string(), Priority::High);
        manager.add(task).await.unwrap();
        
        let summaries = manager.list_summaries(None).await;
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].title, "Test Task");
    }

    #[tokio::test]
    async fn test_get_session_tasks() {
        let manager = TaskManager::new();
        let t1 = Task::new("t1".to_string(), "A".to_string(), "".to_string(), Priority::Medium);
        let t2 = Task::new("t2".to_string(), "B".to_string(), "".to_string(), Priority::Medium);
        manager.add(t1).await.unwrap();
        manager.add(t2).await.unwrap();
        manager.assign("t1", "session-1").await.unwrap();
        
        let tasks = manager.get_session_tasks("session-1").await;
        assert_eq!(tasks.len(), 1);
    }

    #[tokio::test]
    async fn test_get_pending() {
        let manager = TaskManager::new();
        let t1 = Task::new("t1".to_string(), "A".to_string(), "".to_string(), Priority::Medium);
        manager.add(t1).await.unwrap();
        
        let pending = manager.get_pending().await;
        assert_eq!(pending.len(), 1);
    }

    #[tokio::test]
    async fn test_get_active() {
        let manager = TaskManager::new();
        let t1 = Task::new("t1".to_string(), "A".to_string(), "".to_string(), Priority::Medium);
        let t2 = Task::new("t2".to_string(), "B".to_string(), "".to_string(), Priority::Medium);
        manager.add(t1).await.unwrap();
        manager.add(t2).await.unwrap();
        manager.start("t1").await.unwrap();
        
        let active = manager.get_active().await;
        assert_eq!(active.len(), 1);
    }

    #[tokio::test]
    async fn test_remove_task() {
        let manager = TaskManager::new();
        let task = Task::new("t1".to_string(), "Test".to_string(), "".to_string(), Priority::Medium);
        manager.add(task).await.unwrap();
        
        manager.remove("t1").await.unwrap();
        let retrieved = manager.get("t1").await;
        assert!(retrieved.is_none());
    }
}