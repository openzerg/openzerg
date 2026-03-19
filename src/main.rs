mod event;
mod connection;
mod sse;
mod stats_collector;
mod protocol;
mod error;
mod process;
mod session;
mod task;
mod thinking;
mod llm;
mod file;
mod storage;
mod api_server;
mod config;
mod tool;
mod vision;
mod agent;
mod rpc;

use std::sync::Arc;
use error::Result;
use event::EventDispatcher;
use config::Config;
use agent::AgentCore;
use rpc::RpcRegistry;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting OpenZerg Agent v0.5.0...");

    let config = Config::from_env()?;
    tracing::info!("Agent name: {}", config.agent_name);
    tracing::info!("Manager URL: {}", config.manager_url);
    tracing::info!("Workspace: {}", config.workspace);
    tracing::info!("LLM Base URL: {}", config.llm_base_url());
    tracing::info!("LLM Model: {}", config.llm_model());
    tracing::info!("API Port: {}", config.api_port());
    tracing::info!("Vision enabled: {}", config.vision_enabled());

    std::fs::create_dir_all(&config.workspace)?;
    std::fs::create_dir_all(config.openzerg_dir())?;

    let core = Arc::new(AgentCore::new(config.clone()));
    core.init().await?;

    let dispatcher = Arc::new(EventDispatcher::new(config.agent_name.clone(), core.event_tx.clone()));

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

    let rpc_registry = Arc::new(RpcRegistry::new());
    rpc::handler::register_all_methods(&rpc_registry, core.clone()).await;
    tracing::info!("RPC handlers registered");

    let api_state = Arc::new(api_server::ApiState {
        storage: core.storage.clone(),
        session_manager: core.session_manager.clone(),
        task_manager: core.task_manager.clone(),
        process_manager: core.process_manager.clone(),
        event_tx: core.event_tx.clone(),
        tool_registry: core.tool_registry.clone(),
        tool_executor: core.tool_executor.clone(),
    });

    let app = api_server::create_api_router(api_state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.api_port())).await?;
    tracing::info!("API server listening on port {}", config.api_port());
    
    let api_handle = tokio::spawn(async move {
        axum::serve(listener, app).await
    });

    let connection = connection::Connection::new(
        config,
        dispatcher.clone(),
        rpc_registry,
        core,
    );

    tokio::select! {
        result = connection.run() => {
            result?;
        }
        result = api_handle => {
            result??;
        }
    }

    Ok(())
}