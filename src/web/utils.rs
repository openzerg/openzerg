pub fn mask_api_key(key: &str) -> String {
    if key.len() > 6 {
        format!("{}***{}", &key[..3], &key[key.len() - 3..])
    } else if key.len() > 3 {
        format!("{}***", &key[..3])
    } else {
        "***".to_string()
    }
}

pub struct ContextMetrics {
    pub total_tokens: u64,
    pub usage_percent: u8,
    pub message_count: usize,
}

impl Default for ContextMetrics {
    fn default() -> Self {
        Self {
            total_tokens: 0,
            usage_percent: 0,
            message_count: 0,
        }
    }
}

pub fn calculate_context(messages: &[crate::storage::StoredMessage]) -> ContextMetrics {
    let message_count = messages.len();

    let total_chars: usize = messages.iter().map(|m| m.content.len()).sum();
    let total_tokens = (total_chars as f64 / 4.0) as u64;

    let max_context = 128_000u64;
    let usage_percent = if total_tokens > 0 {
        ((total_tokens as f64 / max_context as f64) * 100.0).min(100.0) as u8
    } else {
        0
    };

    ContextMetrics {
        total_tokens,
        usage_percent,
        message_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_api_key_long() {
        assert_eq!(mask_api_key("sk-sp-abc123xyz"), "sk-***xyz");
    }

    #[test]
    fn test_mask_api_key_medium() {
        assert_eq!(mask_api_key("sk-abc"), "sk-***");
    }

    #[test]
    fn test_mask_api_key_short() {
        assert_eq!(mask_api_key("sk"), "***");
    }

    #[test]
    fn test_calculate_context_empty() {
        let messages = vec![];
        let ctx = calculate_context(&messages);
        assert_eq!(ctx.total_tokens, 0);
        assert_eq!(ctx.usage_percent, 0);
        assert_eq!(ctx.message_count, 0);
    }

    #[test]
    fn test_calculate_context_with_messages() {
        use crate::storage::{MessageRole, StoredMessage};
        use chrono::Utc;

        let messages = vec![StoredMessage {
            id: "1".to_string(),
            session_id: "s1".to_string(),
            role: MessageRole::User,
            content: "Hello world".to_string(),
            timestamp: Utc::now(),
            tool_calls: None,
        }];

        let ctx = calculate_context(&messages);
        assert_eq!(ctx.message_count, 1);
        assert!(ctx.total_tokens > 0);
    }
}
