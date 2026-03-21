use serde::Serialize;
use tokio::sync::mpsc;
use crate::error::Error;

#[derive(Debug, Clone, Serialize)]
pub struct SseEvent {
    #[serde(skip)]
    pub event_type: &'static str,
    #[serde(flatten)]
    pub data: SseEventData,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum SseEventData {
    SessionCreated { session_id: String },
    Thinking { content: String, session_id: Option<String> },
    ToolCall { tool: String, args: serde_json::Value },
    ToolResult { content: String },
    Response { content: String, session_id: Option<String> },
    UserMessage { content: String },
    Done { session_id: String },
    Error { message: String, session_id: Option<String> },
}

impl SseEvent {
    pub fn session_created(session_id: &str) -> Self {
        Self {
            event_type: "session_created",
            data: SseEventData::SessionCreated {
                session_id: session_id.to_string(),
            },
        }
    }

    pub fn thinking(content: &str, session_id: Option<String>) -> Self {
        Self {
            event_type: "thinking",
            data: SseEventData::Thinking {
                content: content.to_string(),
                session_id,
            },
        }
    }

    pub fn tool_call(tool: &str, args: &serde_json::Value) -> Self {
        Self {
            event_type: "tool_call",
            data: SseEventData::ToolCall {
                tool: tool.to_string(),
                args: args.clone(),
            },
        }
    }

    pub fn tool_result(content: &str) -> Self {
        Self {
            event_type: "tool_result",
            data: SseEventData::ToolResult {
                content: content.to_string(),
            },
        }
    }

    pub fn response(content: &str, session_id: Option<String>) -> Self {
        Self {
            event_type: "response",
            data: SseEventData::Response {
                content: content.to_string(),
                session_id,
            },
        }
    }

    pub fn user_message(content: &str) -> Self {
        Self {
            event_type: "user_message",
            data: SseEventData::UserMessage {
                content: content.to_string(),
            },
        }
    }

    pub fn done(session_id: &str) -> Self {
        Self {
            event_type: "done",
            data: SseEventData::Done {
                session_id: session_id.to_string(),
            },
        }
    }

    pub fn error(message: &str, session_id: Option<String>) -> Self {
        Self {
            event_type: "error",
            data: SseEventData::Error {
                message: message.to_string(),
                session_id,
            },
        }
    }

    pub fn to_sse_string(&self) -> String {
        let json = serde_json::to_string(&self.data).unwrap_or_default();
        format!("event: {}\ndata: {}\n\n", self.event_type, json)
    }
}

pub type SseSender = mpsc::Sender<SseEvent>;

pub struct SseManager {
    channels: std::collections::HashMap<String, SseSender>,
}

impl SseManager {
    pub fn new() -> Self {
        Self {
            channels: std::collections::HashMap::new(),
        }
    }

    pub fn register(&mut self, query_id: String, tx: SseSender) {
        self.channels.insert(query_id, tx);
    }

    pub fn unregister(&mut self, query_id: &str) {
        self.channels.remove(query_id);
    }

    pub fn get(&self, query_id: &str) -> Option<&SseSender> {
        self.channels.get(query_id)
    }

    pub async fn send(&self, query_id: &str, event: SseEvent) -> crate::error::Result<()> {
        if let Some(tx) = self.channels.get(query_id) {
            tx.send(event).await.map_err(|_| Error::SseChannelClosed)?;
        }
        Ok(())
    }
}

impl Default for SseManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sse_event_session_created() {
        let event = SseEvent::session_created("sess-1");
        assert_eq!(event.event_type, "session_created");
        match event.data {
            SseEventData::SessionCreated { session_id } => assert_eq!(session_id, "sess-1"),
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_sse_event_thinking() {
        let event = SseEvent::thinking("processing...");
        assert_eq!(event.event_type, "thinking");
        match event.data {
            SseEventData::Thinking { content } => assert_eq!(content, "processing..."),
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_sse_event_tool_call() {
        let args = serde_json::json!({"path": "/tmp"});
        let event = SseEvent::tool_call("read", &args);
        assert_eq!(event.event_type, "tool_call");
        match event.data {
            SseEventData::ToolCall { tool, .. } => assert_eq!(tool, "read"),
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_sse_event_tool_result() {
        let event = SseEvent::tool_result("success");
        assert_eq!(event.event_type, "tool_result");
        match event.data {
            SseEventData::ToolResult { content } => assert_eq!(content, "success"),
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_sse_event_response() {
        let event = SseEvent::response("Hello!");
        assert_eq!(event.event_type, "response");
        match event.data {
            SseEventData::Response { content } => assert_eq!(content, "Hello!"),
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_sse_event_done() {
        let event = SseEvent::done("sess-1");
        assert_eq!(event.event_type, "done");
        match event.data {
            SseEventData::Done { session_id } => assert_eq!(session_id, "sess-1"),
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_sse_event_error() {
        let event = SseEvent::error("Something went wrong");
        assert_eq!(event.event_type, "error");
        match event.data {
            SseEventData::Error { message } => assert_eq!(message, "Something went wrong"),
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_sse_event_to_sse_string() {
        let event = SseEvent::response("test");
        let sse = event.to_sse_string();
        assert!(sse.starts_with("event: response\n"));
        assert!(sse.contains("data:"));
    }

    #[test]
    fn test_sse_manager_new() {
        let manager = SseManager::new();
        assert!(manager.get("nonexistent").is_none());
    }

    #[test]
    fn test_sse_manager_default() {
        let manager = SseManager::default();
        assert!(manager.get("x").is_none());
    }

    #[test]
    fn test_sse_manager_register() {
        let mut manager = SseManager::new();
        let (tx, _rx) = mpsc::channel(10);
        manager.register("query-1".to_string(), tx);
        assert!(manager.get("query-1").is_some());
    }

    #[test]
    fn test_sse_manager_unregister() {
        let mut manager = SseManager::new();
        let (tx, _rx) = mpsc::channel(10);
        manager.register("query-1".to_string(), tx);
        manager.unregister("query-1");
        assert!(manager.get("query-1").is_none());
    }

    #[tokio::test]
    async fn test_sse_manager_send_no_channel() {
        let manager = SseManager::new();
        let event = SseEvent::response("test");
        let result = manager.send("nonexistent", event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_sse_manager_send_with_channel() {
        let mut manager = SseManager::new();
        let (tx, mut rx) = mpsc::channel(10);
        manager.register("q1".to_string(), tx);
        
        let event = SseEvent::response("hello");
        manager.send("q1", event).await.unwrap();
        
        let received = rx.recv().await.unwrap();
        match received.data {
            SseEventData::Response { content } => assert_eq!(content, "hello"),
            _ => panic!("Wrong event"),
        }
    }

    #[test]
    fn test_sse_event_data_serialization() {
        let data = SseEventData::Thinking { content: "think".to_string() };
        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("think"));
    }
}