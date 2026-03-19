pub mod session;
pub mod process;
pub mod task;
pub mod activity;
pub mod message;
pub mod tool;

use crate::rpc::registry::RpcRegistry;
use crate::agent::AgentCore;
use std::sync::Arc;

pub async fn register_all_methods(registry: &RpcRegistry, core: Arc<AgentCore>) {
    session::register(registry, core.clone()).await;
    process::register(registry, core.clone()).await;
    task::register(registry, core.clone()).await;
    activity::register(registry, core.clone()).await;
    message::register(registry, core.clone()).await;
    tool::register(registry, core).await;
}