use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use crate::error::{Error, Result};
use super::traits::{Tool, ToolContext, ToolResult};
use super::schema::generate_schema;

const DESCRIPTION: &str = include_str!("prompts/write.md");

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WriteParams {
    #[schemars(description = "The content to write to the file")]
    pub content: String,
    #[schemars(description = "The absolute path to the file to write (must be absolute, not relative)")]
    pub filePath: String,
}

pub struct WriteTool;

impl WriteTool {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl Tool for WriteTool {
    fn id(&self) -> &str { "write" }
    
    fn description(&self) -> &str {
        DESCRIPTION
    }
    
    fn parameters_schema(&self) -> Value {
        generate_schema::<WriteParams>()
    }
    
    async fn execute(&self, args: Value, ctx: ToolContext) -> Result<ToolResult> {
        let params: WriteParams = serde_json::from_value(args)
            .map_err(|e| Error::Tool(format!("Invalid parameters: {}", e)))?;
        
        let path = if std::path::Path::new(&params.filePath).is_absolute() {
            PathBuf::from(&params.filePath)
        } else {
            ctx.workspace.join(&params.filePath)
        };
        
        if !path.starts_with(&ctx.workspace) {
            return Err(Error::Tool("Path must be within workspace".into()));
        }
        
        let exists = path.exists();
        let old_content = if exists {
            Some(tokio::fs::read_to_string(&path).await
                .map_err(|e| Error::File(format!("Failed to read existing file: {}", e)))?)
        } else {
            None
        };
        
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| Error::File(format!("Failed to create directory: {}", e)))?;
        }
        
        tokio::fs::write(&path, &params.content).await
            .map_err(|e| Error::File(format!("Failed to write file: {}", e)))?;
        
        let diff = if let Some(old) = old_content {
            create_diff(&path.display().to_string(), &old, &params.content)
        } else {
            format!("Created new file: {}", path.display())
        };
        
        Ok(ToolResult {
            title: path.display().to_string(),
            output: "Wrote file successfully.".to_string(),
            metadata: serde_json::json!({
                "filepath": path.display().to_string(),
                "exists": exists,
                "diff": diff,
            }),
            attachments: vec![],
            truncated: false,
        })
    }
}

fn create_diff(filename: &str, old: &str, new: &str) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    
    let mut result = format!("--- {}\n+++ {}\n", filename, filename);
    
    let mut old_idx = 0;
    let mut new_idx = 0;
    
    while old_idx < old_lines.len() || new_idx < new_lines.len() {
        if old_idx >= old_lines.len() {
            result.push_str(&format!("+{}\n", new_lines[new_idx]));
            new_idx += 1;
        } else if new_idx >= new_lines.len() {
            result.push_str(&format!("-{}\n", old_lines[old_idx]));
            old_idx += 1;
        } else if old_lines[old_idx] == new_lines[new_idx] {
            result.push_str(&format!(" {}\n", old_lines[old_idx]));
            old_idx += 1;
            new_idx += 1;
        } else {
            result.push_str(&format!("-{}\n", old_lines[old_idx]));
            result.push_str(&format!("+{}\n", new_lines[new_idx]));
            old_idx += 1;
            new_idx += 1;
        }
    }
    
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_diff_addition() {
        let old = "line1\nline2";
        let new = "line1\nline2\nline3";
        let diff = create_diff("test.txt", old, new);
        assert!(diff.contains("+++ test.txt"));
        assert!(diff.contains("+line3"));
    }

    #[test]
    fn test_create_diff_removal() {
        let old = "line1\nline2\nline3";
        let new = "line1\nline2";
        let diff = create_diff("test.txt", old, new);
        assert!(diff.contains("--- test.txt"));
        assert!(diff.contains("-line3"));
    }

    #[test]
    fn test_create_diff_modification() {
        let old = "hello world";
        let new = "hello rust";
        let diff = create_diff("test.txt", old, new);
        assert!(diff.contains("-hello world"));
        assert!(diff.contains("+hello rust"));
    }

    #[test]
    fn test_create_diff_identical() {
        let content = "same\ncontent";
        let diff = create_diff("test.txt", content, content);
        assert!(diff.contains(" same"));
        assert!(!diff.contains("-same"));
        assert!(!diff.contains("+same"));
    }

    #[test]
    fn test_create_diff_empty_old() {
        let new = "new content";
        let diff = create_diff("test.txt", "", new);
        assert!(diff.contains("+new content"));
    }

    #[test]
    fn test_create_diff_empty_new() {
        let old = "old content";
        let diff = create_diff("test.txt", old, "");
        assert!(diff.contains("-old content"));
    }
}