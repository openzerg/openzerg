use crate::error::{Error, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub agent_name: String,
    pub manager_url: String,
    pub internal_token: String,
    pub workspace: String,
    pub llm_base_url: String,
    pub llm_api_key: String,
    pub llm_model: String,
    pub api_port: u16,

    pub vision_base_url: Option<String>,
    pub vision_api_key: Option<String>,
    pub vision_model: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            agent_name: env::var("AGENT_NAME").unwrap_or_else(|_| "default".to_string()),
            manager_url: env::var("MANAGER_URL")
                .unwrap_or_else(|_| "ws://10.200.1.1:17531".to_string()),
            internal_token: env::var("INTERNAL_TOKEN").expect("INTERNAL_TOKEN must be set"),
            workspace: env::var("WORKSPACE").unwrap_or_else(|_| {
                dirs::home_dir()
                    .map(|h| h.join("workspace").display().to_string())
                    .unwrap_or_else(|| "/workspace".to_string())
            }),
            llm_base_url: env::var("LLM_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".to_string()),
            llm_api_key: env::var("LLM_API_KEY").expect("LLM_API_KEY must be set"),
            llm_model: env::var("LLM_MODEL").unwrap_or_else(|_| "gpt-4o".to_string()),
            api_port: env::var("API_PORT")
                .unwrap_or_else(|_| "8081".to_string())
                .parse()
                .unwrap_or(8081),

            vision_base_url: env::var("VISION_BASE_URL").ok(),
            vision_api_key: env::var("VISION_API_KEY").ok(),
            vision_model: env::var("VISION_MODEL").ok(),
        })
    }

    pub fn vision_enabled(&self) -> bool {
        self.vision_api_key.is_some()
    }

    pub fn workspace_path(&self) -> std::path::PathBuf {
        std::path::PathBuf::from(&self.workspace)
    }

    pub fn openzerg_dir(&self) -> std::path::PathBuf {
        self.workspace_path().join(".openzerg")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> Config {
        Config {
            agent_name: "test-agent".to_string(),
            manager_url: "ws://localhost:17531".to_string(),
            internal_token: "test-token".to_string(),
            workspace: "/tmp/test-workspace".to_string(),
            llm_base_url: "https://api.openai.com/v1".to_string(),
            llm_api_key: "test-key".to_string(),
            llm_model: "gpt-4o".to_string(),
            api_port: 8081,
            vision_base_url: Some("https://api.openai.com/v1".to_string()),
            vision_api_key: Some("vision-key".to_string()),
            vision_model: Some("gpt-4o".to_string()),
        }
    }

    #[test]
    fn test_config_workspace_path() {
        let config = create_test_config();
        assert_eq!(
            config.workspace_path(),
            std::path::PathBuf::from("/tmp/test-workspace")
        );
    }

    #[test]
    fn test_config_openzerg_dir() {
        let config = create_test_config();
        assert_eq!(
            config.openzerg_dir(),
            std::path::PathBuf::from("/tmp/test-workspace/.openzerg")
        );
    }

    #[test]
    fn test_config_vision_enabled_with_key() {
        let config = create_test_config();
        assert!(config.vision_enabled());
    }

    #[test]
    fn test_config_vision_disabled_without_key() {
        let config = Config {
            vision_api_key: None,
            ..create_test_config()
        };
        assert!(!config.vision_enabled());
    }

    #[test]
    fn test_config_default_port() {
        let config = create_test_config();
        assert_eq!(config.api_port, 8081);
    }

    #[test]
    fn test_config_default_model() {
        let config = create_test_config();
        assert_eq!(config.llm_model, "gpt-4o");
    }

    #[test]
    fn test_config_agent_name() {
        let config = create_test_config();
        assert_eq!(config.agent_name, "test-agent");
    }

    #[test]
    fn test_config_manager_url() {
        let config = create_test_config();
        assert_eq!(config.manager_url, "ws://localhost:17531");
    }
}
