mod client;
pub mod types;

pub use client::LLMClient;
pub use types::{Message, MessageRole, ChatCompletionRequest, ChatCompletionResponse, ToolCall, ToolResult, ToolDefinition, FunctionDefinition, FunctionCall};