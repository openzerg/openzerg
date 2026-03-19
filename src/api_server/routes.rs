use super::handlers::*;
use super::state::ApiState;
use super::types::*;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

pub fn create_api_router(state: Arc<ApiState>) -> Router {
    Router::new()
        .route("/api/sessions", get(list_sessions))
        .route("/api/sessions/{id}", get(get_session))
        .route("/api/sessions/{id}/messages", get(get_session_messages))
        .route("/api/sessions/{id}/chat", post(send_to_session))
        .route("/api/sessions/{id}/interrupt", post(interrupt_session))
        .route("/api/sessions/{id}/context", get(get_session_context))
        .route("/api/processes", get(list_processes))
        .route("/api/processes/{id}", get(get_process))
        .route("/api/processes/{id}/output", get(get_process_output))
        .route("/api/tasks", get(list_tasks))
        .route("/api/tasks/{id}", get(get_task))
        .route("/api/activities", get(list_activities))
        .route("/api/message", post(send_message))
        .route("/api/remind", post(send_remind))
        .route("/api/tools", get(list_tools))
        .route("/api/tools/{name}/execute", post(execute_tool))
        .route("/health", get(health))
        .layer(CorsLayer::permissive())
        .with_state(state)
}
