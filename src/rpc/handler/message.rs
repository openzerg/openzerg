use crate::rpc::registry::RpcRegistry;
use crate::rpc::protocol::RpcError;
use crate::agent::AgentCore;
use std::sync::Arc;
use serde::Deserialize;

pub async fn register(registry: &RpcRegistry, core: Arc<AgentCore>) {
    let core_clone = core.clone();
    registry.register("message.send", move |params| {
        let core = core_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { content: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'content'"))?;
            
            let event = crate::protocol::AgentEvent::Message {
                content: p.content,
                from: "user".to_string(),
            };
            
            let _ = core.event_tx.send(event);
            
            Ok(serde_json::json!({"sent": true}))
        }
    }).await;

    let core_clone = core.clone();
    registry.register("message.remind", move |params| {
        let core = core_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { message: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'message'"))?;
            
            let id = uuid::Uuid::new_v4().to_string();
            let event = crate::protocol::AgentEvent::Remind {
                id,
                message: p.message,
            };
            
            let _ = core.event_tx.send(event);
            
            Ok(serde_json::json!({"sent": true}))
        }
    }).await;
}