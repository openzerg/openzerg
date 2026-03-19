use crate::rpc::registry::RpcRegistry;
use crate::rpc::protocol::RpcError;
use crate::agent::AgentCore;
use std::sync::Arc;
use serde::Deserialize;

pub async fn register(registry: &RpcRegistry, core: Arc<AgentCore>) {
    let core_clone = core.clone();
    registry.register("builtin.tools.list", move |_params| {
        let core = core_clone.clone();
        async move {
            let definitions = core.tool_registry.tool_definitions().await;
            Ok(serde_json::json!({"tools": definitions}))
        }
    }).await;

    let core_clone = core.clone();
    registry.register("builtin.tools.execute", move |params| {
        let core = core_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { 
                name: String, 
                args: serde_json::Value,
                #[serde(default)]
                session_id: Option<String>,
            }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Invalid params"))?;
            
            let session_id = p.session_id.unwrap_or_else(|| "default".to_string());
            let message_id = uuid::Uuid::new_v4().to_string();
            
            let result = core.tool_registry.execute(&p.name, p.args, crate::tool::ToolContext {
                session_id: session_id.clone(),
                message_id: message_id.clone(),
                workspace: std::path::PathBuf::from("/workspace"),
                openzerg_dir: std::path::PathBuf::from("/workspace/.openzerg"),
                abort: tokio_util::sync::CancellationToken::new(),
                file_manager: Arc::new(crate::file::FileManager::new(std::path::PathBuf::from("/workspace"))),
                process_manager: core.process_manager.clone(),
            }).await.map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            Ok(serde_json::json!({
                "title": result.title,
                "output": result.output,
                "metadata": result.metadata,
                "attachments": result.attachments,
                "truncated": result.truncated,
            }))
        }
    }).await;
}