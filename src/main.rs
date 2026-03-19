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
mod config;
mod tool;
mod vision;
mod agent;
mod grpc;

use std::sync::Arc;
use error::Result;
use event::EventDispatcher;
use config::Config;
use agent::AgentCore;
use tonic::transport::Server;
use grpc::{AgentGrpcServer, agent::agent_service_server::AgentServiceServer};

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
    tracing::info!("gRPC Port: {}", config.grpc_port());
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

    let grpc_server = AgentServiceServer::new(AgentGrpcServer::new(
        core.session_manager.clone(),
        core.process_manager.clone(),
        core.tool_registry.clone(),
    ));
    
    let grpc_addr_str = format!("0.0.0.0:{}", config.grpc_port());
    let grpc_addr: std::net::SocketAddr = grpc_addr_str.parse()
        .map_err(|e| error::Error::Config(format!("Invalid gRPC address: {}", e)))?;
    
    tracing::info!("gRPC server listening on {}", grpc_addr);
    
    let connection = connection::Connection::new(
        config,
        dispatcher.clone(),
        core,
    );

    tokio::select! {
        result = connection.run() => {
            result?;
        }
        result = Server::builder()
            .add_service(grpc_server)
            .serve(grpc_addr) => {
                result.map_err(|e| error::Error::Internal(e.to_string()))?;
        }
    }

    Ok(())
}