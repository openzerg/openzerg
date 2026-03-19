use crate::error::{Error, Result};
use super::types::{ImageMessage, ContentPart, ImageUrl, VisionResponse};

pub struct VisionClient {
    base_url: String,
    api_key: String,
    model: String,
}

impl VisionClient {
    pub fn new(base_url: String, api_key: String, model: String) -> Self {
        Self { base_url, api_key, model }
    }
    
    pub async fn analyze_image(&self, image_url: &str, prompt: &str) -> Result<String> {
        let message = ImageMessage {
            role: "user".to_string(),
            content: vec![
                ContentPart::ImageUrl {
                    image_url: ImageUrl {
                        url: image_url.to_string(),
                    },
                },
                ContentPart::Text {
                    text: prompt.to_string(),
                },
            ],
        };
        
        let request = serde_json::json!({
            "model": self.model,
            "messages": [message],
        });
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| Error::LLM(format!("Failed to build client: {}", e)))?;
        
        let response = client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::LLM(format!("Vision request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::LLM(format!("Vision API error ({}): {}", status, body)));
        }
        
        let vision_response: VisionResponse = response
            .json()
            .await
            .map_err(|e| Error::LLM(format!("Failed to parse vision response: {}", e)))?;
        
        Ok(vision_response.choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }
    
    pub async fn analyze_images(&self, image_urls: &[&str], prompt: &str) -> Result<String> {
        let mut content: Vec<ContentPart> = image_urls
            .iter()
            .map(|url| ContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: url.to_string(),
                },
            })
            .collect();
        
        content.push(ContentPart::Text {
            text: prompt.to_string(),
        });
        
        let message = ImageMessage {
            role: "user".to_string(),
            content,
        };
        
        let request = serde_json::json!({
            "model": self.model,
            "messages": [message],
        });
        
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| Error::LLM(format!("Failed to build client: {}", e)))?;
        
        let response = client
            .post(&format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::LLM(format!("Vision request failed: {}", e)))?;
        
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::LLM(format!("Vision API error ({}): {}", status, body)));
        }
        
        let vision_response: VisionResponse = response
            .json()
            .await
            .map_err(|e| Error::LLM(format!("Failed to parse vision response: {}", e)))?;
        
        Ok(vision_response.choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vision_client_new() {
        let client = VisionClient::new(
            "http://localhost".to_string(),
            "test-key".to_string(),
            "gpt-4o".to_string(),
        );
        assert!(true);
    }

    #[test]
    fn test_image_message_creation() {
        let message = ImageMessage {
            role: "user".to_string(),
            content: vec![
                ContentPart::Text { text: "What is this?".to_string() },
            ],
        };
        assert_eq!(message.role, "user");
        assert_eq!(message.content.len(), 1);
    }

    #[test]
    fn test_content_part_text() {
        let part = ContentPart::Text { text: "hello".to_string() };
        match part {
            ContentPart::Text { text } => assert_eq!(text, "hello"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_content_part_image_url() {
        let part = ContentPart::ImageUrl {
            image_url: ImageUrl { url: "http://example.com/img.png".to_string() },
        };
        match part {
            ContentPart::ImageUrl { image_url } => assert_eq!(image_url.url, "http://example.com/img.png"),
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_image_url_creation() {
        let url = ImageUrl { url: "data:image/png;base64,abc".to_string() };
        assert_eq!(url.url, "data:image/png;base64,abc");
    }
}