use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "openzerg")]
#[command(about = "OpenZerg - Autonomous Agent Framework")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    #[command(about = "Start the agent and connect to manager")]
    Start {
        #[arg(short, long, env = "AGENT_NAME", default_value = "default")]
        name: String,
        #[arg(short = 'm', long, env = "MANAGER_URL", default_value = "ws://10.200.1.1:17531")]
        manager: String,
        #[arg(short = 'w', long, env = "WORKSPACE")]
        workspace: Option<PathBuf>,
        #[arg(short = 'p', long, env = "API_PORT", default_value = "8081")]
        api_port: u16,
        #[arg(short = 'g', long, env = "GRPC_PORT", default_value = "50051")]
        grpc_port: u16,
    },

    #[command(about = "Serve HTTP/gRPC API without manager connection")]
    Serve {
        #[arg(short, long, env = "AGENT_NAME", default_value = "standalone")]
        name: String,
        #[arg(short = 'w', long, env = "WORKSPACE")]
        workspace: Option<PathBuf>,
        #[arg(short = 'p', long, env = "API_PORT", default_value = "8081")]
        api_port: u16,
        #[arg(short = 'g', long, env = "GRPC_PORT", default_value = "50051")]
        grpc_port: u16,
    },

    #[command(about = "LLM Provider management")]
    Provider {
        #[command(subcommand)]
        command: ProviderCommands,
    },

    #[command(about = "Configuration management")]
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },

    #[command(about = "Tool management")]
    Tool {
        #[command(subcommand)]
        command: ToolCommands,
    },

    #[command(about = "Session management (requires running agent)")]
    Session {
        #[command(subcommand)]
        command: SessionCommands,
    },

    #[command(about = "Check agent status")]
    Status {
        #[arg(short, long, default_value = "http://localhost:8081")]
        url: String,
    },
}

#[derive(Subcommand)]
pub enum ProviderCommands {
    #[command(about = "List all providers")]
    List {
        #[arg(short, long)]
        workspace: Option<PathBuf>,
    },

    #[command(about = "Get provider details")]
    Get {
        #[arg(required = true)]
        provider: String,
        #[arg(short, long)]
        workspace: Option<PathBuf>,
    },

    #[command(about = "Create a new provider (models: glm-5, kimi-k2.5)")]
    Create {
        #[arg(short = 'n', long)]
        name: String,
        #[arg(short = 'u', long)]
        base_url: String,
        #[arg(short = 'k', long)]
        api_key: String,
        #[arg(short = 'm', long, value_parser = ["glm-5", "kimi-k2.5"])]
        model: String,
        #[arg(long)]
        max_tokens: Option<i32>,
        #[arg(long)]
        temperature: Option<f64>,
        #[arg(long)]
        top_p: Option<f64>,
        #[arg(long)]
        top_k: Option<i32>,
        #[arg(long)]
        extra_params: Option<String>,
        #[arg(short, long)]
        workspace: Option<PathBuf>,
    },

    #[command(about = "Update a provider")]
    Update {
        #[arg(required = true)]
        provider: String,
        #[arg(short = 'n', long)]
        name: Option<String>,
        #[arg(short = 'u', long)]
        base_url: Option<String>,
        #[arg(short = 'k', long)]
        api_key: Option<String>,
        #[arg(short = 'm', long, value_parser = ["glm-5", "kimi-k2.5"])]
        model: Option<String>,
        #[arg(long)]
        max_tokens: Option<i32>,
        #[arg(long)]
        temperature: Option<f64>,
        #[arg(long)]
        top_p: Option<f64>,
        #[arg(long)]
        top_k: Option<i32>,
        #[arg(long)]
        extra_params: Option<String>,
        #[arg(short, long)]
        workspace: Option<PathBuf>,
    },

    #[command(about = "Delete a provider")]
    Delete {
        #[arg(required = true)]
        provider: String,
        #[arg(short, long)]
        workspace: Option<PathBuf>,
    },

    #[command(about = "Set active provider")]
    Use {
        #[arg(required = true)]
        provider: String,
        #[arg(short, long)]
        workspace: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    #[command(about = "Show current configuration")]
    Show {
        #[arg(short, long)]
        workspace: Option<PathBuf>,
    },

    #[command(about = "Set LLM configuration")]
    SetLlm {
        #[arg(short = 'u', long)]
        base_url: Option<String>,
        #[arg(short = 'k', long)]
        api_key: Option<String>,
        #[arg(short = 'm', long)]
        model: Option<String>,
        #[arg(short, long)]
        workspace: Option<PathBuf>,
    },

    #[command(about = "Set Vision configuration")]
    SetVision {
        #[arg(short = 'u', long)]
        base_url: Option<String>,
        #[arg(short = 'k', long)]
        api_key: Option<String>,
        #[arg(short = 'm', long)]
        model: Option<String>,
        #[arg(short, long)]
        workspace: Option<PathBuf>,
    },

    #[command(about = "Set API port")]
    SetPort {
        #[arg(short, long)]
        port: u16,
        #[arg(short, long)]
        workspace: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
pub enum ToolCommands {
    #[command(about = "List all available tools")]
    List {
        #[arg(short, long, default_value = "http://localhost:8081")]
        url: String,
    },

    #[command(about = "Get tool schema")]
    Get {
        #[arg(short, long)]
        name: String,
        #[arg(short, long, default_value = "http://localhost:8081")]
        url: String,
    },
}

#[derive(Subcommand)]
pub enum SessionCommands {
    #[command(about = "List all sessions")]
    List {
        #[arg(short, long, default_value = "http://localhost:8081")]
        url: String,
    },

    #[command(about = "Get session details")]
    Get {
        #[arg(short, long)]
        id: String,
        #[arg(short, long, default_value = "http://localhost:8081")]
        url: String,
    },

    #[command(about = "Get session messages")]
    Messages {
        #[arg(short, long)]
        id: String,
        #[arg(short, long, default_value = "http://localhost:8081")]
        url: String,
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },

    #[command(about = "Delete a session")]
    Delete {
        #[arg(short, long)]
        id: String,
        #[arg(short, long, default_value = "http://localhost:8081")]
        url: String,
    },
}

pub async fn handle_command(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Start { name, manager, workspace, api_port, grpc_port } => {
            start_agent(name, manager, workspace, api_port, grpc_port).await?;
        }
        Commands::Serve { name, workspace, api_port, grpc_port } => {
            serve_agent(name, workspace, api_port, grpc_port).await?;
        }
        Commands::Provider { command } => {
            handle_provider_command(command).await?;
        }
        Commands::Config { command } => {
            handle_config_command(command).await?;
        }
        Commands::Tool { command } => {
            handle_tool_command(command).await?;
        }
        Commands::Session { command } => {
            handle_session_command(command).await?;
        }
        Commands::Status { url } => {
            check_status(&url).await?;
        }
    }
    Ok(())
}

async fn start_agent(
    name: String,
    manager: String,
    workspace: Option<PathBuf>,
    api_port: u16,
    grpc_port: u16,
) -> anyhow::Result<()> {
    if let Some(ws) = workspace {
        std::env::set_var("WORKSPACE", ws.to_string_lossy().to_string());
    }
    std::env::set_var("AGENT_NAME", name);
    std::env::set_var("MANAGER_URL", manager);
    std::env::set_var("API_PORT", api_port.to_string());
    std::env::set_var("GRPC_PORT", grpc_port.to_string());
    std::env::set_var("INTERNAL_TOKEN", "cli-started-token");
    
    run_agent_internal(true).await
}

async fn serve_agent(
    name: String,
    workspace: Option<PathBuf>,
    api_port: u16,
    grpc_port: u16,
) -> anyhow::Result<()> {
    if let Some(ws) = workspace {
        std::env::set_var("WORKSPACE", ws.to_string_lossy().to_string());
    }
    std::env::set_var("AGENT_NAME", name);
    std::env::set_var("API_PORT", api_port.to_string());
    std::env::set_var("GRPC_PORT", grpc_port.to_string());
    std::env::set_var("INTERNAL_TOKEN", "serve-token");
    
    run_agent_internal(false).await
}

async fn run_agent_internal(connect_manager: bool) -> anyhow::Result<()> {
    tracing::info!("Starting OpenZerg Agent v0.5.0...");

    let config = crate::config::Config::from_env()?;
    tracing::info!("Agent name: {}", config.agent_name);
    if connect_manager {
        tracing::info!("Manager URL: {}", config.manager_url);
    } else {
        tracing::info!("Running in standalone mode (no manager connection)");
    }
    tracing::info!("Workspace: {}", config.workspace);
    tracing::info!("LLM Base URL: {}", config.llm_base_url());
    tracing::info!("LLM Model: {}", config.llm_model());
    tracing::info!("API Port: {}", config.api_port());
    tracing::info!("gRPC Port: {}", config.grpc_port());
    tracing::info!("Vision enabled: {}", config.vision_enabled());

    std::fs::create_dir_all(&config.workspace)?;
    std::fs::create_dir_all(config.openzerg_dir())?;

    let core = std::sync::Arc::new(crate::agent::AgentCore::new(config.clone()));
    core.init().await?;

    let dispatcher = std::sync::Arc::new(crate::event::EventDispatcher::new(config.agent_name.clone(), core.event_tx.clone()));

    let core_clone = core.clone();
    let event_handle = core.subscribe_events();
    
    tokio::spawn(async move {
        let mut rx = event_handle;
        loop {
            match rx.recv().await {
                Ok(event) => {
                    core_clone.handle_event(event).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
            }
        }
    });

    let grpc_server = crate::grpc::AgentServiceServer::new(crate::grpc::AgentGrpcServer::new(
        core.session_manager.clone(),
        core.process_manager.clone(),
        core.tool_registry.clone(),
    ));
    
    let grpc_addr_str = format!("0.0.0.0:{}", config.grpc_port());
    let grpc_addr: std::net::SocketAddr = grpc_addr_str.parse()
        .map_err(|e| anyhow::anyhow!("Invalid gRPC address: {}", e))?;
    
    tracing::info!("gRPC server listening on {}", grpc_addr);
    
    if connect_manager {
        let connection = crate::connection::Connection::new(
            config,
            dispatcher.clone(),
            core,
        );

        tokio::select! {
            result = connection.run() => {
                result.map_err(|e| anyhow::anyhow!(e))?;
            }
            result = tonic::transport::Server::builder()
                .add_service(grpc_server)
                .serve(grpc_addr) => {
                result.map_err(|e| anyhow::anyhow!(e.to_string()))?;
            }
        }
    } else {
        tracing::info!("Standalone mode: starting API server...");
        
        let shared_config = crate::api_server::SharedConfig::new(config.clone());
        
        let api_state = std::sync::Arc::new(crate::api_server::ApiState {
            storage: core.storage.clone(),
            session_manager: core.session_manager.clone(),
            task_manager: core.task_manager.clone(),
            process_manager: core.process_manager.clone(),
            event_tx: core.event_tx.clone(),
            tool_registry: core.tool_registry.clone(),
            tool_executor: core.tool_executor.clone(),
            config: shared_config,
        });
        
        ensure_main_session(&api_state).await?;
        
        let api_addr_str = format!("0.0.0.0:{}", config.api_port());
        let api_listener = tokio::net::TcpListener::bind(&api_addr_str).await
            .map_err(|e| anyhow::anyhow!("Failed to bind API port: {}", e))?;
        
        tracing::info!("HTTP API server listening on {}", api_addr_str);
        tracing::info!("WebUI available at http://{}", api_addr_str);
        
        let grpc_task = tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(grpc_server)
                .serve(grpc_addr)
                .await
        });
        
        let app = crate::api_server::create_api_router(api_state);
        
        axum::serve(api_listener, app).await
            .map_err(|e| anyhow::anyhow!("API server error: {}", e))?;
        
        grpc_task.await
            .map_err(|e| anyhow::anyhow!("gRPC task error: {}", e))?
            .map_err(|e| anyhow::anyhow!("gRPC server error: {}", e))?;
    }

    Ok(())
}

async fn handle_provider_command(command: ProviderCommands) -> anyhow::Result<()> {
    let workspace = std::env::var("WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .map(|h| h.join("workspace"))
                .unwrap_or_else(|| PathBuf::from("/workspace"))
        });
    
    match command {
        ProviderCommands::List { workspace: ws } => {
            let workspace_path = ws.unwrap_or_else(|| workspace.clone());
            let manager = crate::provider::ProviderManager::new(workspace_path);
            let providers = manager.list_providers().await;
            let active_id = manager.get_active_provider().await.map(|p| p.id);
            
            if providers.is_empty() {
                println!("No providers configured.");
                println!("\nCreate one with: openzerg provider create -n <name> -u <base_url> -k <api_key> -m <model>");
            } else {
                println!("{:<15} {:<12} {:<50} {:<8}", "NAME", "MODEL", "BASE_URL", "ACTIVE");
                println!("{}", "-".repeat(90));
                for p in providers {
                    let active = if Some(&p.id) == active_id.as_ref() { "Yes" } else { "No" };
                    let base_url = if p.base_url.len() > 48 { format!("{}...", &p.base_url[..45]) } else { p.base_url.clone() };
                    println!("{:<15} {:<12} {:<50} {:<8}", p.name, p.model, base_url, active);
                }
            }
        }
        ProviderCommands::Get { provider, workspace: ws } => {
            let workspace_path = ws.unwrap_or_else(|| workspace.clone());
            let manager = crate::provider::ProviderManager::new(workspace_path);
            
            let p = if let Some(p) = manager.get_provider(&provider).await {
                p
            } else if let Some(p) = manager.get_provider_by_name(&provider).await {
                p
            } else {
                println!("Provider '{}' not found.", provider);
                return Ok(());
            };
            
            println!("ID: {}", p.id);
            println!("Name: {}", p.name);
            println!("Base URL: {}", p.base_url);
            println!("Model: {}", p.model);
            if let Some(v) = p.max_tokens { println!("Max Tokens: {}", v); }
            if let Some(v) = p.temperature { println!("Temperature: {}", v); }
            if let Some(v) = p.top_p { println!("Top P: {}", v); }
            if let Some(v) = p.top_k { println!("Top K: {}", v); }
            if let Some(ref v) = p.extra_params { println!("Extra Params: {}", v); }
            println!("Active: {}", p.is_active);
            println!("Created: {}", p.created_at);
        }
        ProviderCommands::Create { 
            name, base_url, api_key, model,
            max_tokens, temperature, top_p, top_k, extra_params,
            workspace: ws 
        } => {
            let workspace_path = ws.unwrap_or_else(|| workspace.clone());
            let manager = crate::provider::ProviderManager::new(workspace_path);
            
            let req = crate::provider::CreateProviderRequest {
                name,
                base_url,
                api_key,
                model,
                max_tokens,
                temperature,
                top_p,
                top_k,
                extra_params: extra_params.and_then(|s| serde_json::from_str(&s).ok()),
            };
            
            let provider = manager.create_provider(req).await?;
            println!("Provider '{}' created with ID: {}", provider.name, provider.id);
            if provider.is_active {
                println!("Set as active provider.");
            }
        }
        ProviderCommands::Update { 
            provider, name, base_url, api_key, model,
            max_tokens, temperature, top_p, top_k, extra_params,
            workspace: ws 
        } => {
            let workspace_path = ws.unwrap_or_else(|| workspace.clone());
            let manager = crate::provider::ProviderManager::new(workspace_path);
            
            let id = if let Some(p) = manager.get_provider(&provider).await {
                p.id
            } else if let Some(p) = manager.get_provider_by_name(&provider).await {
                p.id
            } else {
                println!("Provider '{}' not found.", provider);
                return Ok(());
            };
            
            let req = crate::provider::UpdateProviderRequest {
                name,
                base_url,
                api_key,
                model,
                max_tokens,
                temperature,
                top_p,
                top_k,
                extra_params: extra_params.and_then(|s| serde_json::from_str(&s).ok()),
            };
            
            match manager.update_provider(&id, req).await? {
                Some(p) => println!("Provider '{}' updated.", p.name),
                None => println!("Provider '{}' not found.", provider),
            }
        }
        ProviderCommands::Delete { provider, workspace: ws } => {
            let workspace_path = ws.unwrap_or_else(|| workspace.clone());
            let manager = crate::provider::ProviderManager::new(workspace_path);
            
            let id = if let Some(p) = manager.get_provider(&provider).await {
                p.id
            } else if let Some(p) = manager.get_provider_by_name(&provider).await {
                p.id
            } else {
                println!("Provider '{}' not found.", provider);
                return Ok(());
            };
            
            if manager.delete_provider(&id).await? {
                println!("Provider '{}' deleted.", provider);
            } else {
                println!("Provider '{}' not found.", provider);
            }
        }
        ProviderCommands::Use { provider, workspace: ws } => {
            let workspace_path = ws.unwrap_or_else(|| workspace.clone());
            let manager = crate::provider::ProviderManager::new(workspace_path);
            
            let success = if manager.set_active_provider(&provider).await? {
                true
            } else {
                manager.set_active_provider_by_name(&provider).await?
            };
            
            if success {
                let active = manager.get_active_provider().await;
                if let Some(p) = active {
                    println!("Active provider set to: {} ({})", p.name, p.model);
                }
            } else {
                println!("Provider '{}' not found.", provider);
            }
        }
    }
    
    Ok(())
}

async fn handle_config_command(command: ConfigCommands) -> anyhow::Result<()> {
    match command {
        ConfigCommands::Show { workspace } => {
            let workspace_path = workspace
                .or_else(|| std::env::var("WORKSPACE").ok().map(PathBuf::from))
                .or_else(|| dirs::home_dir().map(|h| h.join("workspace")))
                .unwrap_or_else(|| PathBuf::from("/workspace"));
            
            let openzerg_dir = workspace_path.join(".openzerg");
            let config_path = openzerg_dir.join("config.yaml");
            
            if config_path.exists() {
                let content = std::fs::read_to_string(&config_path)?;
                println!("Configuration file: {}", config_path.display());
                println!();
                println!("{}", content);
            } else {
                println!("No configuration file found at {}", config_path.display());
                println!();
                println!("Default configuration:");
                println!("  LLM Base URL: https://api.openai.com/v1");
                println!("  LLM Model: gpt-4o");
                println!("  API Port: 8081");
                println!("  gRPC Port: 50051");
            }
        }
        ConfigCommands::SetLlm { base_url, api_key, model, workspace } => {
            let workspace_path = workspace
                .or_else(|| std::env::var("WORKSPACE").ok().map(PathBuf::from))
                .or_else(|| dirs::home_dir().map(|h| h.join("workspace")))
                .unwrap_or_else(|| PathBuf::from("/workspace"));
            
            let openzerg_dir = workspace_path.join(".openzerg");
            std::fs::create_dir_all(&openzerg_dir)?;
            
            let config_path = openzerg_dir.join("config.yaml");
            
            let mut config: crate::config::RuntimeConfig = if config_path.exists() {
                let content = std::fs::read_to_string(&config_path)?;
                serde_yaml::from_str(&content).unwrap_or_default()
            } else {
                Default::default()
            };
            
            if let Some(v) = base_url {
                config.llm.base_url = v;
            }
            if let Some(v) = api_key {
                config.llm.api_key = v;
            }
            if let Some(v) = model {
                config.llm.model = v;
            }
            
            let content = serde_yaml::to_string(&config)?;
            std::fs::write(&config_path, content)?;
            
            println!("LLM configuration updated:");
            println!("  Base URL: {}", config.llm.base_url);
            println!("  Model: {}", config.llm.model);
            if !config.llm.api_key.is_empty() {
                println!("  API Key: {}***", &config.llm.api_key[..std::cmp::min(4, config.llm.api_key.len())]);
            }
        }
        ConfigCommands::SetVision { base_url, api_key, model, workspace } => {
            let workspace_path = workspace
                .or_else(|| std::env::var("WORKSPACE").ok().map(PathBuf::from))
                .or_else(|| dirs::home_dir().map(|h| h.join("workspace")))
                .unwrap_or_else(|| PathBuf::from("/workspace"));
            
            let openzerg_dir = workspace_path.join(".openzerg");
            std::fs::create_dir_all(&openzerg_dir)?;
            
            let config_path = openzerg_dir.join("config.yaml");
            
            let mut config: crate::config::RuntimeConfig = if config_path.exists() {
                let content = std::fs::read_to_string(&config_path)?;
                serde_yaml::from_str(&content).unwrap_or_default()
            } else {
                Default::default()
            };
            
            if let Some(v) = base_url {
                config.vision.base_url = Some(v);
            }
            if let Some(v) = api_key {
                config.vision.api_key = Some(v);
            }
            if let Some(v) = model {
                config.vision.model = Some(v);
            }
            
            let content = serde_yaml::to_string(&config)?;
            std::fs::write(&config_path, content)?;
            
            println!("Vision configuration updated.");
        }
        ConfigCommands::SetPort { port, workspace } => {
            let workspace_path = workspace
                .or_else(|| std::env::var("WORKSPACE").ok().map(PathBuf::from))
                .or_else(|| dirs::home_dir().map(|h| h.join("workspace")))
                .unwrap_or_else(|| PathBuf::from("/workspace"));
            
            let openzerg_dir = workspace_path.join(".openzerg");
            std::fs::create_dir_all(&openzerg_dir)?;
            
            let config_path = openzerg_dir.join("config.yaml");
            
            let mut config: crate::config::RuntimeConfig = if config_path.exists() {
                let content = std::fs::read_to_string(&config_path)?;
                serde_yaml::from_str(&content).unwrap_or_default()
            } else {
                Default::default()
            };
            
            config.api_port = port;
            
            let content = serde_yaml::to_string(&config)?;
            std::fs::write(&config_path, content)?;
            
            println!("API port set to: {}", port);
        }
    }
    Ok(())
}

async fn handle_tool_command(command: ToolCommands) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    
    match command {
        ToolCommands::List { url } => {
            let response = client
                .get(&format!("{}/tools", url))
                .send()
                .await?;
            
            if response.status().is_success() {
                let tools: Vec<serde_json::Value> = response.json().await?;
                
                if tools.is_empty() {
                    println!("No tools available.");
                } else {
                    println!("{:<20} {:<40}", "NAME", "DESCRIPTION");
                    println!("{}", "-".repeat(60));
                    for tool in tools {
                        let name = tool["name"].as_str().unwrap_or("unknown");
                        let desc = tool["description"].as_str().unwrap_or("");
                        let desc_short = if desc.len() > 37 {
                            format!("{}...", &desc[..37])
                        } else {
                            desc.to_string()
                        };
                        println!("{:<20} {:<40}", name, desc_short);
                    }
                }
            } else {
                println!("Error: {}", response.status());
            }
        }
        ToolCommands::Get { name, url } => {
            let response = client
                .get(&format!("{}/tools/{}", url, name))
                .send()
                .await?;
            
            if response.status().is_success() {
                let tool: serde_json::Value = response.json().await?;
                println!("Name: {}", tool["name"].as_str().unwrap_or("unknown"));
                println!("Description: {}", tool["description"].as_str().unwrap_or(""));
                if let Some(params) = tool.get("parameters") {
                    println!("\nParameters:");
                    println!("{}", serde_json::to_string_pretty(params)?);
                }
            } else {
                println!("Tool '{}' not found or agent not running.", name);
            }
        }
    }
    Ok(())
}

async fn handle_session_command(command: SessionCommands) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    
    match command {
        SessionCommands::List { url } => {
            let response = client
                .get(&format!("{}/sessions", url))
                .send()
                .await?;
            
            if response.status().is_success() {
                let sessions: Vec<serde_json::Value> = response.json().await?;
                
                if sessions.is_empty() {
                    println!("No sessions found.");
                } else {
                    println!("{:<36} {:<15} {:<20} {:<10}", "ID", "STATE", "PURPOSE", "MESSAGES");
                    println!("{}", "-".repeat(85));
                    for session in sessions {
                        let id = session["id"].as_str().unwrap_or("unknown");
                        let state = session["state"].as_str().unwrap_or("unknown");
                        let purpose = session["purpose"].as_str().unwrap_or("unknown");
                        let messages = session["message_count"].as_u64().unwrap_or(0);
                        println!("{:<36} {:<15} {:<20} {:<10}", id, state, purpose, messages);
                    }
                }
            } else {
                println!("Error: {} - Is the agent running?", response.status());
            }
        }
        SessionCommands::Get { id, url } => {
            let response = client
                .get(&format!("{}/sessions/{}", url, id))
                .send()
                .await?;
            
            if response.status().is_success() {
                let session: serde_json::Value = response.json().await?;
                println!("{}", serde_json::to_string_pretty(&session)?);
            } else {
                println!("Session '{}' not found.", id);
            }
        }
        SessionCommands::Messages { id, url, limit } => {
            let response = client
                .get(&format!("{}/sessions/{}/messages?limit={}", url, id, limit))
                .send()
                .await?;
            
            if response.status().is_success() {
                let messages: Vec<serde_json::Value> = response.json().await?;
                
                if messages.is_empty() {
                    println!("No messages in session.");
                } else {
                    for msg in messages.iter().rev() {
                        let role = msg["role"].as_str().unwrap_or("unknown");
                        println!("=== {} ===", role.to_uppercase());
                        if let Some(content) = msg["content"].as_str() {
                            println!("{}", content);
                        } else if let Some(parts) = msg["content"].as_array() {
                            for part in parts {
                                if let Some(text) = part["text"].as_str() {
                                    println!("{}", text);
                                }
                            }
                        }
                        println!();
                    }
                }
            } else {
                println!("Error fetching messages: {}", response.status());
            }
        }
        SessionCommands::Delete { id, url } => {
            let response = client
                .delete(&format!("{}/sessions/{}", url, id))
                .send()
                .await?;
            
            if response.status().is_success() {
                println!("Session '{}' deleted.", id);
            } else {
                println!("Error deleting session: {}", response.status());
            }
        }
    }
    Ok(())
}

async fn check_status(url: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    
    let response = client
        .get(&format!("{}/health", url))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await;
    
    match response {
        Ok(resp) if resp.status().is_success() => {
            println!("Agent is running at {}", url);
            if let Ok(info) = resp.json::<serde_json::Value>().await {
                if let Some(name) = info.get("agent_name") {
                    println!("Agent name: {}", name);
                }
                if let Some(version) = info.get("version") {
                    println!("Version: {}", version);
                }
            }
        }
        Ok(resp) => {
            println!("Agent returned status: {}", resp.status());
        }
        Err(e) => {
            println!("Agent is not responding at {}: {}", url, e);
        }
    }
    
    Ok(())
}

async fn ensure_main_session(state: &std::sync::Arc<crate::api_server::ApiState>) -> anyhow::Result<()> {
    let sessions = state.storage.load_sessions().await?;
    
    let main_exists = sessions.iter().any(|s| s.purpose == "Main");
    let dispatcher_exists = sessions.iter().any(|s| s.purpose == "Dispatcher");
    let worker_exists = sessions.iter().any(|s| s.purpose == "Worker");
    
    if main_exists && dispatcher_exists && worker_exists {
        tracing::info!("Main, Dispatcher, and Worker sessions already exist");
        return Ok(());
    }
    
    if !main_exists {
        let id = state.session_manager.init_main().await;
        
        let session = crate::storage::StoredSession {
            id: id.clone(),
            purpose: "Main".to_string(),
            state: "Idle".to_string(),
            created_at: chrono::Utc::now(),
            started_at: None,
            finished_at: None,
            task_id: None,
            query_id: None,
            message_count: 0,
        };
        state.storage.save_session(&session).await?;
        
        tracing::info!("Created Main session: {}", id);
    }
    
    if !dispatcher_exists {
        if let Some(dispatcher) = state.session_manager.get_dispatcher().await {
            let session = crate::storage::StoredSession {
                id: dispatcher.id.clone(),
                purpose: "Dispatcher".to_string(),
                state: "Idle".to_string(),
                created_at: chrono::Utc::now(),
                started_at: None,
                finished_at: None,
                task_id: None,
                query_id: None,
                message_count: 0,
            };
            state.storage.save_session(&session).await?;
            tracing::info!("Created Dispatcher session: {}", dispatcher.id);
        }
    }
    
    if !worker_exists {
        if let Some(worker) = state.session_manager.get_worker().await {
            let session = crate::storage::StoredSession {
                id: worker.id.clone(),
                purpose: "Worker".to_string(),
                state: "Idle".to_string(),
                created_at: chrono::Utc::now(),
                started_at: None,
                finished_at: None,
                task_id: None,
                query_id: None,
                message_count: 0,
            };
            state.storage.save_session(&session).await?;
            tracing::info!("Created Worker session: {}", worker.id);
        }
    }
    
    Ok(())
}