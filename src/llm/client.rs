use reqwest::Client;
use tokio_stream::StreamExt;
use futures_util::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::Deserialize;
use crate::error::{Error, Result};
use super::types::{Message, ChatCompletionRequest, ChatCompletionResponse, ChatCompletionResponse as Response};

struct LLMConfig {
    base_url: String,
    api_key: String,
    model: String,
}

pub struct LLMClient {
    client: Client,
    config: Arc<RwLock<LLMConfig>>,
}

impl LLMClient {
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        let client = Client::builder()
            .user_agent("openzerg/0.5.0")
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            client,
            config: Arc::new(RwLock::new(LLMConfig { base_url, api_key, model })),
        }
    }

    pub async fn update_config(&self, base_url: Option<String>, api_key: Option<String>, model: Option<String>) {
        let mut config = self.config.write().await;
        if let Some(url) = base_url {
            config.base_url = url;
        }
        if let Some(key) = api_key {
            config.api_key = key;
        }
        if let Some(m) = model {
            config.model = m;
        }
    }

    pub async fn complete(&self, messages: Vec<Message>) -> Result<String> {
        let response = self.send_request(messages, false).await?;
        
        response.choices.first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| Error::LLM("No response from LLM".into()))
    }

    pub async fn complete_with_tools(
        &self,
        messages: Vec<Message>,
        tools: Vec<super::types::ToolDefinition>,
    ) -> Result<ChatCompletionResponse> {
        let config = self.config.read().await;
        let request = ChatCompletionRequest {
            model: config.model.clone(),
            messages,
            tools: Some(tools),
            stream: None,
        };

        let url = format!("{}/chat/completions", config.base_url);
        let api_key = config.api_key.clone();
        drop(config);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::LLM(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::LLM(format!("LLM API error: {} - {}", status, body)));
        }

        response.json().await
            .map_err(|e| Error::LLM(format!("Failed to parse response: {}", e)))
    }

    pub fn stream(&self, messages: Vec<Message>) -> impl Stream<Item = Result<StreamChunk>> {
        let config = self.config.clone();
        let client = self.client.clone();

        async_stream::try_stream! {
            let (url, api_key, model) = {
                let cfg = config.read().await;
                (
                    format!("{}/chat/completions", cfg.base_url),
                    cfg.api_key.clone(),
                    cfg.model.clone()
                )
            };

            let request = ChatCompletionRequest {
                model,
                messages,
                tools: None,
                stream: Some(true),
            };

            let response = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
                .map_err(|e| Error::LLM(format!("Request failed: {}", e)))?;

            let mut stream = response.bytes_stream();
            
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|e| Error::LLM(format!("Stream error: {}", e)))?;
                let text = String::from_utf8_lossy(&chunk);
                
                for line in text.lines() {
                    if line.starts_with("data: ") {
                        let data = &line[6..];
                        if data == "[DONE]" {
                            yield StreamChunk::Done;
                            return;
                        }
                        
                        if let Ok(response) = serde_json::from_str::<StreamResponse>(data) {
                            if let Some(choice) = response.choices.first() {
                                if let Some(delta) = &choice.delta {
                                    if let Some(content) = &delta.content {
                                        yield StreamChunk::Content(content.clone());
                                    }
                                    if let Some(tool_calls) = &delta.tool_calls {
                                        yield StreamChunk::ToolCalls(tool_calls.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    async fn send_request(&self, messages: Vec<Message>, stream: bool) -> Result<ChatCompletionResponse> {
        let config = self.config.read().await;
        let request = ChatCompletionRequest {
            model: config.model.clone(),
            messages,
            tools: None,
            stream: if stream { Some(true) } else { None },
        };

        let url = format!("{}/chat/completions", config.base_url);
        let api_key = config.api_key.clone();
        drop(config);
        
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::LLM(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::LLM(format!("LLM API error: {} - {}", status, body)));
        }

        response.json().await
            .map_err(|e| Error::LLM(format!("Failed to parse response: {}", e)))
    }
}

#[derive(Debug, Clone)]
pub enum StreamChunk {
    Content(String),
    ToolCalls(Vec<super::types::ToolCall>),
    Done,
}

#[derive(Debug, Clone, Deserialize)]
struct StreamResponse {
    choices: Vec<StreamChoice>,
}

#[derive(Debug, Clone, Deserialize)]
struct StreamChoice {
    delta: Option<StreamDelta>,
    finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct StreamDelta {
    content: Option<String>,
    tool_calls: Option<Vec<super::types::ToolCall>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_client_new() {
        let client = LLMClient::new(
            "http://localhost".to_string(),
            "test-key".to_string(),
            "gpt-4".to_string(),
        );
        // Client created successfully
        assert!(true);
    }

    #[test]
    fn test_stream_chunk_content() {
        let chunk = StreamChunk::Content("hello".to_string());
        match chunk {
            StreamChunk::Content(s) => assert_eq!(s, "hello"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_stream_chunk_tool_calls() {
        use crate::llm::{ToolCall, FunctionCall};
        let tool_calls = vec![ToolCall {
            id: "call-1".to_string(),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: "test".to_string(),
                arguments: "{}".to_string(),
            },
        }];
        let chunk = StreamChunk::ToolCalls(tool_calls.clone());
        match chunk {
            StreamChunk::ToolCalls(calls) => assert_eq!(calls.len(), 1),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_stream_chunk_done() {
        let chunk = StreamChunk::Done;
        match chunk {
            StreamChunk::Done => {}
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_stream_response_deserialization() {
        let json = r#"{"choices":[{"delta":{"content":"hi"},"finish_reason":null}]}"#;
        let resp: StreamResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 1);
    }

    #[test]
    fn test_stream_choice_deserialization() {
        let json = r#"{"delta":{"content":"test"},"finish_reason":"stop"}"#;
        let choice: StreamChoice = serde_json::from_str(json).unwrap();
        assert!(choice.delta.is_some());
        assert_eq!(choice.finish_reason, Some("stop".to_string()));
    }

    #[test]
    fn test_stream_delta_deserialization() {
        let json = r#"{"content":"hello"}"#;
        let delta: StreamDelta = serde_json::from_str(json).unwrap();
        assert_eq!(delta.content, Some("hello".to_string()));
        assert!(delta.tool_calls.is_none());
    }

    #[test]
    fn test_stream_delta_with_tool_calls() {
        let json = r#"{"content":null,"tool_calls":[{"id":"1","type":"function","function":{"name":"test","arguments":"{}"}}]}"#;
        let delta: StreamDelta = serde_json::from_str(json).unwrap();
        assert!(delta.tool_calls.is_some());
    }
}