use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PaginationQuery {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct InterruptRequest {
    pub message: String,
    pub level: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}

#[derive(Debug, Deserialize)]
pub struct OutputQuery {
    pub stream: Option<String>,
    pub offset: Option<u64>,
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct MessageRequest {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct RemindRequest {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteToolRequest {
    pub args: serde_json::Value,
    pub session_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_query_deserialize() {
        let json = r#"{"offset":10,"limit":50}"#;
        let query: PaginationQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.offset, Some(10));
        assert_eq!(query.limit, Some(50));
    }

    #[test]
    fn test_pagination_query_defaults() {
        let json = r#"{}"#;
        let query: PaginationQuery = serde_json::from_str(json).unwrap();
        assert!(query.offset.is_none());
        assert!(query.limit.is_none());
    }

    #[test]
    fn test_send_message_request_deserialize() {
        let json = r#"{"content":"hello"}"#;
        let req: SendMessageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "hello");
    }

    #[test]
    fn test_interrupt_request_deserialize() {
        let json = r#"{"message":"stop","level":"high"}"#;
        let req: InterruptRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.message, "stop");
        assert_eq!(req.level, Some("high".to_string()));
    }

    #[test]
    fn test_api_response_success() {
        let resp: ApiResponse<String> = ApiResponse {
            success: true,
            data: Some("result".to_string()),
            error: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("true"));
        assert!(json.contains("result"));
    }

    #[test]
    fn test_api_response_error() {
        let resp: ApiResponse<String> = ApiResponse {
            success: false,
            data: None,
            error: Some("failed".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("failed"));
    }

    #[test]
    fn test_output_query_deserialize() {
        let json = r#"{"stream":"stdout","offset":100,"limit":50}"#;
        let query: OutputQuery = serde_json::from_str(json).unwrap();
        assert_eq!(query.stream, Some("stdout".to_string()));
        assert_eq!(query.offset, Some(100));
    }

    #[test]
    fn test_message_request_deserialize() {
        let json = r#"{"content":"test message"}"#;
        let req: MessageRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "test message");
    }

    #[test]
    fn test_remind_request_deserialize() {
        let json = r#"{"message":"reminder"}"#;
        let req: RemindRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.message, "reminder");
    }

    #[test]
    fn test_execute_tool_request_deserialize() {
        let json = r#"{"args":{"path":"/tmp"},"session_id":"s1"}"#;
        let req: ExecuteToolRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.session_id, Some("s1".to_string()));
    }
}
