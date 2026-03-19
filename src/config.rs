use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    #[serde(default = "default_llm_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_llm_model")]
    pub model: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            base_url: default_llm_base_url(),
            api_key: String::new(),
            model: default_llm_model(),
        }
    }
}

fn default_llm_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_llm_model() -> String {
    "gpt-4o".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VisionConfig {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub vision: VisionConfig,
    #[serde(default = "default_api_port")]
    pub api_port: u16,
}

fn default_api_port() -> u16 {
    8081
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            llm: LlmConfig::default(),
            vision: VisionConfig::default(),
            api_port: default_api_port(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub agent_name: String,
    pub manager_url: String,
    pub internal_token: String,
    pub workspace: String,
    pub runtime: RuntimeConfig,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let workspace = env::var("WORKSPACE").unwrap_or_else(|_| {
            dirs::home_dir()
                .map(|h| h.join("workspace").display().to_string())
                .unwrap_or_else(|| "/workspace".to_string())
        });

        let mut config = Self {
            agent_name: env::var("AGENT_NAME").unwrap_or_else(|_| "default".to_string()),
            manager_url: env::var("MANAGER_URL")
                .unwrap_or_else(|_| "ws://10.200.1.1:17531".to_string()),
            internal_token: env::var("INTERNAL_TOKEN").expect("INTERNAL_TOKEN must be set"),
            workspace: workspace.clone(),
            runtime: RuntimeConfig::default(),
        };

        config.load_runtime_config()?;
        config.apply_env_overrides();

        Ok(config)
    }

    fn load_runtime_config(&mut self) -> Result<()> {
        let config_path = self.openzerg_dir().join("config.yaml");

        if config_path.exists() {
            tracing::info!("Loading runtime config from {:?}", config_path);
            let content = std::fs::read_to_string(&config_path)?;
            if let Ok(runtime) = serde_yaml::from_str::<RuntimeConfig>(&content) {
                self.runtime = runtime;
            }
        }

        Ok(())
    }

    fn apply_env_overrides(&mut self) {
        if let Ok(v) = env::var("LLM_BASE_URL") {
            self.runtime.llm.base_url = v;
        }
        if let Ok(v) = env::var("LLM_API_KEY") {
            self.runtime.llm.api_key = v;
        }
        if let Ok(v) = env::var("LLM_MODEL") {
            self.runtime.llm.model = v;
        }
        if let Ok(v) = env::var("API_PORT") {
            if let Ok(port) = v.parse() {
                self.runtime.api_port = port;
            }
        }
        if let Ok(v) = env::var("VISION_BASE_URL") {
            self.runtime.vision.base_url = Some(v);
        }
        if let Ok(v) = env::var("VISION_API_KEY") {
            self.runtime.vision.api_key = Some(v);
        }
        if let Ok(v) = env::var("VISION_MODEL") {
            self.runtime.vision.model = Some(v);
        }
    }

    pub fn save_runtime_config(&self) -> Result<()> {
        let dir = self.openzerg_dir();
        std::fs::create_dir_all(&dir)?;

        let config_path = dir.join("config.yaml");
        let content = serde_yaml::to_string(&self.runtime)?;
        std::fs::write(&config_path, content)?;

        tracing::info!("Saved runtime config to {:?}", config_path);
        Ok(())
    }

    pub fn update_llm_config(
        &mut self,
        base_url: Option<String>,
        api_key: Option<String>,
        model: Option<String>,
    ) {
        if let Some(v) = base_url {
            self.runtime.llm.base_url = v;
        }
        if let Some(v) = api_key {
            self.runtime.llm.api_key = v;
        }
        if let Some(v) = model {
            self.runtime.llm.model = v;
        }
    }

    pub fn vision_enabled(&self) -> bool {
        self.runtime.vision.api_key.is_some()
    }

    pub fn workspace_path(&self) -> PathBuf {
        PathBuf::from(&self.workspace)
    }

    pub fn openzerg_dir(&self) -> PathBuf {
        self.workspace_path().join(".openzerg")
    }

    pub fn llm_base_url(&self) -> &str {
        &self.runtime.llm.base_url
    }

    pub fn llm_api_key(&self) -> &str {
        &self.runtime.llm.api_key
    }

    pub fn llm_model(&self) -> &str {
        &self.runtime.llm.model
    }

    pub fn api_port(&self) -> u16 {
        self.runtime.api_port
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
            runtime: RuntimeConfig {
                llm: LlmConfig {
                    base_url: "https://api.openai.com/v1".to_string(),
                    api_key: "test-key".to_string(),
                    model: "gpt-4o".to_string(),
                },
                vision: VisionConfig {
                    base_url: Some("https://api.openai.com/v1".to_string()),
                    api_key: Some("vision-key".to_string()),
                    model: Some("gpt-4o".to_string()),
                },
                api_port: 8081,
            },
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
        let mut config = create_test_config();
        config.runtime.vision.api_key = None;
        assert!(!config.vision_enabled());
    }

    #[test]
    fn test_config_default_port() {
        let config = create_test_config();
        assert_eq!(config.api_port(), 8081);
    }

    #[test]
    fn test_config_default_model() {
        let config = create_test_config();
        assert_eq!(config.llm_model(), "gpt-4o");
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

    #[test]
    fn test_llm_config_defaults() {
        let llm = LlmConfig::default();
        assert_eq!(llm.base_url, "https://api.openai.com/v1");
        assert_eq!(llm.model, "gpt-4o");
        assert!(llm.api_key.is_empty());
    }

    #[test]
    fn test_runtime_config_defaults() {
        let runtime = RuntimeConfig::default();
        assert_eq!(runtime.api_port, 8081);
        assert_eq!(runtime.llm.model, "gpt-4o");
    }
}
