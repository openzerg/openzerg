use std::sync::Arc;
use tokio::sync::broadcast;
use crate::error::Result;
use crate::config::Config;
use crate::storage::Storage;
use crate::tool::{ToolRegistry, ToolExecutor};
use crate::process::{ProcessManager, SystemdExecutor};
use crate::vision::VisionClient;
use crate::session::SessionProcessor;

const SOUL_TEMPLATE: &str = include_str!("prompts/soul.md");
const AGENT_TEMPLATE: &str = include_str!("prompts/agent.md");

pub struct AgentCore {
    pub config: Config,
    pub storage: Arc<Storage>,
    pub session_manager: Arc<crate::session::SessionManager>,
    pub task_manager: Arc<crate::task::TaskManager>,
    pub process_manager: Arc<ProcessManager>,
    pub file_manager: Arc<crate::file::FileManager>,
    pub llm_client: Arc<crate::llm::LLMClient>,
    pub thinking_layer: Arc<crate::thinking::ThinkingLayer>,
    pub tool_registry: Arc<ToolRegistry>,
    pub tool_executor: Arc<ToolExecutor>,
    pub session_processor: Arc<SessionProcessor>,
    pub systemd_executor: Arc<SystemdExecutor>,
    pub vision_client: Option<Arc<VisionClient>>,
    pub event_tx: broadcast::Sender<crate::protocol::AgentEvent>,
}

impl AgentCore {
    pub fn new(config: Config) -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        
        let openzerg_dir = config.openzerg_dir();
        let workspace_path = config.workspace_path();
        
        let storage = Arc::new(Storage::new(openzerg_dir.clone()));
        
        let session_manager = Arc::new(crate::session::SessionManager::new());
        let task_manager = Arc::new(crate::task::TaskManager::new());
        let process_manager = Arc::new(ProcessManager::new(
            openzerg_dir.join("process_outputs"),
            event_tx.clone(),
        ));
        let file_manager = Arc::new(crate::file::FileManager::new(workspace_path.clone()));
        let llm_client = Arc::new(crate::llm::LLMClient::new(
            config.llm_base_url().to_string(),
            config.llm_api_key().to_string(),
            config.llm_model().to_string(),
        ));
        let thinking_layer = Arc::new(crate::thinking::ThinkingLayer::new(
            llm_client.clone(),
            session_manager.clone(),
            task_manager.clone(),
        ));
        
        let systemd_executor = Arc::new(SystemdExecutor::new(
            openzerg_dir.join("process_outputs")
        ));
        
        let vision_client = if config.vision_enabled() {
            let vision = &config.runtime.vision;
            Some(Arc::new(VisionClient::new(
                vision.base_url.clone().unwrap_or_else(|| config.llm_base_url().to_string()),
                vision.api_key.clone().unwrap(),
                vision.model.clone().unwrap_or_else(|| "gpt-4o".to_string()),
            )))
        } else {
            None
        };
        
        let tool_registry = Arc::new(ToolRegistry::new());
        
        let tool_executor = Arc::new(ToolExecutor::new(
            tool_registry.clone(),
            workspace_path.clone(),
            openzerg_dir.clone(),
        ));
        
        let session_processor = Arc::new(SessionProcessor::new(
            llm_client.clone(),
            tool_executor.clone(),
            storage.clone(),
        ));

        Self {
            config,
            storage,
            session_manager,
            task_manager,
            process_manager,
            file_manager,
            llm_client,
            thinking_layer,
            tool_registry,
            tool_executor,
            session_processor,
            systemd_executor,
            vision_client,
            event_tx,
        }
    }

    pub async fn init(&self) -> Result<()> {
        self.storage.init().await?;
        
        let fixed = self.storage.fix_session_states().await.unwrap_or(0);
        if fixed > 0 {
            tracing::info!("Fixed {} stale session states", fixed);
        }
        
        self.session_manager.init_main().await;
        self.ensure_workspace_structure().await?;
        self.register_tools().await;
        self.systemd_executor.ensure_slice().await?;
        
        tracing::info!("Agent core initialized");
        Ok(())
    }
    
    async fn register_tools(&self) {
        self.tool_registry.register(Box::new(crate::tool::ReadTool::new())).await;
        self.tool_registry.register(Box::new(crate::tool::WriteTool::new())).await;
        self.tool_registry.register(Box::new(crate::tool::EditTool::new())).await;
        self.tool_registry.register(Box::new(crate::tool::BashTool::new(self.systemd_executor.clone()))).await;
        self.tool_registry.register(Box::new(crate::tool::GlobTool::new())).await;
        self.tool_registry.register(Box::new(crate::tool::GrepTool::new())).await;
        self.tool_registry.register(Box::new(crate::tool::LsTool::new())).await;
        self.tool_registry.register(Box::new(crate::tool::WebFetchTool::new())).await;
        
        tracing::info!("Tools registered: {:?}", self.tool_registry.tool_definitions().await.iter().map(|d| &d.name).collect::<Vec<_>>());
    }

    async fn ensure_workspace_structure(&self) -> Result<()> {
        let workspace = self.config.workspace_path();
        
        tokio::fs::create_dir_all(workspace.join("MEMORY")).await?;
        tokio::fs::create_dir_all(workspace.join("projects")).await?;
        tokio::fs::create_dir_all(workspace.join(".openzerg/process_outputs")).await?;
        
        let soul_path = workspace.join("SOUL.md");
        if !soul_path.exists() {
            let content = SOUL_TEMPLATE
                .replace("{agent_name}", &self.config.agent_name)
                .replace("{created}", &chrono::Utc::now().to_rfc3339());
            tokio::fs::write(&soul_path, content).await?;
        }

        let agent_path = workspace.join("AGENT.md");
        if !agent_path.exists() {
            let content = AGENT_TEMPLATE.replace("{timestamp}", &chrono::Utc::now().to_rfc3339());
            tokio::fs::write(&agent_path, content).await?;
        }

        Ok(())
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<crate::protocol::AgentEvent> {
        self.event_tx.subscribe()
    }
}