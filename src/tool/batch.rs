use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::error::{Error, Result};
use super::traits::{Tool, ToolContext, ToolResult};
use super::schema::generate_schema;

const MIN_BATCH_SIZE: usize = 1;
const MAX_BATCH_SIZE: usize = 25;
const DESCRIPTION: &str = include_str!("prompts/batch.md");

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BatchParams {
    #[schemars(description = "Array of tool calls to execute in parallel (1-25 calls)")]
    pub calls: Vec<BatchCall>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BatchCall {
    #[schemars(description = "The tool ID to call")]
    pub tool: String,
    #[schemars(description = "The arguments to pass to the tool")]
    pub args: Value,
}

pub struct BatchTool {
    registry: std::sync::Arc<super::registry::ToolRegistry>,
}

impl BatchTool {
    pub fn new(registry: std::sync::Arc<super::registry::ToolRegistry>) -> Self {
        Self { registry }
    }
}

#[async_trait]
impl Tool for BatchTool {
    fn id(&self) -> &str { "batch" }
    
    fn description(&self) -> &str {
        DESCRIPTION
    }
    
    fn parameters_schema(&self) -> Value {
        generate_schema::<BatchParams>()
    }
    
    async fn execute(&self, args: Value, ctx: ToolContext) -> Result<ToolResult> {
        let params: BatchParams = serde_json::from_value(args)
            .map_err(|e| Error::Tool(format!("Invalid parameters: {}", e)))?;
        
        if params.calls.len() < MIN_BATCH_SIZE {
            return Err(Error::Tool(format!("Batch must contain at least {} call", MIN_BATCH_SIZE)));
        }
        
        if params.calls.len() > MAX_BATCH_SIZE {
            return Err(Error::Tool(format!("Batch cannot contain more than {} calls", MAX_BATCH_SIZE)));
        }
        
        let mut handles = Vec::new();
        
        for call in params.calls {
            let registry = self.registry.clone();
            let tool_id = call.tool.clone();
            let tool_id_for_handle = tool_id.clone();
            let call_args = call.args.clone();
            let ctx = ctx.clone();
            
            let handle = tokio::spawn(async move {
                registry.execute(&tool_id, call_args, ctx).await
            });
            
            handles.push((tool_id_for_handle, handle));
        }
        
        let mut results = Vec::new();
        
        for (tool_id, handle) in handles {
            match handle.await {
                Ok(result) => match result {
                    Ok(r) => results.push(BatchResult {
                        tool: tool_id,
                        success: true,
                        output: Some(r.output),
                        error: None,
                    }),
                    Err(e) => results.push(BatchResult {
                        tool: tool_id,
                        success: false,
                        output: None,
                        error: Some(e.to_string()),
                    }),
                },
                Err(e) => results.push(BatchResult {
                    tool: tool_id,
                    success: false,
                    output: None,
                    error: Some(format!("Task join error: {}", e)),
                }),
            }
        }
        
        let successful = results.iter().filter(|r| r.success).count();
        let failed = results.len() - successful;
        
        let mut output = format!("Batch completed: {} succeeded, {} failed\n\n", successful, failed);
        
        for result in &results {
            output.push_str(&format!("--- {} ---\n", result.tool));
            if result.success {
                if let Some(ref out) = result.output {
                    output.push_str(out);
                    output.push('\n');
                }
            } else if let Some(ref err) = result.error {
                output.push_str(&format!("Error: {}\n", err));
            }
            output.push('\n');
        }
        
        Ok(ToolResult {
            title: format!("batch ({}/{})", successful, successful + failed),
            output,
            metadata: serde_json::json!({
                "total": results.len(),
                "successful": successful,
                "failed": failed,
            }),
            attachments: vec![],
            truncated: false,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
struct BatchResult {
    tool: String,
    success: bool,
    output: Option<String>,
    error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_batch_params_deserialize() {
        let json = serde_json::json!({
            "calls": [
                {"tool": "read", "args": {"filePath": "/tmp/test"}}
            ]
        });
        let params: BatchParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.calls.len(), 1);
        assert_eq!(params.calls[0].tool, "read");
    }

    #[test]
    fn test_batch_params_deserialize_multiple() {
        let json = serde_json::json!({
            "calls": [
                {"tool": "read", "args": {}},
                {"tool": "write", "args": {}}
            ]
        });
        let params: BatchParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.calls.len(), 2);
    }

    #[test]
    fn test_batch_call_deserialize() {
        let json = serde_json::json!({
            "tool": "bash",
            "args": {"command": "ls", "description": "list"}
        });
        let call: BatchCall = serde_json::from_value(json).unwrap();
        assert_eq!(call.tool, "bash");
    }

    #[test]
    fn test_batch_params_schema() {
        let registry = Arc::new(super::super::registry::ToolRegistry::new());
        let tool = BatchTool::new(registry);
        let schema = tool.parameters_schema();
        assert!(schema.is_object());
    }

    #[test]
    fn test_batch_tool_id() {
        let registry = Arc::new(super::super::registry::ToolRegistry::new());
        let tool = BatchTool::new(registry);
        assert_eq!(tool.id(), "batch");
    }

    #[test]
    fn test_batch_tool_description() {
        let registry = Arc::new(super::super::registry::ToolRegistry::new());
        let tool = BatchTool::new(registry);
        assert!(!tool.description().is_empty());
    }
}