use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_util::sync::CancellationToken;
use crate::error::Result;
use crate::file::FileManager;
use crate::process::ProcessManager;

#[async_trait]
pub trait Tool: Send + Sync {
    fn id(&self) -> &str;
    
    fn description(&self) -> &str;
    
    fn parameters_schema(&self) -> Value;
    
    async fn execute(&self, args: Value, ctx: ToolContext) -> Result<ToolResult>;
}

#[derive(Clone)]
pub struct ToolContext {
    pub session_id: String,
    pub message_id: String,
    pub workspace: PathBuf,
    pub openzerg_dir: PathBuf,
    pub abort: CancellationToken,
    pub file_manager: Arc<FileManager>,
    pub process_manager: Arc<ProcessManager>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub title: String,
    pub output: String,
    pub metadata: Value,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
    #[serde(default)]
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub mime: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

impl Attachment {
    pub fn image(mime: &str, data: &[u8]) -> Self {
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data);
        Self {
            mime: mime.to_string(),
            url: format!("data:{};base64,{}", mime, encoded),
        }
    }
    
    pub fn from_file(path: &std::path::Path) -> Option<Self> {
        let mime = mime_guess::from_path(path)
            .first()
            .map(|m| m.to_string())?;
        
        let data = std::fs::read(path).ok()?;
        Some(Self::image(&mime, &data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attachment_image() {
        let data = vec![1, 2, 3, 4];
        let attachment = Attachment::image("image/png", &data);
        assert_eq!(attachment.mime, "image/png");
        assert!(attachment.url.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn test_attachment_serialization() {
        let attachment = Attachment {
            mime: "text/plain".to_string(),
            url: "data:text/plain;base64,test".to_string(),
        };
        let json = serde_json::to_string(&attachment).unwrap();
        assert!(json.contains("text/plain"));
    }

    #[test]
    fn test_tool_result_serialization() {
        let result = ToolResult {
            title: "test".to_string(),
            output: "output".to_string(),
            metadata: serde_json::json!({"key": "value"}),
            attachments: vec![],
            truncated: false,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("output"));
    }

    #[test]
    fn test_tool_result_deserialization() {
        let json = r#"{"title":"test","output":"out","metadata":{},"attachments":[],"truncated":false}"#;
        let result: ToolResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.title, "test");
        assert_eq!(result.output, "out");
    }

    #[test]
    fn test_tool_definition_serialization() {
        let def = ToolDefinition {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            parameters: serde_json::json!({}),
        };
        let json = serde_json::to_string(&def).unwrap();
        assert!(json.contains("test_tool"));
    }

    #[test]
    fn test_tool_context_clone() {
        let ctx = ToolContext {
            session_id: "s1".to_string(),
            message_id: "m1".to_string(),
            workspace: PathBuf::from("/tmp"),
            openzerg_dir: PathBuf::from("/tmp/.oz"),
            abort: CancellationToken::new(),
            file_manager: Arc::new(FileManager::new(PathBuf::from("/tmp"))),
            process_manager: Arc::new(ProcessManager::new(
                PathBuf::from("/tmp"),
                tokio::sync::broadcast::channel(1).0,
            )),
        };
        let cloned = ctx.clone();
        assert_eq!(cloned.session_id, "s1");
    }
}