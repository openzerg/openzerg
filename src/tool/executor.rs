use std::sync::Arc;
use crate::error::Result;
use crate::tool::{ToolRegistry, ToolContext, ToolResult};
use crate::llm::{ToolCall, ToolDefinition, FunctionDefinition, Message};
use tokio_util::sync::CancellationToken;

pub struct ToolExecutor {
    registry: Arc<ToolRegistry>,
    workspace: std::path::PathBuf,
    openzerg_dir: std::path::PathBuf,
}

impl ToolExecutor {
    pub fn new(
        registry: Arc<ToolRegistry>,
        workspace: std::path::PathBuf,
        openzerg_dir: std::path::PathBuf,
    ) -> Self {
        Self {
            registry,
            workspace,
            openzerg_dir,
        }
    }
    
    pub async fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        let definitions = self.registry.tool_definitions().await;
        
        definitions.into_iter().map(|d| ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: d.name,
                description: d.description,
                parameters: d.parameters,
            },
        }).collect()
    }
    
    pub async fn execute_tool_call(
        &self,
        tool_call: &ToolCall,
        session_id: &str,
        message_id: &str,
    ) -> Result<ToolResult> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .unwrap_or_else(|_| serde_json::json!({}));
        
        let ctx = ToolContext {
            session_id: session_id.to_string(),
            message_id: message_id.to_string(),
            workspace: self.workspace.clone(),
            openzerg_dir: self.openzerg_dir.clone(),
            abort: CancellationToken::new(),
            file_manager: Arc::new(crate::file::FileManager::new(self.workspace.clone())),
            process_manager: Arc::new(crate::process::ProcessManager::new(
                self.openzerg_dir.join("process_outputs"),
                tokio::sync::broadcast::channel(1).0,
            )),
        };
        
        self.registry.execute(&tool_call.function.name, args, ctx).await
    }
    
    pub async fn execute_tool_calls(
        &self,
        tool_calls: &[ToolCall],
        session_id: &str,
        message_id: &str,
    ) -> Vec<(String, Result<ToolResult>)> {
        let mut results = Vec::new();
        
        for tool_call in tool_calls {
            let tool_call_id = tool_call.id.clone();
            let result = self.execute_tool_call(tool_call, session_id, message_id).await;
            results.push((tool_call_id, result));
        }
        
        results
    }
    
    pub fn format_tool_results(
        &self,
        results: Vec<(String, Result<ToolResult>)>,
    ) -> Vec<Message> {
        results.into_iter().map(|(tool_call_id, result)| {
            let content = match result {
                Ok(r) => r.output,
                Err(e) => format!("Error: {}", e),
            };
            
            Message::tool_result(&tool_call_id, &content)
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_executor_new() {
        let registry = Arc::new(ToolRegistry::new());
        let executor = ToolExecutor::new(
            registry,
            std::path::PathBuf::from("/tmp"),
            std::path::PathBuf::from("/tmp/.openzerg"),
        );
        
        let defs = executor.get_tool_definitions().await;
        assert!(defs.is_empty());
    }

    #[tokio::test]
    async fn test_tool_executor_get_definitions() {
        let registry = Arc::new(ToolRegistry::new());
        let executor = ToolExecutor::new(
            registry,
            std::path::PathBuf::from("/tmp"),
            std::path::PathBuf::from("/tmp/.openzerg"),
        );
        
        let defs = executor.get_tool_definitions().await;
        assert!(defs.is_empty());
    }

    #[test]
    fn test_format_tool_results_empty() {
        let registry = Arc::new(ToolRegistry::new());
        let executor = ToolExecutor::new(
            registry,
            std::path::PathBuf::from("/tmp"),
            std::path::PathBuf::from("/tmp/.openzerg"),
        );
        
        let results = vec![];
        let messages = executor.format_tool_results(results);
        assert!(messages.is_empty());
    }

    #[test]
    fn test_format_tool_results_success() {
        let registry = Arc::new(ToolRegistry::new());
        let executor = ToolExecutor::new(
            registry,
            std::path::PathBuf::from("/tmp"),
            std::path::PathBuf::from("/tmp/.openzerg"),
        );
        
        let result = ToolResult {
            title: "test".to_string(),
            output: "success output".to_string(),
            metadata: serde_json::json!({}),
            attachments: vec![],
            truncated: false,
        };
        
        let results = vec![("call-1".to_string(), Ok(result))];
        let messages = executor.format_tool_results(results);
        
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "success output");
    }

    #[test]
    fn test_format_tool_results_error() {
        let registry = Arc::new(ToolRegistry::new());
        let executor = ToolExecutor::new(
            registry,
            std::path::PathBuf::from("/tmp"),
            std::path::PathBuf::from("/tmp/.openzerg"),
        );
        
        let error = crate::error::Error::Tool("test error".to_string());
        let results = vec![("call-1".to_string(), Err(error))];
        let messages = executor.format_tool_results(results);
        
        assert_eq!(messages.len(), 1);
        assert!(messages[0].content.contains("test error"));
    }

    #[test]
    fn test_format_tool_results_multiple() {
        let registry = Arc::new(ToolRegistry::new());
        let executor = ToolExecutor::new(
            registry,
            std::path::PathBuf::from("/tmp"),
            std::path::PathBuf::from("/tmp/.openzerg"),
        );
        
        let result1 = ToolResult {
            title: "r1".to_string(),
            output: "output1".to_string(),
            metadata: serde_json::json!({}),
            attachments: vec![],
            truncated: false,
        };
        
        let result2 = ToolResult {
            title: "r2".to_string(),
            output: "output2".to_string(),
            metadata: serde_json::json!({}),
            attachments: vec![],
            truncated: false,
        };
        
        let results = vec![
            ("call-1".to_string(), Ok(result1)),
            ("call-2".to_string(), Ok(result2)),
        ];
        let messages = executor.format_tool_results(results);
        
        assert_eq!(messages.len(), 2);
    }
}