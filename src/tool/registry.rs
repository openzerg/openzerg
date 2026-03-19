use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::error::{Error, Result};
use super::traits::{Tool, ToolDefinition};

pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Box<dyn Tool>>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    pub async fn register(&self, tool: Box<dyn Tool>) {
        let mut tools = self.tools.write().await;
        let id = tool.id().to_string();
        tools.insert(id, tool);
    }
    
    pub async fn get(&self, id: &str) -> Option<Arc<dyn Tool>> {
        let tools = self.tools.read().await;
        tools.get(id).map(|t| {
            // We need to return a reference, but Box<dyn Tool> doesn't implement Clone
            // So we store tools differently or return a different type
            // For now, we'll restructure
            unimplemented!("Need to restructure for Arc<dyn Tool>")
        })
    }
    
    pub async fn has(&self, id: &str) -> bool {
        let tools = self.tools.read().await;
        tools.contains_key(id)
    }
    
    pub async fn tool_definitions(&self) -> Vec<ToolDefinition> {
        let tools = self.tools.read().await;
        tools.values().map(|t| ToolDefinition {
            name: t.id().to_string(),
            description: t.description().to_string(),
            parameters: t.parameters_schema(),
        }).collect()
    }
    
    pub async fn execute(&self, id: &str, args: serde_json::Value, ctx: super::traits::ToolContext) -> Result<super::traits::ToolResult> {
        let tools = self.tools.read().await;
        let tool = tools.get(id)
            .ok_or_else(|| Error::Tool(format!("Tool not found: {}", id)))?;
        tool.execute(args, ctx).await
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::traits::{Tool, ToolContext, ToolResult};
    use async_trait::async_trait;
    use serde_json::Value;
    use tokio_util::sync::CancellationToken;

    struct MockTool {
        id: &'static str,
        desc: &'static str,
    }

    #[async_trait]
    impl Tool for MockTool {
        fn id(&self) -> &str { self.id }
        fn description(&self) -> &str { self.desc }
        fn parameters_schema(&self) -> Value { serde_json::json!({}) }
        async fn execute(&self, _args: Value, _ctx: ToolContext) -> crate::error::Result<ToolResult> {
            Ok(ToolResult {
                title: "mock".to_string(),
                output: "mock output".to_string(),
                metadata: Value::Null,
                attachments: vec![],
                truncated: false,
            })
        }
    }

    #[tokio::test]
    async fn test_registry_new() {
        let registry = ToolRegistry::new();
        let defs = registry.tool_definitions().await;
        assert!(defs.is_empty());
    }

    #[tokio::test]
    async fn test_registry_register() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(MockTool { id: "test", desc: "Test tool" })).await;
        
        let defs = registry.tool_definitions().await;
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].name, "test");
        assert_eq!(defs[0].description, "Test tool");
    }

    #[tokio::test]
    async fn test_registry_has() {
        let registry = ToolRegistry::new();
        assert!(!registry.has("test").await);
        
        registry.register(Box::new(MockTool { id: "test", desc: "Test" })).await;
        assert!(registry.has("test").await);
        assert!(!registry.has("other").await);
    }

    #[tokio::test]
    async fn test_registry_tool_definitions() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(MockTool { id: "tool1", desc: "First" })).await;
        registry.register(Box::new(MockTool { id: "tool2", desc: "Second" })).await;
        
        let defs = registry.tool_definitions().await;
        assert_eq!(defs.len(), 2);
    }

    #[tokio::test]
    async fn test_registry_execute_not_found() {
        let registry = ToolRegistry::new();
        let ctx = ToolContext {
            session_id: "test".to_string(),
            message_id: "msg".to_string(),
            workspace: std::path::PathBuf::from("/tmp"),
            openzerg_dir: std::path::PathBuf::from("/tmp/.openzerg"),
            abort: CancellationToken::new(),
            file_manager: Arc::new(crate::file::FileManager::new(std::path::PathBuf::from("/tmp"))),
            process_manager: Arc::new(crate::process::ProcessManager::new(
                std::path::PathBuf::from("/tmp"),
                tokio::sync::broadcast::channel(1).0,
            )),
        };
        
        let result = registry.execute("nonexistent", serde_json::json!({}), ctx).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_registry_execute_success() {
        let registry = ToolRegistry::new();
        registry.register(Box::new(MockTool { id: "test", desc: "Test" })).await;
        
        let ctx = ToolContext {
            session_id: "test".to_string(),
            message_id: "msg".to_string(),
            workspace: std::path::PathBuf::from("/tmp"),
            openzerg_dir: std::path::PathBuf::from("/tmp/.openzerg"),
            abort: CancellationToken::new(),
            file_manager: Arc::new(crate::file::FileManager::new(std::path::PathBuf::from("/tmp"))),
            process_manager: Arc::new(crate::process::ProcessManager::new(
                std::path::PathBuf::from("/tmp"),
                tokio::sync::broadcast::channel(1).0,
            )),
        };
        
        let result = registry.execute("test", serde_json::json!({}), ctx).await;
        assert!(result.is_ok());
        let tool_result = result.unwrap();
        assert_eq!(tool_result.title, "mock");
    }

    #[test]
    fn test_registry_default() {
        let registry = ToolRegistry::default();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let defs = registry.tool_definitions().await;
            assert!(defs.is_empty());
        });
    }
}