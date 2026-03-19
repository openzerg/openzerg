use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

impl Message {
    pub fn system(content: &str) -> Self {
        Self {
            role: MessageRole::System,
            content: content.to_string(),
            tool_calls: None,
        }
    }

    pub fn user(content: &str) -> Self {
        Self {
            role: MessageRole::User,
            content: content.to_string(),
            tool_calls: None,
        }
    }

    pub fn assistant(content: &str) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.to_string(),
            tool_calls: None,
        }
    }

    pub fn assistant_with_tools(content: &str, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.to_string(),
            tool_calls: Some(tool_calls),
        }
    }

    pub fn tool_result(_tool_call_id: &str, content: &str) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.to_string(),
            tool_calls: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_system() {
        let msg = Message::system("You are helpful");
        assert_eq!(msg.role, MessageRole::System);
        assert_eq!(msg.content, "You are helpful");
        assert!(msg.tool_calls.is_none());
    }

    #[test]
    fn test_message_user() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_message_assistant() {
        let msg = Message::assistant("Hi there");
        assert_eq!(msg.role, MessageRole::Assistant);
        assert_eq!(msg.content, "Hi there");
    }

    #[test]
    fn test_message_assistant_with_tools() {
        let tool_call = ToolCall {
            id: "call-1".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "read".to_string(),
                arguments: "{}".to_string(),
            },
        };
        let msg = Message::assistant_with_tools("Working", vec![tool_call]);
        assert_eq!(msg.role, MessageRole::Assistant);
        assert!(msg.tool_calls.is_some());
    }

    #[test]
    fn test_message_tool_result() {
        let msg = Message::tool_result("call-1", "result");
        assert_eq!(msg.role, MessageRole::Tool);
        assert_eq!(msg.content, "result");
    }

    #[test]
    fn test_message_role_serialization() {
        let role = MessageRole::User;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, "\"user\"");
    }

    #[test]
    fn test_message_role_deserialization() {
        let json = "\"assistant\"";
        let role: MessageRole = serde_json::from_str(json).unwrap();
        assert_eq!(role, MessageRole::Assistant);
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message::user("test");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_tool_call_serialization() {
        let call = ToolCall {
            id: "id-1".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "bash".to_string(),
                arguments: "{}".to_string(),
            },
        };
        let json = serde_json::to_string(&call).unwrap();
        assert!(json.contains("id-1"));
        assert!(json.contains("bash"));
    }

    #[test]
    fn test_function_call_serialization() {
        let func = FunctionCall {
            name: "read".to_string(),
            arguments: "{\"path\":\"/tmp\"}".to_string(),
        };
        let json = serde_json::to_string(&func).unwrap();
        assert!(json.contains("read"));
    }

    #[test]
    fn test_tool_result_serialization() {
        let result = ToolResult {
            tool_call_id: "call-1".to_string(),
            content: "success".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("call-1"));
        assert!(json.contains("success"));
    }

    #[test]
    fn test_chat_completion_request_serialization() {
        let req = ChatCompletionRequest {
            model: "gpt-4".to_string(),
            messages: vec![Message::user("hi")],
            tools: None,
            stream: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("gpt-4"));
    }

    #[test]
    fn test_tool_definition_serialization() {
        let def = ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "test".to_string(),
                description: "A test".to_string(),
                parameters: serde_json::json!({}),
            },
        };
        let json = serde_json::to_string(&def).unwrap();
        assert!(json.contains("function"));
        assert!(json.contains("test"));
    }

    #[test]
    fn test_function_definition_serialization() {
        let func = FunctionDefinition {
            name: "bash".to_string(),
            description: "Run command".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        };
        let json = serde_json::to_string(&func).unwrap();
        assert!(json.contains("bash"));
    }

    #[test]
    fn test_usage_deserialization() {
        let json = r#"{"prompt_tokens":10,"completion_tokens":20,"total_tokens":30}"#;
        let usage: Usage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 20);
        assert_eq!(usage.total_tokens, 30);
    }

    #[test]
    fn test_choice_deserialization() {
        let json = r#"{"index":0,"message":{"role":"assistant","content":"hello"},"finish_reason":"stop"}"#;
        let choice: Choice = serde_json::from_str(json).unwrap();
        assert_eq!(choice.index, 0);
        assert_eq!(choice.message.content, "hello");
    }

    #[test]
    fn test_chat_completion_response_deserialization() {
        let json = r#"{"id":"chat-1","choices":[{"index":0,"message":{"role":"assistant","content":"hi"},"finish_reason":"stop"}],"usage":{"prompt_tokens":5,"completion_tokens":2,"total_tokens":7}}"#;
        let resp: ChatCompletionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, "chat-1");
        assert_eq!(resp.choices.len(), 1);
    }
}
