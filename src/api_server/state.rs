use crate::process::ProcessManager;
use crate::protocol::AgentEvent;
use crate::session::SessionManager;
use crate::storage::Storage;
use crate::task::TaskManager;
use crate::tool::{ToolExecutor, ToolRegistry};
use std::sync::Arc;

pub struct ApiState {
    pub storage: Arc<Storage>,
    pub session_manager: Arc<SessionManager>,
    pub task_manager: Arc<TaskManager>,
    pub process_manager: Arc<ProcessManager>,
    pub event_tx: tokio::sync::broadcast::Sender<AgentEvent>,
    pub tool_registry: Arc<ToolRegistry>,
    pub tool_executor: Arc<ToolExecutor>,
}
