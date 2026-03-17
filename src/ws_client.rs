use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use futures_util::{SinkExt, StreamExt};
use crate::protocol::{Message, VmConnect, VmHeartbeat, VmStatusReport, VmTaskResult, VmFileTree, VmRepoList};
use crate::error::{Result, Error};

use crate::Config;

pub async fn connect_to_manager(config: Config) -> Result<()> {
    loop {
        tracing::info!("Connecting to Zerg Swarm at {}", config.manager_url);

        match connect_async(&config.manager_url).await {
            Ok((ws_stream, _)) => {
                tracing::info!("Connected to Zerg Swarm");

                let (mut tx, mut rx) = ws_stream.split();

                let connect_msg = Message::VmConnect(VmConnect {
                    agent_name: config.agent_name.clone(),
                    internal_token: config.internal_token.clone(),
                    timestamp: chrono::Utc::now(),
                });

                let json = connect_msg.to_json()?;
                tx.send(WsMessage::Text(json.into())).await
                    .map_err(|e| Error::WebSocket(e.to_string()))?;

                let heartbeat_config = config.clone();
                let heartbeat_handle = tokio::spawn(async move {
                    loop {
                        let heartbeat = Message::VmHeartbeat(VmHeartbeat {
                            agent_name: heartbeat_config.agent_name.clone(),
                            timestamp: chrono::Utc::now(),
                        });

                        if let Ok(json) = heartbeat.to_json() {
                            if tx.send(WsMessage::Text(json.into())).await.is_err() {
                                break;
                            }
                        }

                        tokio::time::sleep(Duration::from_secs(10)).await;

                        let status = crate::stats_collector::collect_status();
                        let report = Message::VmStatusReport(VmStatusReport {
                            agent_name: heartbeat_config.agent_name.clone(),
                            timestamp: chrono::Utc::now(),
                            data: status,
                        });

                        if let Ok(json) = report.to_json() {
                            if tx.send(WsMessage::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                });

                while let Some(msg) = rx.next().await {
                    match msg {
                        Ok(WsMessage::Text(text)) => {
                            if let Ok(message) = Message::from_json(&text) {
                                handle_manager_message(message, &config).await;
                            }
                        }
                        Ok(WsMessage::Close(_)) => {
                            tracing::warn!("Manager closed connection");
                            break;
                        }
                        Err(e) => {
                            tracing::error!("WebSocket error: {}", e);
                            break;
                        }
                        _ => {}
                    }
                }

                heartbeat_handle.abort();
            }
            Err(e) => {
                tracing::error!("Failed to connect: {}", e);
            }
        }

        tracing::info!("Reconnecting in 5 seconds...");
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn handle_manager_message(message: Message, config: &Config) {
    match message {
        Message::HostExecuteTask(task) => {
            let task_id = task.task_id.clone();
            tracing::info!("Executing task: {} - {}", task_id, task.command);
            let result = crate::task_runner::execute_task(&task, &config.workspace).await;
            
            let _response = Message::VmTaskResult(VmTaskResult {
                agent_name: config.agent_name.clone(),
                task_id,
                success: result.success,
                output: result.output,
                timestamp: chrono::Utc::now(),
            });

            tracing::info!("Task completed: success={}", result.success);
        }
        Message::HostRequestFiles(req) => {
            let path = req.path.as_ref().map(|p| p.as_str()).unwrap_or(&config.workspace);
            let _tree = crate::file_server::scan_directory(path);
            let _response = Message::VmFileTree(VmFileTree {
                agent_name: config.agent_name.clone(),
                timestamp: chrono::Utc::now(),
                data: _tree,
            });
            tracing::debug!("Sent file tree for {}", path);
        }
        Message::HostRequestRepos(_) => {
            let _repos = crate::git_manager::scan_repos(&config.workspace);
            let _response = Message::VmRepoList(VmRepoList {
                agent_name: config.agent_name.clone(),
                timestamp: chrono::Utc::now(),
                data: _repos,
            });
            tracing::debug!("Sent repo list");
        }
        _ => {
            tracing::warn!("Unexpected message from manager: {:?}", message);
        }
    }
}