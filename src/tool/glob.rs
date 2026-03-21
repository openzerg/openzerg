use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use crate::error::{Error, Result};
use super::traits::{Tool, ToolContext, ToolResult};
use super::schema::generate_schema;
use super::truncation::MAX_BYTES;

const DESCRIPTION: &str = include_str!("prompts/glob.md");

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GlobParams {
    #[schemars(description = "The glob pattern to match files against (e.g., \"**/*.rs\")")]
    pub pattern: String,
    #[schemars(description = "The directory to search in. Defaults to workspace.")]
    pub path: Option<String>,
}

pub struct GlobTool;

impl GlobTool {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl Tool for GlobTool {
    fn id(&self) -> &str { "glob" }
    
    fn description(&self) -> &str {
        DESCRIPTION
    }
    
    fn parameters_schema(&self) -> Value {
        generate_schema::<GlobParams>()
    }
    
    async fn execute(&self, args: Value, ctx: ToolContext) -> Result<ToolResult> {
        let params: GlobParams = serde_json::from_value(args)
            .map_err(|e| Error::Tool(format!("Invalid parameters: {}", e)))?;
        
        let base_path = params.path
            .map(|p| ctx.workspace.join(p))
            .unwrap_or_else(|| ctx.workspace.clone());
        
        if !base_path.starts_with(&ctx.workspace) {
            return Err(Error::Tool("Path must be within workspace".into()));
        }
        
        let pattern = base_path.join(&params.pattern);
        let pattern_str = pattern.display().to_string();
        
        let mut matches: Vec<(String, std::time::SystemTime)> = Vec::new();
        
        let glob_pattern = glob::Pattern::new(&pattern_str)
            .map_err(|e| Error::Tool(format!("Invalid glob pattern: {}", e)))?;
        
        for entry in walkdir::WalkDir::new(&base_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if glob_pattern.matches_path(path) {
                let mtime = entry.metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                
                let relative = path.strip_prefix(&ctx.workspace)
                    .unwrap_or(path)
                    .display()
                    .to_string();
                
                matches.push((relative, mtime));
            }
        }
        
        matches.sort_by(|a, b| b.1.cmp(&a.1));
        
        let results: Vec<String> = matches.into_iter().take(100).map(|(p, _)| p).collect();
        let total = results.len();
        
        let output = if results.is_empty() {
            format!("No files matching pattern: {}", params.pattern)
        } else {
            format!("Found {} files:\n{}", total, results.join("\n"))
        };
        
        Ok(ToolResult {
            title: format!("glob {}", params.pattern),
            output,
            metadata: serde_json::json!({
                "pattern": params.pattern,
                "count": total,
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
    fn test_glob_tool_id() {
        let tool = GlobTool::new();
        assert_eq!(tool.id(), "glob");
    }

    #[test]
    fn test_glob_tool_description() {
        let tool = GlobTool::new();
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_glob_params_deserialize() {
        let json = serde_json::json!({
            "pattern": "**/*.rs"
        });
        let params: GlobParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.pattern, "**/*.rs");
        assert!(params.path.is_none());
    }

    #[test]
    fn test_glob_params_deserialize_with_path() {
        let json = serde_json::json!({
            "pattern": "*.txt",
            "path": "/tmp"
        });
        let params: GlobParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.pattern, "*.txt");
        assert_eq!(params.path, Some("/tmp".to_string()));
    }

    #[test]
    fn test_glob_params_schema() {
        let tool = GlobTool::new();
        let schema = tool.parameters_schema();
        assert!(schema.is_object());
    }
}