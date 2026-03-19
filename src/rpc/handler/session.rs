use crate::rpc::registry::RpcRegistry;
use crate::rpc::protocol::RpcError;
use crate::agent::AgentCore;
use std::sync::Arc;
use serde::Deserialize;

pub async fn register(registry: &RpcRegistry, core: Arc<AgentCore>) {
    let core_clone = core.clone();
    registry.register("session.list", move |_params| {
        let core = core_clone.clone();
        async move {
            let sessions = core.session_manager.list_all().await;
            Ok(serde_json::to_value(sessions).unwrap())
        }
    }).await;

    let core_clone = core.clone();
    registry.register("session.get", move |params| {
        let core = core_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { id: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'id'"))?;
            
            let session = core.session_manager.get(&p.id).await
                .ok_or_else(|| RpcError::session_not_found(&p.id))?;
            
            Ok(serde_json::to_value(session).unwrap())
        }
    }).await;

    let core_clone = core.clone();
    registry.register("session.messages", move |params| {
        let core = core_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { id: String, #[serde(default)] offset: Option<usize>, #[serde(default)] limit: Option<usize> }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'id'"))?;
            
            let messages = core.storage.load_messages(Some(&p.id)).await
                .map_err(|e| RpcError::internal_error(e.to_string()))?;
            
            let offset = p.offset.unwrap_or(0);
            let limit = p.limit.unwrap_or(100);
            let messages: Vec<_> = messages.into_iter().skip(offset).take(limit).collect();
            
            Ok(serde_json::json!({"messages": messages, "total": messages.len()}))
        }
    }).await;

    let core_clone = core.clone();
    registry.register("session.chat", move |params| {
        let core = core_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { id: String, content: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing params"))?;
            
            let event = crate::protocol::AgentEvent::Message {
                content: p.content,
                from: "user".to_string(),
            };
            
            let _ = core.event_tx.send(event);
            
            Ok(serde_json::json!({"session_id": p.id}))
        }
    }).await;

    let core_clone = core.clone();
    registry.register("session.interrupt", move |params| {
        let core = core_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { id: String, message: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing params"))?;
            
            let event = crate::protocol::AgentEvent::Interrupt {
                message: p.message,
                target_session: Some(p.id),
            };
            
            let _ = core.event_tx.send(event);
            
            Ok(serde_json::json!({"interrupted": true}))
        }
    }).await;

    let core_clone = core.clone();
    registry.register("session.context", move |params| {
        let core = core_clone.clone();
        async move {
            #[derive(Deserialize)]
            struct Params { id: String }
            let p: Params = serde_json::from_value(params.unwrap_or(serde_json::Value::Null))
                .map_err(|_| RpcError::invalid_params("Missing 'id'"))?;
            
            let session = core.session_manager.get(&p.id).await
                .ok_or_else(|| RpcError::session_not_found(&p.id))?;
            
            let messages = core.storage.load_messages(Some(&p.id)).await.unwrap_or_default();
            let processes = core.storage.load_processes().await.unwrap_or_default()
                .into_iter().filter(|p| p.session_id == p.id).collect::<Vec<_>>();
            let activities = core.storage.load_activities(Some(&p.id)).await.unwrap_or_default();
            let tasks = core.storage.load_tasks().await.unwrap_or_default()
                .into_iter().filter(|t| t.session_id.as_deref() == Some(p.id.as_str())).collect::<Vec<_>>();
            
            Ok(serde_json::json!({
                "session": session,
                "messages": messages,
                "processes": processes,
                "activities": activities,
                "tasks": tasks,
            }))
        }
    }).await;
}