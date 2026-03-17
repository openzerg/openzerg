mod ws_client;
mod file_server;
mod stats_collector;
mod git_manager;
mod task_runner;
mod protocol;
mod error;

use std::env;
use error::Result;

#[derive(Debug, Clone)]
pub struct Config {
    pub agent_name: String,
    pub manager_url: String,
    pub internal_token: String,
    pub workspace: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            agent_name: env::var("AGENT_NAME")
                .unwrap_or_else(|_| "default".to_string()),
            manager_url: env::var("MANAGER_URL")
                .unwrap_or_else(|_| "ws://10.200.1.1:17531".to_string()),
            internal_token: env::var("INTERNAL_TOKEN")
                .expect("INTERNAL_TOKEN must be set"),
            workspace: env::var("WORKSPACE")
                .unwrap_or_else(|_| "/workspace".to_string()),
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    tracing::info!("Starting OpenZerg Agent...");

    let config = Config::from_env()?;
    tracing::info!("Agent name: {}", config.agent_name);
    tracing::info!("Manager URL: {}", config.manager_url);
    tracing::info!("Workspace: {}", config.workspace);

    std::fs::create_dir_all(&config.workspace)?;

    let ws_task = tokio::spawn(async move {
        ws_client::connect_to_manager(config).await
    });

    ws_task.await.map_err(|e| error::Error::WebSocket(e.to_string()))??;

    Ok(())
}