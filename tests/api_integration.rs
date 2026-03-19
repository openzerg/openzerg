use axum::{
    body::Body,
    http::{Request, Method, StatusCode},
    Router,
};
use tower::ServiceExt;
use http_body_util::BodyExt;
use openzerg::{
    api_server::{create_api_router, ApiState},
    storage::Storage,
    sse::SseManager,
    session::SessionManager,
    task::TaskManager,
    process::ProcessManager,
    tool::{ToolRegistry, ToolExecutor},
};
use std::sync::Arc;
use std::path::PathBuf;

async fn create_test_app() -> Router {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().to_path_buf();
    let openzerg_dir = workspace.join(".openzerg");
    
    let storage = Storage::new(openzerg_dir.clone());
    storage.init().await.unwrap();
    let storage = Arc::new(storage);
    
    let session_manager = Arc::new(SessionManager::new());
    let task_manager = Arc::new(TaskManager::new());
    let process_manager = Arc::new(ProcessManager::new(
        openzerg_dir.join("process_outputs"),
        tokio::sync::broadcast::channel(1).0,
    ));
    
    let (event_tx, _) = tokio::sync::broadcast::channel(100);
    
    let tool_registry = Arc::new(ToolRegistry::new());
    let tool_executor = Arc::new(ToolExecutor::new(
        tool_registry.clone(),
        workspace.clone(),
        openzerg_dir.clone(),
    ));
    
    let state = Arc::new(ApiState {
        storage,
        session_manager,
        task_manager,
        process_manager,
        event_tx,
        tool_registry,
        tool_executor,
    });
    
    create_api_router(state)
}

#[tokio::test]
async fn test_health_endpoint() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(Request::builder().method(Method::GET).uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn test_list_sessions_empty() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(Request::builder().method(Method::GET).uri("/api/sessions").body(Body::empty()).unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["success"].as_bool().unwrap());
    assert_eq!(json["data"]["total"], 0);
}

#[tokio::test]
async fn test_list_processes_empty() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(Request::builder().method(Method::GET).uri("/api/processes").body(Body::empty()).unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_list_tasks_empty() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(Request::builder().method(Method::GET).uri("/api/tasks").body(Body::empty()).unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_get_session_not_found() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(Request::builder().method(Method::GET).uri("/api/sessions/nonexistent").body(Body::empty()).unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(!json["success"].as_bool().unwrap());
    assert!(json["error"].as_str().unwrap().contains("not found"));
}

#[tokio::test]
async fn test_get_process_not_found() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(Request::builder().method(Method::GET).uri("/api/processes/nonexistent").body(Body::empty()).unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(!json["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_get_task_not_found() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(Request::builder().method(Method::GET).uri("/api/tasks/nonexistent").body(Body::empty()).unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(!json["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_list_tools() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(Request::builder().method(Method::GET).uri("/api/tools").body(Body::empty()).unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_send_message() {
    let app = create_test_app().await;
    
    let body = serde_json::to_string(&serde_json::json!({
        "content": "Hello, world!"
    })).unwrap();
    
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/message")
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["success"].as_bool().unwrap());
    assert!(json["data"]["sent"].as_bool().unwrap());
}

#[tokio::test]
async fn test_send_remind() {
    let app = create_test_app().await;
    
    let body = serde_json::to_string(&serde_json::json!({
        "message": "Remember this"
    })).unwrap();
    
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/remind")
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_list_activities_empty() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(Request::builder().method(Method::GET).uri("/api/activities").body(Body::empty()).unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_pagination_query() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(Request::builder().method(Method::GET).uri("/api/sessions?offset=0&limit=10").body(Body::empty()).unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_interrupt_session() {
    let app = create_test_app().await;
    
    let body = serde_json::to_string(&serde_json::json!({
        "message": "Stop!"
    })).unwrap();
    
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/sessions/test-session/interrupt")
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_chat_with_session() {
    let app = create_test_app().await;
    
    let body = serde_json::to_string(&serde_json::json!({
        "content": "Hello!"
    })).unwrap();
    
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/sessions/test-session/chat")
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_get_session_messages_empty() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(Request::builder().method(Method::GET).uri("/api/sessions/test-session/messages").body(Body::empty()).unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_get_process_output_not_found() {
    let app = create_test_app().await;
    
    let response = app
        .oneshot(Request::builder().method(Method::GET).uri("/api/processes/nonexistent/output").body(Body::empty()).unwrap())
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["success"].as_bool().unwrap());
}

#[tokio::test]
async fn test_execute_tool_not_found() {
    let app = create_test_app().await;
    
    let body = serde_json::to_string(&serde_json::json!({
        "args": {}
    })).unwrap();
    
    let response = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/tools/nonexistent/execute")
                .header("Content-Type", "application/json")
                .body(Body::from(body))
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
    
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(!json["success"].as_bool().unwrap());
}