use crate::rpc::registry::RpcRegistry;
use crate::rpc::protocol::RpcError;
use crate::agent::AgentCore;
use std::sync::Arc;

pub async fn register(registry: &RpcRegistry, core: Arc<AgentCore>) {
    let core_clone = core.clone();
    registry.register("activity.list", move |_params| {
        let core = core_clone.clone();
        async move {
            let activities = core.storage.load_activities(None).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            Ok(serde_json::json!({"activities": activities}))
        }
    }).await;
}