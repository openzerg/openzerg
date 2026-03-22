use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::error::Result;
use super::types::{Session, SessionPurpose, SessionState, SessionSummary};

const MAX_CONCURRENT_SESSIONS: usize = 10;

pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    main_session_id: Arc<RwLock<Option<String>>>,
    dispatcher_session_id: Arc<RwLock<Option<String>>>,
    worker_session_id: Arc<RwLock<Option<String>>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            main_session_id: Arc::new(RwLock::new(None)),
            dispatcher_session_id: Arc::new(RwLock::new(None)),
            worker_session_id: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn init_main(&self) -> String {
        let mut sessions = self.sessions.write().await;
        let mut main_id = self.main_session_id.write().await;

        if let Some(id) = main_id.as_ref() {
            return id.clone();
        }

        let id = uuid::Uuid::new_v4().to_string();
        let session = Session::new(id.clone(), SessionPurpose::Main);
        sessions.insert(id.clone(), session);
        *main_id = Some(id.clone());
        
        tracing::info!("Main session created: {}", id);
        drop(sessions);
        drop(main_id);
        
        self.init_dispatcher().await;
        self.init_worker().await;
        
        id
    }
    
    pub async fn load_main_from_storage(&self, storage: &crate::storage::Storage) -> Option<String> {
        if let Ok(sessions) = storage.load_sessions().await {
            if let Some(main) = sessions.iter().find(|s| s.purpose == "Main") {
                let mut session_map = self.sessions.write().await;
                let mut main_id = self.main_session_id.write().await;
                
                // Create Session object from storage data
                let session = Session::new(main.id.clone(), SessionPurpose::Main);
                session_map.insert(main.id.clone(), session);
                *main_id = Some(main.id.clone());
                
                tracing::info!("Loaded Main session from storage: {}", main.id);
                
                // Initialize Dispatcher and Worker sessions
                drop(session_map);
                drop(main_id);
                self.init_dispatcher().await;
                self.init_worker().await;
                
                return Some(main.id.clone());
            }
        }
        None
    }
    
    async fn init_dispatcher(&self) -> String {
        let mut sessions = self.sessions.write().await;
        let mut dispatcher_id = self.dispatcher_session_id.write().await;

        if let Some(id) = dispatcher_id.as_ref() {
            return id.clone();
        }

        let id = uuid::Uuid::new_v4().to_string();
        let session = Session::new(id.clone(), SessionPurpose::Dispatcher);
        sessions.insert(id.clone(), session);
        *dispatcher_id = Some(id.clone());
        
        tracing::info!("Dispatcher session created: {}", id);
        id
    }
    
    async fn init_worker(&self) -> String {
        let mut sessions = self.sessions.write().await;
        let mut worker_id = self.worker_session_id.write().await;

        if let Some(id) = worker_id.as_ref() {
            return id.clone();
        }

        let id = uuid::Uuid::new_v4().to_string();
        let session = Session::new(id.clone(), SessionPurpose::Worker);
        sessions.insert(id.clone(), session);
        *worker_id = Some(id.clone());
        
        tracing::info!("Worker session created: {}", id);
        id
    }

    pub async fn spawn(&self, purpose: SessionPurpose) -> Result<String> {
        let sessions = self.sessions.read().await;
        let active_count = sessions.values()
            .filter(|s| s.state != SessionState::Completed && s.state != SessionState::Failed && s.state != SessionState::Cancelled)
            .count();
        drop(sessions);

        if active_count >= MAX_CONCURRENT_SESSIONS {
            return Err(crate::error::Error::Session(
                format!("Maximum concurrent sessions ({}) reached", MAX_CONCURRENT_SESSIONS)
            ));
        }

        let id = uuid::Uuid::new_v4().to_string();
        let session = Session::new(id.clone(), purpose);
        
        self.sessions.write().await.insert(id.clone(), session);
        
        tracing::info!("Session spawned: {} ({:?})", id, purpose);
        Ok(id)
    }

    pub async fn get(&self, id: &str) -> Option<Session> {
        self.sessions.read().await.get(id).cloned()
    }

    pub async fn update_state(&self, id: &str, state: SessionState) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(id) {
            session.state = state;
            
            if state == SessionState::Completed || state == SessionState::Failed || state == SessionState::Cancelled {
                session.finished_at = Some(chrono::Utc::now());
            }
        }
        
        Ok(())
    }

    pub async fn update_activity(&self, id: &str, activity: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(id) {
            session.current_activity = activity.to_string();
        }
        
        Ok(())
    }

    pub async fn bind_task(&self, id: &str, task_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(id) {
            session.task_id = Some(task_id.to_string());
        }
        
        Ok(())
    }

    pub async fn bind_query(&self, id: &str, query_id: &str) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        
        if let Some(session) = sessions.get_mut(id) {
            session.query_id = Some(query_id.to_string());
        }
        
        Ok(())
    }

    pub async fn complete(&self, id: &str) -> Result<()> {
        self.update_state(id, SessionState::Completed).await
    }

    pub async fn fail(&self, id: &str) -> Result<()> {
        self.update_state(id, SessionState::Failed).await
    }

    pub async fn cancel(&self, id: &str) -> Result<()> {
        self.update_state(id, SessionState::Cancelled).await
    }

    pub async fn list_active(&self) -> Vec<Session> {
        let sessions = self.sessions.read().await;
        sessions.values()
            .filter(|s| s.state != SessionState::Completed && s.state != SessionState::Failed && s.state != SessionState::Cancelled)
            .cloned()
            .collect()
    }

    pub async fn list_all(&self) -> Vec<Session> {
        self.sessions.read().await.values().cloned().collect()
    }

    pub async fn get_summaries(&self) -> Vec<SessionSummary> {
        self.sessions.read().await.values().map(|s| s.summary()).collect()
    }

    pub async fn get_main(&self) -> Option<Session> {
        let main_id = self.main_session_id.read().await;
        if let Some(id) = main_id.as_ref() {
            self.get(id).await
        } else {
            None
        }
    }

    pub async fn set_main_id(&self, id: &str) {
        let mut main_id = self.main_session_id.write().await;
        *main_id = Some(id.to_string());
    }

    pub async fn set_dispatcher_id(&self, id: &str) {
        let mut dispatcher_id = self.dispatcher_session_id.write().await;
        *dispatcher_id = Some(id.to_string());
    }

    pub async fn set_worker_id(&self, id: &str) {
        let mut worker_id = self.worker_session_id.write().await;
        *worker_id = Some(id.to_string());
    }

    pub async fn cleanup_finished(&self) -> usize {
        let mut sessions = self.sessions.write().await;
        let main_id = self.main_session_id.read().await.clone();
        let dispatcher_id = self.dispatcher_session_id.read().await.clone();
        let worker_id = self.worker_session_id.read().await.clone();
        
        let to_remove: Vec<String> = sessions
            .iter()
            .filter(|(id, s)| {
                let is_permanent = main_id.as_ref() == Some(*id)
                    || dispatcher_id.as_ref() == Some(*id)
                    || worker_id.as_ref() == Some(*id);
                
                (s.state == SessionState::Completed || s.state == SessionState::Failed || s.state == SessionState::Cancelled)
                    && !is_permanent
            })
            .map(|(id, _)| id.clone())
            .collect();

        let removed = to_remove.len();
        for id in to_remove {
            sessions.remove(&id);
            tracing::debug!("Session removed: {}", id);
        }

        removed
    }
    
    pub async fn get_dispatcher(&self) -> Option<Session> {
        let dispatcher_id = self.dispatcher_session_id.read().await;
        if let Some(id) = dispatcher_id.as_ref() {
            self.get(id).await
        } else {
            None
        }
    }
    
    pub async fn get_worker(&self) -> Option<Session> {
        let worker_id = self.worker_session_id.read().await;
        if let Some(id) = worker_id.as_ref() {
            self.get(id).await
        } else {
            None
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_manager_new() {
        let manager = SessionManager::new();
        let sessions = manager.list_all().await;
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_session_manager_default() {
        let manager = SessionManager::default();
        let sessions = manager.list_all().await;
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn test_init_main() {
        let manager = SessionManager::new();
        let id = manager.init_main().await;
        assert!(!id.is_empty());
        
        let session = manager.get(&id).await.unwrap();
        assert_eq!(session.purpose, SessionPurpose::Main);
        assert_eq!(session.state, SessionState::Idle);
    }

    #[tokio::test]
    async fn test_init_main_idempotent() {
        let manager = SessionManager::new();
        let id1 = manager.init_main().await;
        let id2 = manager.init_main().await;
        assert_eq!(id1, id2);
    }

    #[tokio::test]
    async fn test_spawn_session() {
        let manager = SessionManager::new();
        let id = manager.spawn(SessionPurpose::Query).await.unwrap();
        assert!(!id.is_empty());
        
        let session = manager.get(&id).await.unwrap();
        assert_eq!(session.purpose, SessionPurpose::Query);
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let manager = SessionManager::new();
        let session = manager.get("nonexistent").await;
        assert!(session.is_none());
    }

    #[tokio::test]
    async fn test_update_state() {
        let manager = SessionManager::new();
        let id = manager.spawn(SessionPurpose::Task).await.unwrap();
        
        manager.update_state(&id, SessionState::Generating).await.unwrap();
        let session = manager.get(&id).await.unwrap();
        assert_eq!(session.state, SessionState::Generating);
    }

    #[tokio::test]
    async fn test_complete_session() {
        let manager = SessionManager::new();
        let id = manager.spawn(SessionPurpose::Query).await.unwrap();
        
        manager.complete(&id).await.unwrap();
        let session = manager.get(&id).await.unwrap();
        assert_eq!(session.state, SessionState::Completed);
        assert!(session.finished_at.is_some());
    }

    #[tokio::test]
    async fn test_fail_session() {
        let manager = SessionManager::new();
        let id = manager.spawn(SessionPurpose::Task).await.unwrap();
        
        manager.fail(&id).await.unwrap();
        let session = manager.get(&id).await.unwrap();
        assert_eq!(session.state, SessionState::Failed);
    }

    #[tokio::test]
    async fn test_cancel_session() {
        let manager = SessionManager::new();
        let id = manager.spawn(SessionPurpose::Query).await.unwrap();
        
        manager.cancel(&id).await.unwrap();
        let session = manager.get(&id).await.unwrap();
        assert_eq!(session.state, SessionState::Cancelled);
    }

    #[tokio::test]
    async fn test_update_activity() {
        let manager = SessionManager::new();
        let id = manager.spawn(SessionPurpose::Main).await.unwrap();
        
        manager.update_activity(&id, "Working on task").await.unwrap();
        let session = manager.get(&id).await.unwrap();
        assert_eq!(session.current_activity, "Working on task");
    }

    #[tokio::test]
    async fn test_bind_task() {
        let manager = SessionManager::new();
        let id = manager.spawn(SessionPurpose::Task).await.unwrap();
        
        manager.bind_task(&id, "task-123").await.unwrap();
        let session = manager.get(&id).await.unwrap();
        assert_eq!(session.task_id, Some("task-123".to_string()));
    }

    #[tokio::test]
    async fn test_bind_query() {
        let manager = SessionManager::new();
        let id = manager.spawn(SessionPurpose::Query).await.unwrap();
        
        manager.bind_query(&id, "query-456").await.unwrap();
        let session = manager.get(&id).await.unwrap();
        assert_eq!(session.query_id, Some("query-456".to_string()));
    }

    #[tokio::test]
    async fn test_list_active() {
        let manager = SessionManager::new();
        let id1 = manager.spawn(SessionPurpose::Query).await.unwrap();
        let _id2 = manager.spawn(SessionPurpose::Task).await.unwrap();
        
        manager.complete(&id1).await.unwrap();
        
        let active = manager.list_active().await;
        assert_eq!(active.len(), 1);
    }

    #[tokio::test]
    async fn test_list_all() {
        let manager = SessionManager::new();
        manager.spawn(SessionPurpose::Query).await.unwrap();
        manager.spawn(SessionPurpose::Task).await.unwrap();
        
        let all = manager.list_all().await;
        assert_eq!(all.len(), 2);
    }

    #[tokio::test]
    async fn test_get_summaries() {
        let manager = SessionManager::new();
        manager.spawn(SessionPurpose::Query).await.unwrap();
        
        let summaries = manager.get_summaries().await;
        assert_eq!(summaries.len(), 1);
    }

    #[tokio::test]
    async fn test_get_main() {
        let manager = SessionManager::new();
        let id = manager.init_main().await;
        
        let session = manager.get_main().await.unwrap();
        assert_eq!(session.id, id);
    }

    #[tokio::test]
    async fn test_cleanup_finished() {
        let manager = SessionManager::new();
        let id1 = manager.spawn(SessionPurpose::Query).await.unwrap();
        let id2 = manager.spawn(SessionPurpose::Task).await.unwrap();
        
        manager.complete(&id1).await.unwrap();
        manager.complete(&id2).await.unwrap();
        
        let removed = manager.cleanup_finished().await;
        assert_eq!(removed, 2);
        
        let all = manager.list_all().await;
        assert!(all.is_empty());
    }

    #[tokio::test]
    async fn test_max_concurrent_sessions() {
        let manager = SessionManager::new();
        
        for _ in 0..10 {
            manager.spawn(SessionPurpose::Query).await.unwrap();
        }
        
        let result = manager.spawn(SessionPurpose::Query).await;
        assert!(result.is_err());
    }
}