mod event;
mod connection;
mod sse;
mod stats_collector;
mod protocol;
mod error;
mod config;
mod process;
mod session;
mod task;
mod thinking;
mod llm;
mod file;
mod storage;
mod tool;
mod vision;
mod agent;
mod grpc;
mod cli;
mod provider;
mod api_server;
mod web;

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    let cli = cli::Cli::parse();
    cli::handle_command(cli).await
}