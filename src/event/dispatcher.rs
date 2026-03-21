use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use crate::protocol::{AgentEvent, HostEvent, Message, VmEventAck};
use crate::sse::SseManager;
use crate::error::Result;

pub struct EventDispatcher {
    agent_name: String,
    sse_manager: Arc<RwLock<SseManager>>,
    event_tx: broadcast::Sender<AgentEvent>,
    interrupt_tx: broadcast::Sender<InterruptSignal>,
}

#[derive(Debug, Clone)]
pub struct InterruptSignal {
    pub target_session: Option<String>,
    pub message: String,
}

impl EventDispatcher {
    pub fn new(agent_name: String, event_tx: broadcast::Sender<AgentEvent>) -> Self {
        let (interrupt_tx, _) = broadcast::channel(10);
        
        Self {
            agent_name,
            sse_manager: Arc::new(RwLock::new(SseManager::new())),
            event_tx,
            interrupt_tx,
        }
    }

    pub fn sse_manager(&self) -> Arc<RwLock<SseManager>> {
        self.sse_manager.clone()
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<AgentEvent> {
        self.event_tx.subscribe()
    }

    pub fn subscribe_interrupts(&self) -> broadcast::Receiver<InterruptSignal> {
        self.interrupt_tx.subscribe()
    }

    pub async fn dispatch(&self, host_event: HostEvent) -> Result<Message> {
        let event_id = host_event.event_id.clone();
        let event = host_event.event;

        let (accepted, message) = match event {
            AgentEvent::Interrupt { message, target_session } => {
                self.handle_interrupt(message.clone(), target_session).await;
                (true, Some(message))
            }

            AgentEvent::ProcessNotification { process_id, event, output_preview } => {
                self.handle_process_notification(process_id, event, output_preview).await;
                (true, None)
            }

            AgentEvent::Query { query_id, question } => {
                self.handle_query(query_id, question).await;
                (true, None)
            }

            AgentEvent::Message { content, from } => {
                self.handle_message(content, from).await;
                (true, None)
            }

            AgentEvent::AssignTask { task_id, title, description, priority, deadline, context } => {
                self.handle_assign_task(task_id, title, description, priority, deadline, context).await;
                (true, None)
            }

            AgentEvent::Remind { id, message } => {
                self.handle_remind(id, message).await;
                (true, None)
            }

            AgentEvent::ConfigUpdate { llm_base_url, llm_api_key, llm_model } => {
                self.handle_config_update(llm_base_url, llm_api_key, llm_model).await;
                (true, None)
            }

            AgentEvent::ResourceWarning { resource, message } => {
                self.handle_resource_warning(resource, message).await;
                (true, None)
            }

            // SSE events are internal, just acknowledge them
            AgentEvent::SessionCreated { .. } => (true, None),
            AgentEvent::Thinking { .. } => (true, None),
            AgentEvent::Response { .. } => (true, None),
            AgentEvent::Done { .. } => (true, None),
            AgentEvent::Error { .. } => (true, None),
            AgentEvent::SubSessionResult { .. } => (true, None),
            AgentEvent::SessionTask { .. } => (true, None),
            AgentEvent::UserMessage { .. } => (true, None),
        };

        Ok(Message::VmEventAck(VmEventAck {
            event_id,
            accepted,
            message,
        }))
    }

    async fn handle_interrupt(&self, message: String, target_session: Option<String>) {
        tracing::info!("Interrupt received: {}", message);
        
        let signal = InterruptSignal {
            target_session: target_session.clone(),
            message,
        };

        let _ = self.interrupt_tx.send(signal);
    }

    async fn handle_process_notification(
        &self,
        process_id: String,
        event: crate::protocol::ProcessEvent,
        output_preview: Option<String>,
    ) {
        tracing::info!("Process notification: {} - {:?}", process_id, event);
        
        let _ = self.event_tx.send(AgentEvent::ProcessNotification {
            process_id,
            event,
            output_preview,
        });
    }

    async fn handle_query(&self, query_id: String, question: String) {
        tracing::info!("Query received: {} - {}", query_id, question);
        
        let _ = self.event_tx.send(AgentEvent::Query {
            query_id,
            question,
        });
    }

    async fn handle_message(&self, content: String, from: String) {
        tracing::info!("Message from {}: {}", from, content);
        
        let _ = self.event_tx.send(AgentEvent::Message {
            content,
            from,
        });
    }

    async fn handle_assign_task(
        &self,
        task_id: String,
        title: String,
        description: String,
        priority: crate::protocol::Priority,
        deadline: Option<chrono::DateTime<chrono::Utc>>,
        context: Option<serde_json::Value>,
    ) {
        tracing::info!("Task assigned: {} - {}", task_id, title);
        
        let _ = self.event_tx.send(AgentEvent::AssignTask {
            task_id,
            title,
            description,
            priority,
            deadline,
            context,
        });
    }

    async fn handle_remind(&self, id: String, message: String) {
        tracing::info!("Remind: {} - {}", id, message);
        
        let _ = self.event_tx.send(AgentEvent::Remind {
            id,
            message,
        });
    }

    async fn handle_config_update(
        &self,
        llm_base_url: Option<String>,
        llm_api_key: Option<String>,
        llm_model: Option<String>,
    ) {
        tracing::info!("Config update received");
        
        let _ = self.event_tx.send(AgentEvent::ConfigUpdate {
            llm_base_url,
            llm_api_key,
            llm_model,
        });
    }

    async fn handle_resource_warning(
        &self,
        resource: crate::protocol::ResourceType,
        message: String,
    ) {
        tracing::warn!("Resource warning: {:?} - {}", resource, message);
        
        let _ = self.event_tx.send(AgentEvent::ResourceWarning {
            resource,
            message,
        });
    }
}

pub struct EventDispatcherHandle {
    event_tx: broadcast::Sender<AgentEvent>,
    interrupt_tx: broadcast::Sender<InterruptSignal>,
}

impl EventDispatcherHandle {
    pub fn subscribe_events(&self) -> broadcast::Receiver<AgentEvent> {
        self.event_tx.subscribe()
    }

    pub fn subscribe_interrupts(&self) -> broadcast::Receiver<InterruptSignal> {
        self.interrupt_tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ProcessEvent, Priority, ResourceType};

    fn create_test_dispatcher(name: &str) -> EventDispatcher {
        let (event_tx, _) = broadcast::channel(100);
        EventDispatcher::new(name.to_string(), event_tx)
    }

    #[test]
    fn test_event_dispatcher_new() {
        let dispatcher = create_test_dispatcher("test-agent");
        assert!(dispatcher.sse_manager().try_read().is_ok());
    }

    #[tokio::test]
    async fn test_dispatch_interrupt() {
        let dispatcher = create_test_dispatcher("test");
        let mut rx = dispatcher.subscribe_interrupts();
        
        let event = HostEvent {
            event_id: "e1".to_string(),
            event: AgentEvent::Interrupt {
                message: "stop".to_string(),
                target_session: None,
            },
        };
        
        let result = dispatcher.dispatch(event).await.unwrap();
        match result {
            Message::VmEventAck(ack) => {
                assert!(ack.accepted);
                assert_eq!(ack.message, Some("stop".to_string()));
            }
            _ => panic!("Wrong message type"),
        }
        
        let signal = rx.recv().await.unwrap();
        assert_eq!(signal.message, "stop");
    }

    #[tokio::test]
    async fn test_dispatch_process_notification() {
        let dispatcher = create_test_dispatcher("test");
        let mut rx = dispatcher.subscribe_events();
        
        let event = HostEvent {
            event_id: "e1".to_string(),
            event: AgentEvent::ProcessNotification {
                process_id: "p1".to_string(),
                event: ProcessEvent::Started,
                output_preview: None,
            },
        };
        
        dispatcher.dispatch(event).await.unwrap();
        
        let received = rx.recv().await.unwrap();
        match received {
            AgentEvent::ProcessNotification { process_id, .. } => {
                assert_eq!(process_id, "p1");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_dispatch_query() {
        let dispatcher = create_test_dispatcher("test");
        let mut rx = dispatcher.subscribe_events();
        
        let event = HostEvent {
            event_id: "e1".to_string(),
            event: AgentEvent::Query {
                query_id: "q1".to_string(),
                question: "What is this?".to_string(),
            },
        };
        
        dispatcher.dispatch(event).await.unwrap();
        
        let received = rx.recv().await.unwrap();
        match received {
            AgentEvent::Query { query_id, question } => {
                assert_eq!(query_id, "q1");
                assert_eq!(question, "What is this?");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_dispatch_message() {
        let dispatcher = create_test_dispatcher("test");
        let mut rx = dispatcher.subscribe_events();
        
        let event = HostEvent {
            event_id: "e1".to_string(),
            event: AgentEvent::Message {
                content: "Hello".to_string(),
                from: "user".to_string(),
            },
        };
        
        dispatcher.dispatch(event).await.unwrap();
        
        let received = rx.recv().await.unwrap();
        match received {
            AgentEvent::Message { content, from } => {
                assert_eq!(content, "Hello");
                assert_eq!(from, "user");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_dispatch_assign_task() {
        let dispatcher = create_test_dispatcher("test");
        let mut rx = dispatcher.subscribe_events();
        
        let event = HostEvent {
            event_id: "e1".to_string(),
            event: AgentEvent::AssignTask {
                task_id: "t1".to_string(),
                title: "Task".to_string(),
                description: "Description".to_string(),
                priority: Priority::High,
                deadline: None,
                context: None,
            },
        };
        
        dispatcher.dispatch(event).await.unwrap();
        
        let received = rx.recv().await.unwrap();
        match received {
            AgentEvent::AssignTask { task_id, title, .. } => {
                assert_eq!(task_id, "t1");
                assert_eq!(title, "Task");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_dispatch_remind() {
        let dispatcher = create_test_dispatcher("test");
        let mut rx = dispatcher.subscribe_events();
        
        let event = HostEvent {
            event_id: "e1".to_string(),
            event: AgentEvent::Remind {
                id: "r1".to_string(),
                message: "Remember this".to_string(),
            },
        };
        
        dispatcher.dispatch(event).await.unwrap();
        
        let received = rx.recv().await.unwrap();
        match received {
            AgentEvent::Remind { id, message } => {
                assert_eq!(id, "r1");
                assert_eq!(message, "Remember this");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_dispatch_config_update() {
        let dispatcher = create_test_dispatcher("test");
        let mut rx = dispatcher.subscribe_events();
        
        let event = HostEvent {
            event_id: "e1".to_string(),
            event: AgentEvent::ConfigUpdate {
                llm_base_url: Some("http://api".to_string()),
                llm_api_key: None,
                llm_model: Some("gpt-4".to_string()),
            },
        };
        
        dispatcher.dispatch(event).await.unwrap();
        
        let received = rx.recv().await.unwrap();
        match received {
            AgentEvent::ConfigUpdate { llm_base_url, llm_model, .. } => {
                assert_eq!(llm_base_url, Some("http://api".to_string()));
                assert_eq!(llm_model, Some("gpt-4".to_string()));
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_dispatch_resource_warning() {
        let dispatcher = create_test_dispatcher("test");
        let mut rx = dispatcher.subscribe_events();
        
        let event = HostEvent {
            event_id: "e1".to_string(),
            event: AgentEvent::ResourceWarning {
                resource: ResourceType::Memory,
                message: "Low memory".to_string(),
            },
        };
        
        dispatcher.dispatch(event).await.unwrap();
        
        let received = rx.recv().await.unwrap();
        match received {
            AgentEvent::ResourceWarning { resource, message } => {
                assert_eq!(resource, ResourceType::Memory);
                assert_eq!(message, "Low memory");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_interrupt_signal() {
        let signal = InterruptSignal {
            target_session: Some("s1".to_string()),
            message: "stop".to_string(),
        };
        assert_eq!(signal.target_session, Some("s1".to_string()));
        assert_eq!(signal.message, "stop");
    }
}