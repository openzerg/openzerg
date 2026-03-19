use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("LLM error: {0}")]
    LLM(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Task error: {0}")]
    Task(String),

    #[error("Process error: {0}")]
    Process(String),

    #[error("File error: {0}")]
    File(String),

    #[error("Tool error: {0}")]
    Tool(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("SSE channel closed")]
    SseChannelClosed,

    #[error("Interrupted")]
    Interrupted,

    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

impl From<tokio::task::JoinError> for Error {
    fn from(e: tokio::task::JoinError) -> Self {
        Error::Other(anyhow::anyhow!("Task join error: {}", e))
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_io() {
        let err = Error::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(err.to_string().contains("IO error"));
    }

    #[test]
    fn test_error_display_serialization() {
        let json = serde_json::from_str::<i32>("not a number");
        let err = Error::Serialization(json.unwrap_err());
        assert!(err.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_error_display_not_found() {
        let err = Error::NotFound("resource".to_string());
        assert_eq!(err.to_string(), "Not found: resource");
    }

    #[test]
    fn test_error_display_validation() {
        let err = Error::Validation("invalid input".to_string());
        assert_eq!(err.to_string(), "Validation error: invalid input");
    }

    #[test]
    fn test_error_display_websocket() {
        let err = Error::WebSocket("connection failed".to_string());
        assert_eq!(err.to_string(), "WebSocket error: connection failed");
    }

    #[test]
    fn test_error_display_connection() {
        let err = Error::Connection("timeout".to_string());
        assert_eq!(err.to_string(), "Connection error: timeout");
    }

    #[test]
    fn test_error_display_llm() {
        let err = Error::LLM("rate limit".to_string());
        assert_eq!(err.to_string(), "LLM error: rate limit");
    }

    #[test]
    fn test_error_display_session() {
        let err = Error::Session("not found".to_string());
        assert_eq!(err.to_string(), "Session error: not found");
    }

    #[test]
    fn test_error_display_task() {
        let err = Error::Task("failed".to_string());
        assert_eq!(err.to_string(), "Task error: failed");
    }

    #[test]
    fn test_error_display_process() {
        let err = Error::Process("killed".to_string());
        assert_eq!(err.to_string(), "Process error: killed");
    }

    #[test]
    fn test_error_display_file() {
        let err = Error::File("permission denied".to_string());
        assert_eq!(err.to_string(), "File error: permission denied");
    }

    #[test]
    fn test_error_display_tool() {
        let err = Error::Tool("not found".to_string());
        assert_eq!(err.to_string(), "Tool error: not found");
    }

    #[test]
    fn test_error_display_config() {
        let err = Error::Config("invalid".to_string());
        assert_eq!(err.to_string(), "Config error: invalid");
    }

    #[test]
    fn test_error_display_sse_channel_closed() {
        let err = Error::SseChannelClosed;
        assert_eq!(err.to_string(), "SSE channel closed");
    }

    #[test]
    fn test_error_display_interrupted() {
        let err = Error::Interrupted;
        assert_eq!(err.to_string(), "Interrupted");
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let err: Error = io_err.into();
        assert!(matches!(err, Error::Io(_)));
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<i32>("invalid").unwrap_err();
        let err: Error = json_err.into();
        assert!(matches!(err, Error::Serialization(_)));
    }

    #[test]
    fn test_result_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
    }

    #[test]
    fn test_result_err() {
        let result: Result<i32> = Err(Error::NotFound("test".to_string()));
        assert!(result.is_err());
    }
}
