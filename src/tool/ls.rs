use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use crate::error::{Error, Result};
use super::traits::{Tool, ToolContext, ToolResult};
use super::schema::generate_schema;

const DESCRIPTION: &str = include_str!("prompts/ls.md");

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LsParams {
    #[schemars(description = "The directory path to list. Defaults to workspace.")]
    pub path: Option<String>,
}

pub struct LsTool;

impl LsTool {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl Tool for LsTool {
    fn id(&self) -> &str { "ls" }
    
    fn description(&self) -> &str {
        DESCRIPTION
    }
    
    fn parameters_schema(&self) -> Value {
        generate_schema::<LsParams>()
    }
    
    async fn execute(&self, args: Value, ctx: ToolContext) -> Result<ToolResult> {
        let params: LsParams = serde_json::from_value(args)
            .map_err(|e| Error::Tool(format!("Invalid parameters: {}", e)))?;
        
        let path = params.path
            .map(|p| ctx.workspace.join(p))
            .unwrap_or_else(|| ctx.workspace.clone());
        
        if !path.starts_with(&ctx.workspace) {
            return Err(Error::Tool("Path must be within workspace".into()));
        }
        
        if !path.exists() {
            return Err(Error::File(format!("Directory not found: {}", path.display())));
        }
        
        if !path.is_dir() {
            return Err(Error::File(format!("Path is not a directory: {}", path.display())));
        }
        
        let mut entries = Vec::new();
        
        let mut dir = tokio::fs::read_dir(&path).await
            .map_err(|e| Error::File(format!("Failed to read directory: {}", e)))?;
        
        while let Some(entry) = dir.next_entry().await.map_err(|e| Error::File(format!("Failed to read entry: {}", e)))? {
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = entry.file_type().await
                .map(|t| t.is_dir())
                .unwrap_or(false);
            
            entries.push(if is_dir { format!("{}/", name) } else { name });
        }
        
        entries.sort_by(|a, b| {
            let a_is_dir = a.ends_with('/');
            let b_is_dir = b.ends_with('/');
            b_is_dir.cmp(&a_is_dir).then_with(|| a.cmp(b))
        });
        
        let output = format!("{}\n{} entries", entries.join("\n"), entries.len());
        
        Ok(ToolResult {
            title: path.display().to_string(),
            output,
            metadata: serde_json::json!({
                "path": path.display().to_string(),
                "count": entries.len(),
            }),
            attachments: vec![],
            truncated: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ls_tool_id() {
        let tool = LsTool::new();
        assert_eq!(tool.id(), "ls");
    }

    #[test]
    fn test_ls_tool_description() {
        let tool = LsTool::new();
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_ls_params_deserialize() {
        let json = serde_json::json!({});
        let params: LsParams = serde_json::from_value(json).unwrap();
        assert!(params.path.is_none());
    }

    #[test]
    fn test_ls_params_deserialize_with_path() {
        let json = serde_json::json!({
            "path": "/tmp"
        });
        let params: LsParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.path, Some("/tmp".to_string()));
    }

    #[test]
    fn test_ls_params_schema() {
        let tool = LsTool::new();
        let schema = tool.parameters_schema();
        assert!(schema.is_object());
    }
}