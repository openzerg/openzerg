use super::core::AgentCore;

impl AgentCore {
    pub async fn handle_event(&self, event: crate::protocol::AgentEvent) {
        match event {
            crate::protocol::AgentEvent::Query { query_id, question } => {
                self.handle_query(query_id, question).await;
            }

            crate::protocol::AgentEvent::Message { content, from } => {
                self.handle_message(content, from).await;
            }

            crate::protocol::AgentEvent::AssignTask { task_id, title, description, priority, deadline, context } => {
                self.handle_assign_task(task_id, title, description, priority, deadline, context).await;
            }

            crate::protocol::AgentEvent::Remind { id, message } => {
                self.handle_remind(id, message).await;
            }

            crate::protocol::AgentEvent::ConfigUpdate { llm_base_url, llm_api_key, llm_model } => {
                self.handle_config_update(llm_base_url, llm_api_key, llm_model).await;
            }

            crate::protocol::AgentEvent::ResourceWarning { resource, message } => {
                tracing::warn!("Resource warning ({:?}): {}", resource, message);
            }

            crate::protocol::AgentEvent::ProcessNotification { process_id, event, output_preview: _ } => {
                tracing::debug!("Process {}: {:?}", process_id, event);
            }

            crate::protocol::AgentEvent::Interrupt { message, target_session } => {
                tracing::info!("Interrupt: {} (target: {:?})", message, target_session);
            }

            // SSE events are handled by the API server, not the agent
            crate::protocol::AgentEvent::SessionCreated { .. } => {}
            crate::protocol::AgentEvent::Thinking { .. } => {}
            crate::protocol::AgentEvent::Response { .. } => {}
            crate::protocol::AgentEvent::Done { .. } => {}
            crate::protocol::AgentEvent::Error { .. } => {}
        }
    }

    async fn handle_query(&self, query_id: String, question: String) {
        tracing::info!("Query: {} - {}", query_id, question);
        
        match self.session_manager.spawn(crate::session::SessionPurpose::Query).await {
            Ok(session_id) => {
                self.session_manager.bind_query(&session_id, &query_id).await.ok();
                self.session_manager.update_state(&session_id, crate::session::SessionState::Generating).await.ok();
                
                // Send SessionCreated event
                let _ = self.event_tx.send(crate::protocol::AgentEvent::SessionCreated {
                    session_id: session_id.clone(),
                    purpose: "Query".to_string(),
                });
                
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
                };
                self.storage.save_session(&session).await.ok();
                
                let msg = crate::storage::StoredMessage {
                    id: uuid::Uuid::new_v4().to_string(),
                    session_id: session_id.clone(),
                    role: crate::storage::MessageRole::User,
                    content: question.clone(),
                    timestamp: chrono::Utc::now(),
                    tool_calls: None,
                };
                self.storage.save_message(&msg).await.ok();
                
                // Send Thinking event
                let _ = self.event_tx.send(crate::protocol::AgentEvent::Thinking {
                    session_id: session_id.clone(),
                    content: "Processing your request...".to_string(),
                });
                
                match self.llm_client.complete(vec![
                    crate::llm::Message::user(&question),
                ]).await {
                    Ok(response) => {
                        let msg = crate::storage::StoredMessage {
                            id: uuid::Uuid::new_v4().to_string(),
                            session_id: session_id.clone(),
                            role: crate::storage::MessageRole::Assistant,
                            content: response.clone(),
                            timestamp: chrono::Utc::now(),
                            tool_calls: None,
                        };
                        self.storage.save_message(&msg).await.ok();
                        
                        // Send Response event
                        let _ = self.event_tx.send(crate::protocol::AgentEvent::Response {
                            session_id: session_id.clone(),
                            content: response,
                        });
                    }
                    Err(e) => {
                        tracing::error!("Query failed: {}", e);
                        // Send Error event
                        let _ = self.event_tx.send(crate::protocol::AgentEvent::Error {
                            session_id: session_id.clone(),
                            message: e.to_string(),
                        });
                    }
                }
                
                self.session_manager.complete(&session_id).await.ok();
                self.storage.finish_session(&session_id).await.ok();
                
                // Send Done event
                let _ = self.event_tx.send(crate::protocol::AgentEvent::Done {
                    session_id: session_id.clone(),
                });
            }
            Err(e) => {
                tracing::error!("Failed to create session for query: {}", e);
            }
        }
    }

    async fn handle_message(&self, content: String, from: String) {
        tracing::info!("Message from {}: {}", from, content);
        
        let activity = crate::storage::StoredActivity {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: None,
            activity_type: crate::storage::ActivityType::Message,
            description: format!("Message from {}: {}", from, content),
            details: serde_json::json!({ "from": from, "content": content }),
            timestamp: chrono::Utc::now(),
        };
        self.storage.save_activity(&activity).await.ok();
        
        match self.thinking_layer.process_event(crate::protocol::AgentEvent::Message { content, from }).await {
            Ok(plan) => {
                tracing::info!("Analysis: {}", plan.analysis);
                
                for mut task in plan.tasks {
                    let task_id = task.id.clone();
                    
                    match plan.assignments.get(&task_id) {
                        Some(crate::thinking::Assignment::NewSession) => {
                            match self.session_manager.spawn(crate::session::SessionPurpose::Task).await {
                                Ok(session_id) => {
                                    task.session_id = Some(session_id.clone());
                                    task.status = crate::task::TaskStatus::Assigned;
                                    let stored_task = crate::storage::StoredTask {
                                        id: task.id.clone(),
                                        content: task.title.clone(),
                                        status: format!("{:?}", task.status),
                                        priority: format!("{:?}", task.priority),
                                        session_id: Some(session_id.clone()),
                                        created_at: chrono::Utc::now(),
                                        updated_at: chrono::Utc::now(),
                                        completed_at: None,
                                    };
                                    self.storage.save_task(&stored_task).await.ok();
                                    self.session_manager.bind_task(&session_id, &task_id).await.ok();
                                }
                                Err(e) => {
                                    tracing::error!("Failed to spawn session: {}", e);
                                }
                            }
                        }
                        Some(crate::thinking::Assignment::ToSession(session_id)) => {
                            task.session_id = Some(session_id.clone());
                            task.status = crate::task::TaskStatus::Assigned;
                            let stored_task = crate::storage::StoredTask {
                                id: task.id.clone(),
                                content: task.title.clone(),
                                status: format!("{:?}", task.status),
                                priority: format!("{:?}", task.priority),
                                session_id: Some(session_id.clone()),
                                created_at: chrono::Utc::now(),
                                updated_at: chrono::Utc::now(),
                                completed_at: None,
                            };
                            self.storage.save_task(&stored_task).await.ok();
                            self.session_manager.bind_task(session_id, &task_id).await.ok();
                        }
                        _ => {
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
            }
            Err(e) => {
                tracing::error!("Thinking layer failed: {}", e);
            }
        }
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
        
        let event = crate::protocol::AgentEvent::AssignTask {
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
}