use std::sync::Arc;
use crate::error::{Error, Result};
use crate::llm::{LLMClient, Message, ToolCall, ChatCompletionResponse, FunctionCall};
use crate::tool::ToolExecutor;
use crate::storage::Storage;

const MAX_TOOL_ITERATIONS: usize = 10;

pub struct SessionProcessor {
    llm_client: Arc<LLMClient>,
    tool_executor: Arc<ToolExecutor>,
    storage: Arc<Storage>,
}

impl SessionProcessor {
    pub fn new(
        llm_client: Arc<LLMClient>,
        tool_executor: Arc<ToolExecutor>,
        storage: Arc<Storage>,
    ) -> Self {
        Self {
            llm_client,
            tool_executor,
            storage,
        }
    }
    
    pub async fn process(
        &self,
        session_id: &str,
        initial_message: &str,
        system_prompt: Option<&str>,
    ) -> Result<String> {
        let mut messages = Vec::new();
        
        if let Some(prompt) = system_prompt {
            messages.push(Message::system(prompt));
        }
        
        messages.push(Message::user(initial_message));
        
        let mut iteration = 0;
        let mut final_response = String::new();
        
        loop {
            iteration += 1;
            
            if iteration > MAX_TOOL_ITERATIONS {
                return Err(Error::LLM(format!("Max tool iterations ({}) reached", MAX_TOOL_ITERATIONS)));
            }
            
            let tools = self.tool_executor.get_tool_definitions().await;
            
            let response = self.llm_client
                .complete_with_tools(messages.clone(), tools)
                .await?;
            
            let choice = response.choices.first()
                .ok_or_else(|| Error::LLM("No response choice".into()))?;
            
            let assistant_message = &choice.message;
            
            let assistant_content = assistant_message.content.clone();
            if !assistant_content.is_empty() {
                final_response = assistant_content.clone();
            }
            
            let tool_calls = match &assistant_message.tool_calls {
                Some(calls) if !calls.is_empty() => calls,
                _ => {
                    return Ok(final_response);
                }
            };
            
            let message_id = uuid::Uuid::new_v4().to_string();
            let stored_msg = crate::storage::StoredMessage {
                id: message_id.clone(),
                session_id: session_id.to_string(),
                role: crate::storage::MessageRole::Assistant,
                content: assistant_content.clone(),
                timestamp: chrono::Utc::now(),
                tool_calls: Some(tool_calls.iter().map(|tc| {
                    crate::storage::StoredToolCall {
                        id: tc.id.clone(),
                        name: tc.function.name.clone(),
                        arguments: tc.function.arguments.clone(),
                    }
                }).collect()),
            };
            self.storage.save_message(&stored_msg).await?;
            
            messages.push(Message::assistant_with_tools(
                &assistant_content,
                tool_calls.clone(),
            ));
            
            let results = self.tool_executor
                .execute_tool_calls(tool_calls, session_id, &message_id)
                .await;
            
            for (tool_call_id, result) in results {
                let content = match result {
                    Ok(r) => {
                        let stored_result = crate::storage::StoredToolResult {
                            tool_call_id: tool_call_id.clone(),
                            output: r.output.clone(),
                            success: true,
                        };
                        self.storage.save_tool_result(&stored_result).await.ok();
                        r.output
                    }
                    Err(e) => {
                        format!("Error: {}", e)
                    }
                };
                
                messages.push(Message::tool_result(&tool_call_id, &content));
            }
        }
    }
    
    pub async fn process_with_history(
        &self,
        session_id: &str,
        user_message: &str,
        system_prompt: Option<&str>,
    ) -> Result<String> {
        let mut messages = Vec::new();
        
        if let Some(prompt) = system_prompt {
            messages.push(Message::system(prompt));
        }
        
        let history = self.storage.get_messages(session_id).await?;
        
        for msg in history {
            let m = match msg.role {
                crate::storage::MessageRole::System => Message::system(&msg.content),
                crate::storage::MessageRole::User => Message::user(&msg.content),
                crate::storage::MessageRole::Assistant => {
                    if let Some(tool_calls) = &msg.tool_calls {
                        let calls: Vec<ToolCall> = tool_calls.iter().map(|tc| ToolCall {
                            id: tc.id.clone(),
                            tool_type: "function".to_string(),
                            function: FunctionCall {
                                name: tc.name.clone(),
                                arguments: tc.arguments.clone(),
                            },
                        }).collect();
                        Message::assistant_with_tools(&msg.content, calls)
                    } else {
                        Message::assistant(&msg.content)
                    }
                }
                crate::storage::MessageRole::Tool => {
                    continue;
                }
            };
            messages.push(m);
        }
        
        messages.push(Message::user(user_message));
        
        let mut iteration = 0;
        let mut final_response = String::new();
        
        loop {
            iteration += 1;
            
            if iteration > MAX_TOOL_ITERATIONS {
                return Err(Error::LLM(format!("Max tool iterations ({}) reached", MAX_TOOL_ITERATIONS)));
            }
            
            let tools = self.tool_executor.get_tool_definitions().await;
            
            let response = self.llm_client
                .complete_with_tools(messages.clone(), tools)
                .await?;
            
            let choice = response.choices.first()
                .ok_or_else(|| Error::LLM("No response choice".into()))?;
            
            let assistant_message = &choice.message;
            
            let assistant_content = assistant_message.content.clone();
            if !assistant_content.is_empty() {
                final_response = assistant_content.clone();
            }
            
            let tool_calls = match &assistant_message.tool_calls {
                Some(calls) if !calls.is_empty() => calls,
                _ => {
                    return Ok(final_response);
                }
            };
            
            let message_id = uuid::Uuid::new_v4().to_string();
            let stored_msg = crate::storage::StoredMessage {
                id: message_id.clone(),
                session_id: session_id.to_string(),
                role: crate::storage::MessageRole::Assistant,
                content: assistant_content.clone(),
                timestamp: chrono::Utc::now(),
                tool_calls: Some(tool_calls.iter().map(|tc| {
                    crate::storage::StoredToolCall {
                        id: tc.id.clone(),
                        name: tc.function.name.clone(),
                        arguments: tc.function.arguments.clone(),
                    }
                }).collect()),
            };
            self.storage.save_message(&stored_msg).await?;
            
            messages.push(Message::assistant_with_tools(
                &assistant_content,
                tool_calls.clone(),
            ));
            
            let results = self.tool_executor
                .execute_tool_calls(tool_calls, session_id, &message_id)
                .await;
            
            for (tool_call_id, result) in results {
                let content = match result {
                    Ok(r) => {
                        let stored_result = crate::storage::StoredToolResult {
                            tool_call_id: tool_call_id.clone(),
                            output: r.output.clone(),
                            success: true,
                        };
                        self.storage.save_tool_result(&stored_result).await.ok();
                        r.output
                    }
                    Err(e) => {
                        format!("Error: {}", e)
                    }
                };
                
                messages.push(Message::tool_result(&tool_call_id, &content));
            }
        }
    }
}