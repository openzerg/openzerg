mod client;
pub mod types;

pub use client::{LLMClient, StreamChunk};
pub use types::{Message, MessageRole, ChatCompletionRequest, ChatCompletionResponse, ToolCall, ToolResult, ToolDefinition, FunctionDefinition, FunctionCall};