use openzerg::{
    session::{SessionManager, SessionPurpose, SessionState},
    task::{TaskManager, Task, TaskStatus, TaskResult},
    protocol::Priority,
};
use std::sync::Arc;

#[tokio::test]
async fn test_session_manager_flow() {
    let manager = Arc::new(SessionManager::new());
    
    // Init main session
    let main_id = manager.init_main().await;
    assert!(!main_id.is_empty());
    
    // Spawn a query session
    let query_id = manager.spawn(SessionPurpose::Query).await.unwrap();
    assert!(!query_id.is_empty());
    
    // Spawn a task session
    let task_id = manager.spawn(SessionPurpose::Task).await.unwrap();
    
    // Update state
    manager.update_state(&query_id, SessionState::Generating).await.unwrap();
    let session = manager.get(&query_id).await.unwrap();
    assert_eq!(session.state, SessionState::Generating);
    
    // Complete session
    manager.complete(&task_id).await.unwrap();
    let session = manager.get(&task_id).await.unwrap();
    assert_eq!(session.state, SessionState::Completed);
    
    // List active
    let active = manager.list_active().await;
    assert_eq!(active.len(), 2); // main and query
    
    // List all
    let all = manager.list_all().await;
    assert_eq!(all.len(), 3);
    
    // Cleanup finished
    let removed = manager.cleanup_finished().await;
    assert_eq!(removed, 1); // task session
}

#[tokio::test]
async fn test_task_manager_flow() {
    let manager = Arc::new(TaskManager::new());
    
    // Create tasks
    let t1 = Task::new("t1".to_string(), "Task 1".to_string(), "Description 1".to_string(), Priority::High);
    let t2 = Task::new("t2".to_string(), "Task 2".to_string(), "Description 2".to_string(), Priority::Low);
    
    manager.add(t1).await.unwrap();
    manager.add(t2).await.unwrap();
    
    // Assign task
    manager.assign("t1", "session-1").await.unwrap();
    let task = manager.get("t1").await.unwrap();
    assert_eq!(task.status, TaskStatus::Assigned);
    assert_eq!(task.session_id, Some("session-1".to_string()));
    
    // Start task
    manager.start("t1").await.unwrap();
    let task = manager.get("t1").await.unwrap();
    assert_eq!(task.status, TaskStatus::InProgress);
    
    // Complete task
    let result = TaskResult {
        success: true,
        summary: "Done".to_string(),
        details: Some(serde_json::json!({"output": "success"})),
    };
    manager.complete("t1", result).await.unwrap();
    let task = manager.get("t1").await.unwrap();
    assert_eq!(task.status, TaskStatus::Completed);
    
    // Fail task
    manager.fail("t2", "Something went wrong").await.unwrap();
    let task = manager.get("t2").await.unwrap();
    assert_eq!(task.status, TaskStatus::Failed);
    
    // Get pending (none)
    let pending = manager.get_pending().await;
    assert!(pending.is_empty());
    
    // Get active (none)
    let active = manager.get_active().await;
    assert!(active.is_empty());
    
    // Get session tasks
    let session_tasks = manager.get_session_tasks("session-1").await;
    assert_eq!(session_tasks.len(), 1);
}

#[tokio::test]
async fn test_session_task_integration() {
    let session_manager = Arc::new(SessionManager::new());
    let task_manager = Arc::new(TaskManager::new());
    
    // Create a task
    let task = Task::new("task-1".to_string(), "Build feature".to_string(), "Implement X".to_string(), Priority::Medium);
    task_manager.add(task).await.unwrap();
    
    // Spawn a session for the task
    let session_id = session_manager.spawn(SessionPurpose::Task).await.unwrap();
    
    // Bind task to session
    session_manager.bind_task(&session_id, "task-1").await.unwrap();
    
    // Assign task to session
    task_manager.assign("task-1", &session_id).await.unwrap();
    
    // Start working
    task_manager.start("task-1").await.unwrap();
    session_manager.update_state(&session_id, SessionState::Generating).await.unwrap();
    
    // Complete
    let result = TaskResult {
        success: true,
        summary: "Feature built".to_string(),
        details: None,
    };
    task_manager.complete("task-1", result).await.unwrap();
    session_manager.complete(&session_id).await.unwrap();
    
    // Verify
    let session = session_manager.get(&session_id).await.unwrap();
    assert_eq!(session.state, SessionState::Completed);
    assert_eq!(session.task_id, Some("task-1".to_string()));
    
    let task = task_manager.get("task-1").await.unwrap();
    assert_eq!(task.status, TaskStatus::Completed);
    assert_eq!(task.session_id, Some(session_id));
}

#[tokio::test]
async fn test_max_sessions_limit() {
    let manager = Arc::new(SessionManager::new());
    
    // Spawn 10 sessions (max)
    for _ in 0..10 {
        manager.spawn(SessionPurpose::Query).await.unwrap();
    }
    
    // 11th should fail
    let result = manager.spawn(SessionPurpose::Query).await;
    assert!(result.is_err());
    
    // Complete one
    let all = manager.list_all().await;
    manager.complete(&all[0].id).await.unwrap();
    
    // Now should succeed
    let result = manager.spawn(SessionPurpose::Query).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_task_priorities() {
    let manager = Arc::new(TaskManager::new());
    
    let low = Task::new("low".to_string(), "Low".to_string(), "".to_string(), Priority::Low);
    let medium = Task::new("medium".to_string(), "Medium".to_string(), "".to_string(), Priority::Medium);
    let high = Task::new("high".to_string(), "High".to_string(), "".to_string(), Priority::High);
    let urgent = Task::new("urgent".to_string(), "Urgent".to_string(), "".to_string(), Priority::Urgent);
    
    manager.add(low).await.unwrap();
    manager.add(medium).await.unwrap();
    manager.add(high).await.unwrap();
    manager.add(urgent).await.unwrap();
    
    let all = manager.list(None).await;
    assert_eq!(all.len(), 4);
}

#[tokio::test]
async fn test_task_cancel() {
    let manager = Arc::new(TaskManager::new());
    
    let task = Task::new("t1".to_string(), "Task".to_string(), "".to_string(), Priority::Medium);
    manager.add(task).await.unwrap();
    
    manager.start("t1").await.unwrap();
    manager.cancel("t1").await.unwrap();
    
    let task = manager.get("t1").await.unwrap();
    assert_eq!(task.status, TaskStatus::Cancelled);
    assert!(task.completed_at.is_some());
}

#[tokio::test]
async fn test_session_summaries() {
    let manager = Arc::new(SessionManager::new());
    
    manager.spawn(SessionPurpose::Query).await.unwrap();
    manager.spawn(SessionPurpose::Task).await.unwrap();
    
    let summaries = manager.get_summaries().await;
    assert_eq!(summaries.len(), 2);
    
    assert!(summaries.iter().any(|s| s.purpose == "Query"));
    assert!(summaries.iter().any(|s| s.purpose == "Task"));
}

#[tokio::test]
async fn test_task_summaries() {
    let manager = Arc::new(TaskManager::new());
    
    let t1 = Task::new("t1".to_string(), "Task 1".to_string(), "".to_string(), Priority::High);
    let t2 = Task::new("t2".to_string(), "Task 2".to_string(), "".to_string(), Priority::Low);
    
    manager.add(t1).await.unwrap();
    manager.add(t2).await.unwrap();
    
    manager.start("t1").await.unwrap();
    
    let summaries = manager.list_summaries(None).await;
    assert_eq!(summaries.len(), 2);
    
    let in_progress = manager.list_summaries(Some(TaskStatus::InProgress)).await;
    assert_eq!(in_progress.len(), 1);
    assert_eq!(in_progress[0].id, "t1");
}