use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct ImageMessage {
    pub role: String,
    pub content: Vec<ContentPart>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ImageUrl },
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VisionResponse {
    pub id: String,
    pub choices: Vec<VisionChoice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VisionChoice {
    pub index: u32,
    pub message: VisionMessageContent,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VisionMessageContent {
    pub role: String,
    pub content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_message_creation() {
        let msg = ImageMessage {
            role: "user".to_string(),
            content: vec![ContentPart::Text {
                text: "hello".to_string(),
            }],
        };
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content.len(), 1);
    }

    #[test]
    fn test_content_part_text_serialization() {
        let part = ContentPart::Text {
            text: "describe".to_string(),
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("text"));
        assert!(json.contains("describe"));
    }

    #[test]
    fn test_content_part_image_url_serialization() {
        let part = ContentPart::ImageUrl {
            image_url: ImageUrl {
                url: "http://example.com/img.png".to_string(),
            },
        };
        let json = serde_json::to_string(&part).unwrap();
        assert!(json.contains("image_url"));
        assert!(json.contains("example.com"));
    }

    #[test]
    fn test_image_url_serialization() {
        let url = ImageUrl {
            url: "data:image/png;base64,abc".to_string(),
        };
        let json = serde_json::to_string(&url).unwrap();
        assert!(json.contains("base64"));
    }

    #[test]
    fn test_vision_response_deserialization() {
        let json = r#"{"id":"v1","choices":[{"index":0,"message":{"role":"assistant","content":"ok"},"finish_reason":"stop"}]}"#;
        let resp: VisionResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.id, "v1");
        assert_eq!(resp.choices.len(), 1);
    }

    #[test]
    fn test_vision_choice_deserialization() {
        let json =
            r#"{"index":1,"message":{"role":"assistant","content":"test"},"finish_reason":null}"#;
        let choice: VisionChoice = serde_json::from_str(json).unwrap();
        assert_eq!(choice.index, 1);
        assert_eq!(choice.message.content, "test");
    }

    #[test]
    fn test_vision_message_content_deserialization() {
        let json = r#"{"role":"user","content":"hello"}"#;
        let msg: VisionMessageContent = serde_json::from_str(json).unwrap();
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "hello");
    }

    #[test]
    fn test_image_message_serialization() {
        let msg = ImageMessage {
            role: "user".to_string(),
            content: vec![ContentPart::Text {
                text: "what is this?".to_string(),
            }],
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("user"));
        assert!(json.contains("what is this?"));
    }
}
