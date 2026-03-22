use super::core::AgentCore;
use crate::protocol::AgentEvent;
use crate::session::SessionPurpose;
use futures::StreamExt;

impl AgentCore {
    pub async fn handle_event(&self, event: AgentEvent) {
        match event {
            AgentEvent::Query { query_id, question } => {
                self.handle_query(query_id, question).await;
            }

            AgentEvent::Message { content, from } => {
                self.handle_message(content, from).await;
            }

            AgentEvent::SessionTask { session_id, task, context } => {
                self.handle_session_task(session_id, task, context).await;
            }

            AgentEvent::AssignTask { task_id, title, description, priority, deadline, context } => {
                self.handle_assign_task(task_id, title, description, priority, deadline, context).await;
            }

            AgentEvent::Remind { id, message } => {
                self.handle_remind(id, message).await;
            }

            AgentEvent::ConfigUpdate { llm_base_url, llm_api_key, llm_model } => {
                self.handle_config_update(llm_base_url, llm_api_key, llm_model).await;
            }

            AgentEvent::ResourceWarning { resource, message } => {
                tracing::warn!("Resource warning ({:?}): {}", resource, message);
            }

            AgentEvent::ProcessNotification { process_id, event, output_preview: _ } => {
                tracing::debug!("Process {}: {:?}", process_id, event);
            }

            AgentEvent::Interrupt { message, target_session } => {
                tracing::info!("Interrupt: {} (target: {:?})", message, target_session);
            }

            AgentEvent::SessionCreated { .. } => {}
            AgentEvent::Thinking { .. } => {}
            AgentEvent::Response { .. } => {}
            AgentEvent::Done { .. } => {}
            AgentEvent::Error { .. } => {}
            AgentEvent::SubSessionResult { .. } => {}
            AgentEvent::UserMessage { .. } => {}
        }
    }

    async fn handle_query(&self, query_id: String, question: String) {
        tracing::info!("Query: {} - {}", query_id, question);
        
        let main_session_id = self.session_manager.get_main().await.map(|s| s.id.clone());
        
        if let Some(ref main_id) = main_session_id {
            let msg = crate::storage::StoredMessage {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: main_id.clone(),
                role: crate::storage::MessageRole::User,
                content: question.clone(),
                timestamp: chrono::Utc::now(),
                tool_calls: None,
            };
            self.storage.save_message(&msg).await.ok();
        }
        
        match self.session_manager.spawn(SessionPurpose::Query).await {
            Ok(session_id) => {
                self.session_manager.bind_query(&session_id, &query_id).await.ok();
                self.session_manager.update_state(&session_id, crate::session::SessionState::Generating).await.ok();
                
                let _ = self.event_tx.send(AgentEvent::SessionCreated {
                    session_id: session_id.clone(),
                    purpose: "Query".to_string(),
                });
                
                let system_prompt = include_str!("prompts/sessions/query.md").to_string();
                let session = crate::storage::StoredSession {
                    id: session_id.clone(),
                    purpose: "Query".to_string(),
                    state: "Generating".to_string(),
                    created_at: chrono::Utc::now(),
                    started_at: Some(chrono::Utc::now()),
                    finished_at: None,
                    task_id: None,
                    query_id: Some(query_id.clone()),
                    message_count: 0,
                    system_prompt: system_prompt.clone(),
                };
                self.storage.save_session(&session).await.ok();
                
                let _ = self.event_tx.send(AgentEvent::Thinking {
                    session_id: session_id.clone(),
                    content: "Processing your request...".to_string(),
                });
                
                let messages = vec![crate::llm::Message::user(&question)];
                let mut stream = Box::pin(self.llm_client.stream(messages));
                let mut full_response = String::new();
                let mut status = "completed".to_string();
                
                while let Some(chunk) = stream.next().await {
                    match chunk {
                        Ok(crate::llm::StreamChunk::Content(content)) => {
                            full_response.push_str(&content);
                            
                            let _ = self.event_tx.send(AgentEvent::Response {
                                session_id: session_id.clone(),
                                content,
                            });
                        }
                        Ok(crate::llm::StreamChunk::Done) => {
                            break;
                        }
                        Err(e) => {
                            tracing::error!("Query stream failed: {}", e);
                            status = "failed".to_string();
                            let _ = self.event_tx.send(AgentEvent::Error {
                                session_id: session_id.clone(),
                                message: e.to_string(),
                            });
                            break;
                        }
                        _ => {}
                    }
                }
                
                self.session_manager.complete(&session_id).await.ok();
                self.storage.finish_session(&session_id).await.ok();
                
                let _ = self.event_tx.send(AgentEvent::Done {
                    session_id: session_id.clone(),
                });
                
                if let Some(ref main_id) = main_session_id {
                    if status == "completed" && !full_response.is_empty() {
                        let msg = crate::storage::StoredMessage {
                            id: uuid::Uuid::new_v4().to_string(),
                            session_id: main_id.clone(),
                            role: crate::storage::MessageRole::Assistant,
                            content: full_response.clone(),
                            timestamp: chrono::Utc::now(),
                            tool_calls: None,
                        };
                        self.storage.save_message(&msg).await.ok();
                    }
                    
                    let _ = self.event_tx.send(AgentEvent::SubSessionResult {
                        parent_session_id: main_id.clone(),
                        child_session_id: session_id.clone(),
                        child_session_type: "Query".to_string(),
                        status,
                        summary: full_response.chars().take(100).collect::<String>(),
                        details: full_response.clone(),
                    });
                }
            }
            Err(e) => {
                tracing::error!("Failed to create session for query: {}", e);
            }
        }
    }

    async fn handle_message(&self, content: String, from: String) {
        tracing::info!("Message from {}: {}", from, content);
        
        let main_session = self.session_manager.get_main().await;
        if let Some(ref main) = main_session {
            let msg = crate::storage::StoredMessage {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: main.id.clone(),
                role: if from == "user" { crate::storage::MessageRole::User } else { crate::storage::MessageRole::Assistant },
                content: content.clone(),
                timestamp: chrono::Utc::now(),
                tool_calls: None,
            };
            self.storage.save_message(&msg).await.ok();
        }
        
        let activity = crate::storage::StoredActivity {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: None,
            activity_type: crate::storage::ActivityType::Message,
            description: format!("Message from {}: {}", from, content),
            details: serde_json::json!({ "from": from, "content": content }),
            timestamp: chrono::Utc::now(),
        };
        self.storage.save_activity(&activity).await.ok();
        
        let _ = self.event_tx.send(AgentEvent::Thinking {
            session_id: self.session_manager.get_dispatcher().await.map(|d| d.id).unwrap_or_default(),
            content: "Analyzing your request...".to_string(),
        });
        
        match self.thinking_layer.process_event(AgentEvent::Message { content: content.clone(), from }).await {
            Ok(plan) => {
                tracing::info!("Analysis: {}", plan.analysis);
                
                // Save Dispatcher's analysis to Dispatcher session's message history
                let dispatcher = self.session_manager.get_dispatcher().await;
                if let Some(ref disp) = dispatcher {
                    let msg = crate::storage::StoredMessage {
                        id: uuid::Uuid::new_v4().to_string(),
                        session_id: disp.id.clone(),
                        role: crate::storage::MessageRole::Assistant,
                        content: format!("Analysis: {}\nPlan: {} tasks created", plan.analysis, plan.tasks.len()),
                        timestamp: chrono::Utc::now(),
                        tool_calls: None,
                    };
                    if let Err(e) = self.storage.save_message(&msg).await {
                        tracing::error!("Failed to save Dispatcher message: {}", e);
                    } else {
                        tracing::info!("Saved Dispatcher analysis to session {}", disp.id);
                    }
                } else {
                    tracing::warn!("Dispatcher session not found, cannot save analysis");
                }
                
                for mut task in plan.tasks {
                    let task_id = task.id.clone();
                    let session_id = match plan.assignments.get(&task_id) {
                        Some(crate::thinking::Assignment::ToSession(sid)) => Some(sid.clone()),
                        Some(crate::thinking::Assignment::NewSession) => {
                            match self.session_manager.spawn(SessionPurpose::Task).await {
                                Ok(sid) => {
                                    task.session_id = Some(sid.clone());
                                    Some(sid)
                                }
                                Err(e) => {
                                    tracing::error!("Failed to spawn session: {}", e);
                                    None
                                }
                            }
                        }
                        _ => None,
                    };
                    
                    if let Some(sid) = session_id {
                        task.status = crate::task::TaskStatus::Assigned;
                        
                        let system_prompt = self.get_system_prompt(SessionPurpose::Task).await;
                        let session_record = crate::storage::StoredSession {
                            id: sid.clone(),
                            purpose: "Task".to_string(),
                            state: "Idle".to_string(),
                            created_at: chrono::Utc::now(),
                            started_at: None,
                            finished_at: None,
                            task_id: Some(task_id.clone()),
                            query_id: None,
                            message_count: 0,
                            system_prompt: system_prompt.clone().unwrap_or_default(),
                        };
                        self.storage.save_session(&session_record).await.ok();
                        
                        let stored_task = crate::storage::StoredTask {
                            id: task.id.clone(),
                            content: task.title.clone(),
                            status: format!("{:?}", task.status),
                            priority: format!("{:?}", task.priority),
                            session_id: Some(sid.clone()),
                            created_at: chrono::Utc::now(),
                            updated_at: chrono::Utc::now(),
                            completed_at: None,
                        };
                        self.storage.save_task(&stored_task).await.ok();
                        self.session_manager.bind_task(&sid, &task_id).await.ok();
                        
                        let _ = self.event_tx.send(AgentEvent::SessionTask {
                            session_id: sid.clone(),
                            task: task.title.clone(),
                            context: None,
                        });
                    } else {
                        task.status = crate::task::TaskStatus::Pending;
                        let stored_task = crate::storage::StoredTask {
                            id: task.id.clone(),
                            content: task.title.clone(),
                            status: format!("{:?}", task.status),
                            priority: format!("{:?}", task.priority),
                            session_id: None,
                            created_at: chrono::Utc::now(),
                            updated_at: chrono::Utc::now(),
                            completed_at: None,
                        };
                        self.storage.save_task(&stored_task).await.ok();
                    }
                }
            }
            Err(e) => {
                tracing::error!("Thinking layer failed: {}", e);
                
                if let Some(main) = main_session {
                    let _ = self.event_tx.send(AgentEvent::SessionTask {
                        session_id: main.id.clone(),
                        task: content,
                        context: None,
                    });
                }
            }
        }
    }

    async fn handle_session_task(&self, session_id: String, task: String, _context: Option<serde_json::Value>) {
        tracing::info!("SessionTask for {}: {}", session_id, task);
        
        let session = self.session_manager.get(&session_id).await;
        if session.is_none() {
            tracing::error!("Session not found: {}", session_id);
            return;
        }
        
        let session = session.unwrap();
        let purpose = session.purpose;
        
        self.session_manager.update_state(&session_id, crate::session::SessionState::Generating).await.ok();
        self.storage.update_session_state(&session_id, "Generating").await.ok();
        
        let system_prompt = self.get_system_prompt(purpose).await;
        
        let mut messages = Vec::new();
        if let Some(ref prompt) = system_prompt {
            messages.push(crate::llm::Message::system(prompt));
        }
        
        let history = self.storage.get_messages(&session_id).await.unwrap_or_default();
        for msg in history {
            let m = match msg.role {
                crate::storage::MessageRole::User => crate::llm::Message::user(&msg.content),
                crate::storage::MessageRole::Assistant => crate::llm::Message::assistant(&msg.content),
                _ => continue,
            };
            messages.push(m);
        }
        messages.push(crate::llm::Message::user(&task));
        
        let mut full_response = String::new();
        let mut thinking_sent = false;
        
        let mut stream = Box::pin(self.llm_client.stream(messages));
        
        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(crate::llm::StreamChunk::Content(content)) => {
                    full_response.push_str(&content);
                    
                    if !thinking_sent {
                        let _ = self.event_tx.send(AgentEvent::Thinking {
                            session_id: session_id.clone(),
                            content: "Processing your request...".to_string(),
                        });
                        thinking_sent = true;
                    }
                    
                    let _ = self.event_tx.send(AgentEvent::Response {
                        session_id: session_id.clone(),
                        content: content,
                    });
                }
                Ok(crate::llm::StreamChunk::Done) => {
                    break;
                }
                Err(e) => {
                    tracing::error!("Stream error: {}", e);
                    let _ = self.event_tx.send(AgentEvent::Error {
                        session_id: session_id.clone(),
                        message: e.to_string(),
                    });
                    break;
                }
                _ => {}
            }
        }
        
        if !full_response.is_empty() {
            let msg = crate::storage::StoredMessage {
                id: uuid::Uuid::new_v4().to_string(),
                session_id: session_id.clone(),
                role: crate::storage::MessageRole::Assistant,
                content: full_response.clone(),
                timestamp: chrono::Utc::now(),
                tool_calls: None,
            };
            self.storage.save_message(&msg).await.ok();
        }
        
        if purpose != SessionPurpose::Main && purpose != SessionPurpose::Dispatcher {
            self.session_manager.complete(&session_id).await.ok();
            self.storage.finish_session(&session_id).await.ok();
        } else {
            self.session_manager.update_state(&session_id, crate::session::SessionState::Idle).await.ok();
            self.storage.update_session_state(&session_id, "Idle").await.ok();
        }
        
        let _ = self.event_tx.send(AgentEvent::Done {
            session_id: session_id.clone(),
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
        
        let stored_task = crate::storage::StoredTask {
            id: task_id.clone(),
            content: title.clone(),
            status: "Pending".to_string(),
            priority: format!("{:?}", priority),
            session_id: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            completed_at: None,
        };
        self.storage.save_task(&stored_task).await.ok();
        
        let event = AgentEvent::AssignTask {
            task_id,
            title,
            description,
            priority,
            deadline,
            context,
        };
        
        match self.thinking_layer.process_event(event).await {
            Ok(plan) => {
                tracing::info!("Task analysis: {}", plan.analysis);
            }
            Err(e) => {
                tracing::error!("Task planning failed: {}", e);
            }
        }
    }

    async fn handle_remind(&self, id: String, message: String) {
        tracing::info!("Remind {}: {}", id, message);
        
        let activity = crate::storage::StoredActivity {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: None,
            activity_type: crate::storage::ActivityType::Message,
            description: format!("Remind: {}", message),
            details: serde_json::json!({ "remind_id": id, "message": message }),
            timestamp: chrono::Utc::now(),
        };
        self.storage.save_activity(&activity).await.ok();
    }

    async fn handle_config_update(
        &self,
        llm_base_url: Option<String>,
        llm_api_key: Option<String>,
        llm_model: Option<String>,
    ) {
        tracing::info!("Config update received");
        if let Some(url) = &llm_base_url {
            tracing::info!("  LLM Base URL: {}", url);
        }
        if let Some(key) = &llm_api_key {
            tracing::info!("  LLM API Key: {}...", &key[..8.min(key.len())]);
        }
        if let Some(model) = &llm_model {
            tracing::info!("  LLM Model: {}", model);
        }
        
        self.llm_client.update_config(llm_base_url, llm_api_key, llm_model).await;
    }
    
    async fn get_system_prompt(&self, purpose: SessionPurpose) -> Option<String> {
        match purpose {
            SessionPurpose::Main => Some(include_str!("prompts/sessions/main.md").to_string()),
            SessionPurpose::Dispatcher => Some(include_str!("prompts/sessions/dispatcher.md").to_string()),
            SessionPurpose::Worker => Some(include_str!("prompts/sessions/worker.md").to_string()),
            SessionPurpose::Query => Some(include_str!("prompts/sessions/query.md").to_string()),
            SessionPurpose::Task => Some(include_str!("prompts/sessions/task.md").to_string()),
            SessionPurpose::Remind => None,
        }
    }
}