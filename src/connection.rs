use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use futures_util::{SinkExt, StreamExt};

use crate::protocol::{Message, VmConnect, VmHeartbeat, VmStatusReport};
use crate::event::EventDispatcher;
use crate::error::{Result, Error};
use crate::Config;
use crate::stats_collector;

pub struct Connection {
    config: Config,
    dispatcher: Arc<EventDispatcher>,
}

impl Connection {
    pub fn new(config: Config, dispatcher: Arc<EventDispatcher>) -> Self {
        Self { config, dispatcher }
    }

    pub async fn run(&self) -> Result<()> {
        loop {
            tracing::info!("Connecting to Zerg Swarm at {}", self.config.manager_url);

            match self.connect_and_run().await {
                Ok(_) => {
                    tracing::warn!("Connection closed normally");
                }
                Err(e) => {
                    tracing::error!("Connection error: {}", e);
                }
            }

            tracing::info!("Reconnecting in 5 seconds...");
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    async fn connect_and_run(&self) -> Result<()> {
        let url = format!("{}/ws/vm", self.config.manager_url);
        
        let (ws_stream, _) = connect_async(&url)
            .await
            .map_err(|e| Error::Connection(e.to_string()))?;

        tracing::info!("Connected to Zerg Swarm");

        let (mut tx, mut rx) = ws_stream.split();

        let connect_msg = Message::VmConnect(VmConnect {
            agent_name: self.config.agent_name.clone(),
            internal_token: self.config.internal_token.clone(),
            timestamp: chrono::Utc::now(),
        });

        let json = connect_msg.to_json()?;
        tx.send(WsMessage::Text(json.into()))
            .await
            .map_err(|e| Error::WebSocket(e.to_string()))?;

        let heartbeat_config = self.config.clone();
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

                let status = stats_collector::collect_status();
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
                        match message {
                            Message::HostEvent(host_event) => {
                                let response = self.dispatcher.dispatch(host_event).await?;
                                if let Message::VmEventAck(ack) = response {
                                    tracing::debug!("Event acked: {}", ack.event_id);
                                }
                            }
                            _ => {
                                tracing::warn!("Unexpected message type from manager");
                            }
                        }
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
        Ok(())
    }
}