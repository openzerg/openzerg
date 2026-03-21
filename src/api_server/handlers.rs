use std::sync::Arc;
use std::convert::Infallible;
use axum::{
    extract::{Path, Query, State},
    Json,
    response::{
        sse::{Event, KeepAlive, Sse},
    },
};
use futures::stream::{self, Stream};
use super::state::ApiState;
use super::types::*;
use crate::storage::{StoredSession, StoredProcess, StoredTask};
use crate::protocol::AgentEvent;
use crate::sse::SseEvent;

pub async fn health() -> impl axum::response::IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

pub async fn list_sessions(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<PaginationQuery>,
) -> impl axum::response::IntoResponse {
    match state.storage.load_visible_sessions().await {
        Ok(mut sessions) => {
            let offset = query.offset.unwrap_or(0);
            let limit = query.limit.unwrap_or(100);
            sessions.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            let total = sessions.len();
            let sessions: Vec<_> = sessions.into_iter().skip(offset).take(limit).collect();
            ApiResponse::<serde_json::Value> {
                success: true,
                data: Some(serde_json::json!({
                    "sessions": sessions,
                    "total": total,
                })),
                error: None,
            }
        }
        Err(e) => ApiResponse::<serde_json::Value> {
            success: false,
            data: None,
            error: Some(e.to_string()),
        },
    }
}

pub async fn get_session(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> impl axum::response::IntoResponse {
    match state.storage.load_sessions().await {
        Ok(sessions) => {
            match sessions.into_iter().find(|s| s.id == id) {
                Some(session) => ApiResponse::<StoredSession> {
                    success: true,
                    data: Some(session),
                    error: None,
                },
                None => ApiResponse::<StoredSession> {
                    success: false,
                    data: None,
                    error: Some("Session not found".to_string()),
                },
            }
        }
        Err(e) => ApiResponse::<StoredSession> {
            success: false,
            data: None,
            error: Some(e.to_string()),
        },
    }
}

pub async fn get_session_messages(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Query(query): Query<PaginationQuery>,
) -> impl axum::response::IntoResponse {
    match state.storage.load_messages(Some(&id)).await {
        Ok(mut messages) => {
            let offset = query.offset.unwrap_or(0);
            let limit = query.limit.unwrap_or(100);
            messages.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
            let total = messages.len();
            let messages: Vec<_> = messages.into_iter().skip(offset).take(limit).collect();
            ApiResponse::<serde_json::Value> {
                success: true,
                data: Some(serde_json::json!({
                    "messages": messages,
                    "total": total,
                })),
                error: None,
            }
        }
        Err(e) => ApiResponse::<serde_json::Value> {
            success: false,
            data: None,
            error: Some(e.to_string()),
        },
    }
}

pub async fn send_to_session(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> impl axum::response::IntoResponse {
    let event = AgentEvent::Message {
        content: req.content,
        from: "user".to_string(),
    };
    
    let _ = state.event_tx.send(event);
    
    ApiResponse::<serde_json::Value> {
        success: true,
        data: Some(serde_json::json!({ "session_id": id })),
        error: None,
    }
}

pub async fn interrupt_session(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Json(req): Json<InterruptRequest>,
) -> impl axum::response::IntoResponse {
    let event = AgentEvent::Interrupt {
        message: req.message,
        target_session: Some(id),
    };
    
    let _ = state.event_tx.send(event);
    
    ApiResponse::<serde_json::Value> {
        success: true,
        data: Some(serde_json::json!({ "interrupted": true })),
        error: None,
    }
}

pub async fn list_processes(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<PaginationQuery>,
) -> impl axum::response::IntoResponse {
    match state.storage.load_processes().await {
        Ok(mut processes) => {
            let offset = query.offset.unwrap_or(0);
            let limit = query.limit.unwrap_or(100);
            processes.sort_by(|a, b| b.started_at.cmp(&a.started_at));
            let total = processes.len();
            let processes: Vec<_> = processes.into_iter().skip(offset).take(limit).collect();
            ApiResponse::<serde_json::Value> {
                success: true,
                data: Some(serde_json::json!({
                    "processes": processes,
                    "total": total,
                })),
                error: None,
            }
        }
        Err(e) => ApiResponse::<serde_json::Value> {
            success: false,
            data: None,
            error: Some(e.to_string()),
        },
    }
}

pub async fn get_process(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> impl axum::response::IntoResponse {
    match state.storage.load_processes().await {
        Ok(processes) => {
            match processes.into_iter().find(|p| p.id == id) {
                Some(process) => ApiResponse::<StoredProcess> {
                    success: true,
                    data: Some(process),
                    error: None,
                },
                None => ApiResponse::<StoredProcess> {
                    success: false,
                    data: None,
                    error: Some("Process not found".to_string()),
                },
            }
        }
        Err(e) => ApiResponse::<StoredProcess> {
            success: false,
            data: None,
            error: Some(e.to_string()),
        },
    }
}

pub async fn get_process_output(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
    Query(query): Query<OutputQuery>,
) -> impl axum::response::IntoResponse {
    let stream = query.stream.as_deref().unwrap_or("stdout");
    let content = match state.storage.read_process_output(&id, stream).await {
        Ok(c) => c,
        Err(e) => {
            return ApiResponse::<serde_json::Value> {
                success: false,
                data: None,
                error: Some(e.to_string()),
            };
        }
    };
    
    let offset = query.offset.unwrap_or(0) as usize;
    let limit = query.limit.unwrap_or(10000);
    let end = (offset + limit).min(content.len());
    let slice = &content[offset..end];
    
    ApiResponse::<serde_json::Value> {
        success: true,
        data: Some(serde_json::json!({
            "process_id": id,
            "stream": stream,
            "content": slice,
            "total_size": content.len(),
        })),
        error: None,
    }
}

pub async fn list_tasks(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<PaginationQuery>,
) -> impl axum::response::IntoResponse {
    match state.storage.load_tasks().await {
        Ok(mut tasks) => {
            let offset = query.offset.unwrap_or(0);
            let limit = query.limit.unwrap_or(100);
            tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            let total = tasks.len();
            let tasks: Vec<_> = tasks.into_iter().skip(offset).take(limit).collect();
            ApiResponse::<serde_json::Value> {
                success: true,
                data: Some(serde_json::json!({
                    "tasks": tasks,
                    "total": total,
                })),
                error: None,
            }
        }
        Err(e) => ApiResponse::<serde_json::Value> {
            success: false,
            data: None,
            error: Some(e.to_string()),
        },
    }
}

pub async fn get_task(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> impl axum::response::IntoResponse {
    match state.storage.load_tasks().await {
        Ok(tasks) => {
            match tasks.into_iter().find(|t| t.id == id) {
                Some(task) => ApiResponse::<StoredTask> {
                    success: true,
                    data: Some(task),
                    error: None,
                },
                None => ApiResponse::<StoredTask> {
                    success: false,
                    data: None,
                    error: Some("Task not found".to_string()),
                },
            }
        }
        Err(e) => ApiResponse::<StoredTask> {
            success: false,
            data: None,
            error: Some(e.to_string()),
        },
    }
}

pub async fn list_activities(
    State(state): State<Arc<ApiState>>,
    Query(query): Query<PaginationQuery>,
) -> impl axum::response::IntoResponse {
    match state.storage.load_activities(None).await {
        Ok(mut activities) => {
            let offset = query.offset.unwrap_or(0);
            let limit = query.limit.unwrap_or(100);
            activities.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            let total = activities.len();
            let activities: Vec<_> = activities.into_iter().skip(offset).take(limit).collect();
            ApiResponse::<serde_json::Value> {
                success: true,
                data: Some(serde_json::json!({
                    "activities": activities,
                    "total": total,
                })),
                error: None,
            }
        }
        Err(e) => ApiResponse::<serde_json::Value> {
            success: false,
            data: None,
            error: Some(e.to_string()),
        },
    }
}

pub async fn send_message(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<MessageRequest>,
) -> impl axum::response::IntoResponse {
    let event = AgentEvent::Message {
        content: req.content,
        from: "user".to_string(),
    };
    
    let _ = state.event_tx.send(event);
    
    ApiResponse::<serde_json::Value> {
        success: true,
        data: Some(serde_json::json!({ "sent": true })),
        error: None,
    }
}

pub async fn send_remind(
    State(state): State<Arc<ApiState>>,
    Json(req): Json<RemindRequest>,
) -> impl axum::response::IntoResponse {
    let id = uuid::Uuid::new_v4().to_string();
    let event = AgentEvent::Remind {
        id,
        message: req.message,
    };
    
    let _ = state.event_tx.send(event);
    
    ApiResponse::<serde_json::Value> {
        success: true,
        data: Some(serde_json::json!({ "sent": true })),
        error: None,
    }
}

pub async fn list_tools(
    State(state): State<Arc<ApiState>>,
) -> impl axum::response::IntoResponse {
    let definitions = state.tool_registry.tool_definitions().await;
    
    ApiResponse::<serde_json::Value> {
        success: true,
        data: Some(serde_json::json!({
            "tools": definitions,
        })),
        error: None,
    }
}

pub async fn execute_tool(
    State(state): State<Arc<ApiState>>,
    Path(name): Path<String>,
    Json(req): Json<ExecuteToolRequest>,
) -> impl axum::response::IntoResponse {
    let session_id = req.session_id.unwrap_or_else(|| "default".to_string());
    let message_id = uuid::Uuid::new_v4().to_string();
    
    match state.tool_registry.execute(&name, req.args, crate::tool::ToolContext {
        session_id: session_id.clone(),
        message_id: message_id.clone(),
        workspace: std::path::PathBuf::from("/workspace"),
        openzerg_dir: std::path::PathBuf::from("/workspace/.openzerg"),
        abort: tokio_util::sync::CancellationToken::new(),
        file_manager: Arc::new(crate::file::FileManager::new(std::path::PathBuf::from("/workspace"))),
        process_manager: state.process_manager.clone(),
    }).await {
        Ok(result) => ApiResponse::<serde_json::Value> {
            success: true,
            data: Some(serde_json::json!({
                "title": result.title,
                "output": result.output,
                "metadata": result.metadata,
                "attachments": result.attachments,
                "truncated": result.truncated,
            })),
            error: None,
        },
        Err(e) => ApiResponse::<serde_json::Value> {
            success: false,
            data: None,
            error: Some(e.to_string()),
        },
    }
}

pub async fn get_session_context(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<String>,
) -> impl axum::response::IntoResponse {
    let session = state.storage.load_sessions().await
        .ok()
        .and_then(|sessions| sessions.into_iter().find(|s| s.id == id));
    
    let session = match session {
        Some(s) => s,
        None => return ApiResponse::<serde_json::Value> {
            success: false,
            data: None,
            error: Some("Session not found".to_string()),
        },
    };
    
    let messages = state.storage.load_messages(Some(&id)).await.unwrap_or_default();
    
    let tool_results = state.storage.get_tool_results(&id).await.unwrap_or_default();
    
    let processes = state.storage.load_processes().await
        .unwrap_or_default()
        .into_iter()
        .filter(|p| p.session_id == id)
        .collect::<Vec<_>>();
    
    let activities = state.storage.load_activities(Some(&id)).await.unwrap_or_default();
    
    let tasks = state.storage.load_tasks().await
        .unwrap_or_default()
        .into_iter()
        .filter(|t| t.session_id.as_deref() == Some(id.as_str()))
        .collect::<Vec<_>>();
    
    let tool_calls: Vec<_> = messages.iter()
        .filter_map(|m| m.tool_calls.clone())
        .flatten()
        .collect();
    
    ApiResponse::<serde_json::Value> {
        success: true,
        data: Some(serde_json::json!({
            "session": session,
            "messages": messages,
            "tool_calls": tool_calls,
            "tool_results": tool_results,
            "processes": processes,
            "activities": activities,
            "tasks": tasks,
        })),
        error: None,
    }
}

pub async fn session_events(
    Path(session_id): Path<String>,
    State(state): State<Arc<ApiState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.event_tx.subscribe();
    
    let stream = async_stream::stream! {
        yield Ok(Event::default().event("connected").data("Connected to session events"));
        
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let sse_event = match event {
                        AgentEvent::Message { content, from } => {
                            if from == "user" {
                                SseEvent::user_message(&content)
                            } else {
                                SseEvent::response(&format!("[{}] {}", from, content))
                            }
                        }
                        AgentEvent::Query { query_id, question } => {
                            SseEvent::response(&format!("[Query {}] {}", query_id, question))
                        }
                        AgentEvent::AssignTask { task_id, title, .. } => {
                            SseEvent::tool_call("assign_task", &serde_json::json!({
                                "task_id": task_id,
                                "title": title
                            }))
                        }
                        AgentEvent::Remind { id, message } => {
                            SseEvent::response(&format!("[Remind {}] {}", id, message))
                        }
                        AgentEvent::Interrupt { message, .. } => {
                            SseEvent::error(&message)
                        }
                        AgentEvent::ProcessNotification { process_id, event, output_preview } => {
                            SseEvent::tool_result(&format!(
                                "[Process {}] {:?} {}",
                                process_id,
                                event,
                                output_preview.unwrap_or_default()
                            ))
                        }
                        AgentEvent::ConfigUpdate { .. } => {
                            SseEvent::session_created("config_updated")
                        }
                        AgentEvent::ResourceWarning { resource, message } => {
                            SseEvent::error(&format!("[{:?}] {}", resource, message))
                        }
                        AgentEvent::SessionCreated { session_id, purpose } => {
                            SseEvent::session_created(&format!("{}:{}", session_id, purpose))
                        }
                        AgentEvent::Thinking { session_id, content } => {
                            SseEvent::thinking(&format!("[{}] {}", session_id, content))
                        }
                        AgentEvent::Response { session_id, content } => {
                            SseEvent::response(&format!("[{}] {}", session_id, content))
                        }
                        AgentEvent::Done { session_id } => {
                            SseEvent::done(&session_id)
                        }
                        AgentEvent::Error { session_id, message } => {
                            SseEvent::error(&format!("[{}] {}", session_id, message))
                        }
                        AgentEvent::SubSessionResult { parent_session_id, child_session_id, child_session_type, status, summary, details: _ } => {
                            SseEvent::response(&format!(
                                "[SubSession] {} {} -> {}: {} | {}",
                                child_session_type, child_session_id, parent_session_id, status, summary
                            ))
                        }
                        AgentEvent::SessionTask { session_id, task, context: _ } => {
                            SseEvent::thinking(&format!("[SessionTask] {} -> {}", session_id, task))
                        }
                        AgentEvent::UserMessage { content } => {
                            SseEvent::user_message(&content)
                        }
                    };
                    
                    let data_json = serde_json::to_string(&sse_event.data).unwrap_or_default();
                    yield Ok(Event::default()
                        .event(sse_event.event_type)
                        .data(data_json));
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
            }
        }
    };
    
    Sse::new(stream).keep_alive(KeepAlive::default())
}