use crate::rpc::registry::RpcRegistry;
use crate::rpc::protocol::RpcError;
use crate::agent::AgentCore;
use std::sync::Arc;
use serde::Deserialize;

pub async fn register(registry: &RpcRegistry, core: Arc<AgentCore>) {
    let core_clone = core.clone();
    registry.register("process.list", move |_params| {
        let core = core_clone.clone();
        async move {
            let processes = core.storage.load_processes().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            Ok(serde_json::json!({"processes": processes}))
        }
    }).await;

    let core_clone = core.clone();
    registry.register("process.get", move |params| {
        let core = core_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { id: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'id'"))?;
            
            let processes = core.storage.load_processes().await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            let process = processes.into_iter().find(|p| p.id == p.id)
                .ok_or_else(|| RpcError::process_not_found(&p.id))?;
            
            Ok(serde_json::to_value(process).unwrap())
        }
    }).await;

    let core_clone = core.clone();
    registry.register("process.output", move |params| {
        let core = core_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { 
                id: String, 
                #[serde(default)] stream: Option<String>,
                #[serde(default)] offset: Option<usize>,
                #[serde(default)] limit: Option<usize>,
            }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'id'"))?;
            
            let stream = p.stream.as_deref().unwrap_or("stdout");
            let content = core.storage.read_process_output(&p.id, stream).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let offset = p.offset.unwrap_or(0);
            let limit = p.limit.unwrap_or(10000);
            let end = (offset + limit).min(content.len());
            let slice = &content[offset..end];
            
            Ok(serde_json::json!({
                "process_id": p.id,
                "stream": stream,
                "content": slice,
                "total_size": content.len(),
            }))
        }
    }).await;
}