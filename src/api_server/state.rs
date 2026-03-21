use crate::config::Config;
use crate::process::ProcessManager;
use crate::protocol::AgentEvent;
use crate::session::SessionManager;
use crate::storage::Storage;
use crate::task::TaskManager;
use crate::tool::{ToolExecutor, ToolRegistry};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct SharedConfig {
    inner: Arc<RwLock<Config>>,
}

impl SharedConfig {
    pub fn new(config: Config) -> Self {
        Self {
            inner: Arc::new(RwLock::new(config)),
        }
    }
    
    pub fn read(&self) -> tokio::sync::RwLockReadGuard<'_, Config> {
        futures::executor::block_on(self.inner.read())
    }
    
    pub fn write(&self) -> tokio::sync::RwLockWriteGuard<'_, Config> {
        futures::executor::block_on(self.inner.write())
    }
    
    pub async fn read_async(&self) -> tokio::sync::RwLockReadGuard<'_, Config> {
        self.inner.read().await
    }
    
    pub async fn write_async(&self) -> tokio::sync::RwLockWriteGuard<'_, Config> {
        self.inner.write().await
    }
}

pub struct ApiState {
    pub storage: Arc<Storage>,
    pub session_manager: Arc<SessionManager>,
    pub task_manager: Arc<TaskManager>,
    pub process_manager: Arc<ProcessManager>,
    pub event_tx: tokio::sync::broadcast::Sender<AgentEvent>,
    pub tool_registry: Arc<ToolRegistry>,
    pub tool_executor: Arc<ToolExecutor>,
    pub config: SharedConfig,
}
