use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,

    // Core parameters (actually used by opencode)
    #[serde(default)]
    pub max_tokens: Option<i32>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub top_p: Option<f64>,
    #[serde(default)]
    pub top_k: Option<i32>,

    // Extra parameters for model-specific options (e.g., reasoning_effort for GLM-5)
    #[serde(default)]
    pub extra_params: Option<serde_json::Value>,

    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub active_provider_id: String,
    pub providers: Vec<Provider>,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            active_provider_id: String::new(),
            providers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CreateProviderRequest {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: Option<i32>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<i32>,
    pub extra_params: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateProviderRequest {
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub max_tokens: Option<i32>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<i32>,
    pub extra_params: Option<serde_json::Value>,
}
