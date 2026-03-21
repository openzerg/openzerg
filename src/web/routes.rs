use super::handlers;
use crate::api_server::ApiState;
use axum::Router;
use std::sync::Arc;

pub fn web_routes() -> Router<Arc<ApiState>> {
    Router::new()
        .route("/ui/", axum::routing::get(handlers::dashboard))
        .route(
            "/ui/sessions/{id}",
            axum::routing::get(handlers::session_detail),
        )
        .route(
            "/ui/providers",
            axum::routing::get(handlers::providers_list),
        )
        .route(
            "/ui/providers/new",
            axum::routing::get(handlers::provider_new),
        )
        .route(
            "/ui/providers/{id}",
            axum::routing::get(handlers::provider_edit),
        )
        .route("/ui/config", axum::routing::get(handlers::config_page))
        .route(
            "/api/providers",
            axum::routing::post(handlers::provider_create),
        )
        .route(
            "/api/providers/{id}",
            axum::routing::put(handlers::provider_update),
        )
        .route(
            "/api/providers/{id}/delete",
            axum::routing::post(handlers::provider_delete),
        )
        .route("/api/config", axum::routing::put(handlers::config_update))
}
