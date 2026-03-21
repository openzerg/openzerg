use async_trait::async_trait;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;
use crate::error::{Error, Result};
use super::super::traits::{Tool, ToolContext, ToolResult};
use super::super::schema::generate_schema;
use super::replacer::get_replacers;
use super::diff::create_diff;

const DESCRIPTION: &str = include_str!("../prompts/edit.md");

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EditParams {
    #[schemars(description = "The absolute path to the file to modify")]
    pub filePath: String,
    #[schemars(description = "The text to replace")]
    pub oldString: String,
    #[schemars(description = "The text to replace it with (must be different from oldString)")]
    pub newString: String,
    #[schemars(description = "Replace all occurrences of oldString (default false)")]
    pub replaceAll: Option<bool>,
}

pub struct EditTool;

impl EditTool {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl Tool for EditTool {
    fn id(&self) -> &str { "edit" }
    
    fn description(&self) -> &str {
        DESCRIPTION
    }
    
    fn parameters_schema(&self) -> Value {
        generate_schema::<EditParams>()
    }
    
    async fn execute(&self, args: Value, ctx: ToolContext) -> Result<ToolResult> {
        let params: EditParams = serde_json::from_value(args)
            .map_err(|e| Error::Tool(format!("Invalid parameters: {}", e)))?;
        
        if params.oldString == params.newString {
            return Err(Error::Tool("oldString and newString are identical".into()));
        }
        
        let path = if std::path::Path::new(&params.filePath).is_absolute() {
            PathBuf::from(&params.filePath)
        } else {
            ctx.workspace.join(&params.filePath)
        };
        
        if !path.starts_with(&ctx.workspace) {
            return Err(Error::Tool("Path must be within workspace".into()));
        }
        
        if !path.exists() {
            return Err(Error::File(format!("File not found: {}", path.display())));
        }
        
        if path.is_dir() {
            return Err(Error::File(format!("Path is a directory: {}", path.display())));
        }
        
        let old_content = tokio::fs::read_to_string(&path).await
            .map_err(|e| Error::File(format!("Failed to read file: {}", e)))?;
        
        let new_content = replace(
            &old_content,
            &params.oldString,
            &params.newString,
            params.replaceAll.unwrap_or(false),
        )?;
        
        let diff = create_diff(&path.display().to_string(), &old_content, &new_content);
        
        tokio::fs::write(&path, &new_content).await
            .map_err(|e| Error::File(format!("Failed to write file: {}", e)))?;
        
        Ok(ToolResult {
            title: path.display().to_string(),
            output: "Edit applied successfully.".to_string(),
            metadata: serde_json::json!({ "diff": diff }),
            attachments: vec![],
            truncated: false,
        })
    }
}

fn replace(content: &str, old: &str, new: &str, replace_all: bool) -> Result<String> {
    let mut matches_found = false;
    
    for replacer in get_replacers() {
        let matches = replacer.find(content, old);
        
        if matches.is_empty() {
            continue;
        }
        
        matches_found = true;
        
        if matches.len() == 1 || replace_all {
            let mut result = content.to_string();
            if replace_all {
                for m in &matches {
                    result = result.replace(m, new);
                }
            } else {
                if let Some(pos) = result.find(&matches[0]) {
                    result = format!("{}{}{}", &result[..pos], new, &result[pos + matches[0].len()..]);
                }
            }
            return Ok(result);
        }
        
        if matches.len() > 1 && !replace_all {
            return Err(Error::Tool(
                "Found multiple matches. Provide more context to make unique or use replaceAll=true.".into()
            ));
        }
    }
    
    if !matches_found {
        Err(Error::Tool(
            "Could not find oldString in the file. It must match exactly, including whitespace, indentation, and line endings.".into()
        ))
    } else {
        Err(Error::Tool("Unexpected error in replace logic".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_simple() {
        let content = "hello world";
        let result = replace(content, "world", "rust", false).unwrap();
        assert_eq!(result, "hello rust");
    }

    #[test]
    fn test_replace_multiple_with_replaceall() {
        let content = "hello hello hello";
        let result = replace(content, "hello", "hi", true).unwrap();
        assert_eq!(result, "hi hi hi");
    }

    #[test]
    fn test_replace_not_found() {
        let content = "hello world";
        let result = replace(content, "foo", "bar", false);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_replace_first_occurrence() {
        let content = "hello hello hello";
        let result = replace(content, "hello", "hi", false).unwrap();
        assert_eq!(result, "hi hello hello");
    }
}