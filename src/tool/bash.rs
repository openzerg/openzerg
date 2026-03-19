use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use crate::error::{Error, Result};
use super::traits::{Tool, ToolContext, ToolResult};
use super::schema::generate_schema;
use super::truncation::{truncate_output, MAX_BYTES};

const DEFAULT_TIMEOUT_MS: u64 = 120_000;
const DESCRIPTION: &str = include_str!("prompts/bash.txt");

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BashParams {
    #[schemars(description = "The command to execute")]
    pub command: String,
    #[schemars(description = "Optional timeout in milliseconds (default 120000)")]
    pub timeout: Option<u64>,
    #[schemars(description = "The working directory to run the command in. Defaults to workspace.")]
    pub workdir: Option<String>,
    #[schemars(description = "Clear, concise description of what this command does in 5-10 words")]
    pub description: String,
}

pub struct BashTool {
    executor: Arc<crate::process::systemd_executor::SystemdExecutor>,
}

impl BashTool {
    pub fn new(executor: Arc<crate::process::systemd_executor::SystemdExecutor>) -> Self {
        Self { executor }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn id(&self) -> &str { "bash" }
    
    fn description(&self) -> &str {
        DESCRIPTION
    }
    
    fn parameters_schema(&self) -> Value {
        generate_schema::<BashParams>()
    }
    
    async fn execute(&self, args: Value, ctx: ToolContext) -> Result<ToolResult> {
        let params: BashParams = serde_json::from_value(args)
            .map_err(|e| Error::Tool(format!("Invalid parameters: {}", e)))?;
        
        if params.timeout.map(|t| t < 0).unwrap_or(false) {
            return Err(Error::Tool("Timeout must be a positive number".into()));
        }
        
        let workdir = params.workdir
            .map(|p| ctx.workspace.join(p))
            .unwrap_or_else(|| ctx.workspace.clone());
        
        if !workdir.starts_with(&ctx.workspace) && workdir != ctx.workspace {
            return Err(Error::Tool("Workdir must be within workspace".into()));
        }
        
        let timeout = params.timeout.unwrap_or(DEFAULT_TIMEOUT_MS);
        
        let env: std::collections::HashMap<String, String> = std::env::vars().collect();
        
        let process = self.executor.execute(
            &params.command,
            &workdir,
            &env,
            timeout,
            &ctx.session_id,
        ).await?;
        
        let stdout = tokio::fs::read_to_string(&process.stdout_path).await
            .unwrap_or_default();
        let stderr = tokio::fs::read_to_string(&process.stderr_path).await
            .unwrap_or_default();
        
        let mut output = stdout;
        if !stderr.is_empty() {
            if !output.is_empty() {
                output.push_str("\n");
            }
            output.push_str(&stderr);
        }
        
        let mut metadata = vec![];
        if process.status == crate::process::ProcessStatus::Timeout {
            metadata.push(format!("Command terminated after exceeding timeout {} ms", timeout));
        }
        
        if ctx.abort.is_cancelled() {
            metadata.push("Command was aborted".to_string());
        }
        
        if !metadata.is_empty() {
            output.push_str("\n\n<bash_metadata>\n");
            output.push_str(&metadata.join("\n"));
            output.push_str("\n</bash_metadata>");
        }
        
        let (output, truncated) = truncate_output(&output, MAX_BYTES);
        
        Ok(ToolResult {
            title: params.description.clone(),
            output,
            metadata: serde_json::json!({
                "exit_code": process.exit_code,
                "process_id": process.id,
                "stdout_size": process.stdout_size,
                "stderr_size": process.stderr_size,
                "description": params.description,
            }),
            attachments: vec![],
            truncated,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bash_tool_id() {
        let executor = Arc::new(crate::process::SystemdExecutor::new(std::path::PathBuf::from("/tmp")));
        let tool = BashTool::new(executor);
        assert_eq!(tool.id(), "bash");
    }

    #[test]
    fn test_bash_tool_description() {
        let executor = Arc::new(crate::process::SystemdExecutor::new(std::path::PathBuf::from("/tmp")));
        let tool = BashTool::new(executor);
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_bash_params_deserialize() {
        let json = serde_json::json!({
            "command": "ls -la",
            "description": "List files"
        });
        let params: BashParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.command, "ls -la");
        assert_eq!(params.description, "List files");
        assert!(params.timeout.is_none());
        assert!(params.workdir.is_none());
    }

    #[test]
    fn test_bash_params_deserialize_with_options() {
        let json = serde_json::json!({
            "command": "echo test",
            "description": "Print test",
            "timeout": 30000,
            "workdir": "/tmp"
        });
        let params: BashParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.command, "echo test");
        assert_eq!(params.description, "Print test");
        assert_eq!(params.timeout, Some(30000));
        assert_eq!(params.workdir, Some("/tmp".to_string()));
    }

    #[test]
    fn test_bash_params_schema() {
        let executor = Arc::new(crate::process::SystemdExecutor::new(std::path::PathBuf::from("/tmp")));
        let tool = BashTool::new(executor);
        let schema = tool.parameters_schema();
        assert!(schema.is_object());
    }
}