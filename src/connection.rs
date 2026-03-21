use std::sync::Arc;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;

use crate::protocol::{Message, VmConnect, VmHeartbeat, VmStatusReport};
use crate::event::EventDispatcher;
use crate::error::{Result, Error};
use crate::config::Config;
use crate::stats_collector;
use crate::agent::AgentCore;

pub struct Connection {
    config: Config,
    dispatcher: Arc<EventDispatcher>,
    core: Arc<AgentCore>,
}

impl Connection {
    pub fn new(
        config: Config,
        dispatcher: Arc<EventDispatcher>,
        core: Arc<AgentCore>,
    ) -> Self {
        Self { config, dispatcher, core }
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

        let (ws_tx, ws_rx) = ws_stream.split();
        let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel::<String>();

        let connect_msg = Message::VmConnect(VmConnect {
            agent_name: self.config.agent_name.clone(),
            internal_token: self.config.internal_token.clone(),
            timestamp: chrono::Utc::now(),
        });
        let json = connect_msg.to_json()?;
        outgoing_tx.send(json).map_err(|_| Error::Connection("Channel closed".into()))?;

        let heartbeat_tx = outgoing_tx.clone();
        let agent_name = self.config.agent_name.clone();
        let heartbeat_handle = tokio::spawn(async move {
            loop {
                let heartbeat = Message::VmHeartbeat(VmHeartbeat {
                    agent_name: agent_name.clone(),
                    timestamp: chrono::Utc::now(),
                });

                if let Ok(json) = heartbeat.to_json() {
                    if heartbeat_tx.send(json).is_err() {
                        break;
                    }
                }

                tokio::time::sleep(Duration::from_secs(10)).await;

                let status = stats_collector::collect_status();
                let report = Message::VmStatusReport(VmStatusReport {
                    agent_name: agent_name.clone(),
                    timestamp: chrono::Utc::now(),
                    data: status,
                });

                if let Ok(json) = report.to_json() {
                    if heartbeat_tx.send(json).is_err() {
                        break;
                    }
                }
            }
        });

        let sender_handle = tokio::spawn(async move {
            let mut tx = ws_tx;
            while let Some(json) = outgoing_rx.recv().await {
                if tx.send(WsMessage::Text(json.into())).await.is_err() {
                    break;
                }
            }
        });

        let result = async {
            let mut rx = ws_rx;
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
                        return Ok(());
                    }
                    Err(e) => {
                        tracing::error!("WebSocket error: {}", e);
                        return Err(Error::WebSocket(e.to_string()));
                    }
                    _ => {}
                }
            }
            Ok(())
        }.await;

        heartbeat_handle.abort();
        sender_handle.abort();

        result
    }
}