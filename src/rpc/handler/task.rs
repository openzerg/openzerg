use crate::rpc::registry::RpcRegistry;
use crate::rpc::protocol::RpcError;
use crate::agent::AgentCore;
use std::sync::Arc;
use serde::Deserialize;

pub async fn register(registry: &RpcRegistry, core: Arc<AgentCore>) {
    let core_clone = core.clone();
    registry.register("task.list", move |_params| {
        let core = core_clone.clone();
        async move {
            let tasks = core.storage.load_tasks().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            Ok(serde_json::json!({"tasks": tasks}))
        }
    }).await;

    let core_clone = core.clone();
    registry.register("task.get", move |params| {
        let core = core_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { id: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'id'"))?;
            
            let tasks = core.storage.load_tasks().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            let task = tasks.into_iter().find(|t| t.id == p.id)
                .ok_or_else(|| RpcError::task_not_found(&p.id))?;
            
            Ok(serde_json::to_value(task).unwrap())
        }
    }).await;
}