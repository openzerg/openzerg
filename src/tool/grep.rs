use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use crate::error::{Error, Result};
use super::traits::{Tool, ToolContext, ToolResult};
use super::schema::generate_schema;
use super::truncation::{truncate_output, MAX_BYTES};

const DESCRIPTION: &str = include_str!("prompts/grep.txt");

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GrepParams {
    #[schemars(description = "The regex pattern to search for in file contents")]
    pub pattern: String,
    #[schemars(description = "The directory to search in. Defaults to workspace.")]
    pub path: Option<String>,
    #[schemars(description = "File pattern to include in the search (e.g., \"*.js\")")]
    pub include: Option<String>,
}

pub struct GrepTool;

impl GrepTool {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl Tool for GrepTool {
    fn id(&self) -> &str { "grep" }
    
    fn description(&self) -> &str {
        DESCRIPTION
    }
    
    fn parameters_schema(&self) -> Value {
        generate_schema::<GrepParams>()
    }
    
    async fn execute(&self, args: Value, ctx: ToolContext) -> Result<ToolResult> {
        let params: GrepParams = serde_json::from_value(args)
            .map_err(|e| Error::Tool(format!("Invalid parameters: {}", e)))?;
        
        let base_path = params.path
            .map(|p| ctx.workspace.join(p))
            .unwrap_or_else(|| ctx.workspace.clone());
        
        if !base_path.starts_with(&ctx.workspace) {
            return Err(Error::Tool("Path must be within workspace".into()));
        }
        
        let regex = regex::Regex::new(&params.pattern)
            .map_err(|e| Error::Tool(format!("Invalid regex pattern: {}", e)))?;
        
        let include_pattern: Option<glob::Pattern> = params.include
            .as_ref()
            .and_then(|p| glob::Pattern::new(p).ok());
        
        let mut matches: Vec<(String, usize, String)> = Vec::new();
        
        for entry in walkdir::WalkDir::new(&base_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            
            if let Some(ref pattern) = include_pattern {
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                if !pattern.matches(file_name) {
                    continue;
                }
            }
            
            let content = match tokio::fs::read_to_string(path).await {
                Ok(c) => c,
                Err(_) => continue,
            };
            
            for (line_num, line) in content.lines().enumerate() {
                if regex.is_match(line) {
                    let relative = path.strip_prefix(&ctx.workspace)
                        .unwrap_or(path)
                        .display()
                        .to_string();
                    
                    matches.push((relative, line_num + 1, line.to_string()));
                    
                    if matches.len() >= 1000 {
                        break;
                    }
                }
            }
            
            if matches.len() >= 1000 {
                break;
            }
        }
        
        let total = matches.len();
        let output = if matches.is_empty() {
            format!("No matches found for pattern: {}", params.pattern)
        } else {
            let mut result = format!("Found {} matches:\n", total);
            for (file, line, content) in matches.iter().take(100) {
                result.push_str(&format!("{}:{}: {}\n", file, line, content));
            }
            if total > 100 {
                result.push_str(&format!("\n... and {} more matches", total - 100));
            }
            result
        };
        
        let (output, truncated) = truncate_output(&output, MAX_BYTES);
        
        Ok(ToolResult {
            title: format!("grep {}", params.pattern),
            output,
            metadata: serde_json::json!({
                "pattern": params.pattern,
                "count": total,
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
    fn test_grep_tool_id() {
        let tool = GrepTool::new();
        assert_eq!(tool.id(), "grep");
    }

    #[test]
    fn test_grep_tool_description() {
        let tool = GrepTool::new();
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn test_grep_params_deserialize() {
        let json = serde_json::json!({
            "pattern": "fn\\s+\\w+"
        });
        let params: GrepParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.pattern, "fn\\s+\\w+");
        assert!(params.path.is_none());
        assert!(params.include.is_none());
    }

    #[test]
    fn test_grep_params_deserialize_with_options() {
        let json = serde_json::json!({
            "pattern": "TODO",
            "path": "src",
            "include": "*.rs"
        });
        let params: GrepParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.pattern, "TODO");
        assert_eq!(params.path, Some("src".to_string()));
        assert_eq!(params.include, Some("*.rs".to_string()));
    }

    #[test]
    fn test_grep_params_schema() {
        let tool = GrepTool::new();
        let schema = tool.parameters_schema();
        assert!(schema.is_object());
    }
}