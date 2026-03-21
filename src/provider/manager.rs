use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::error::Result;
use super::types::{Provider, ProviderConfig, CreateProviderRequest, UpdateProviderRequest};

pub struct ProviderManager {
    config_path: PathBuf,
    config: Arc<RwLock<ProviderConfig>>,
}

impl ProviderManager {
    pub fn new(workspace: PathBuf) -> Self {
        let config_path = workspace.join(".openzerg").join("providers.json");
        let config = Self::load_config(&config_path).unwrap_or_default();
        
        Self {
            config_path,
            config: Arc::new(RwLock::new(config)),
        }
    }
    
    fn load_config(path: &PathBuf) -> Result<ProviderConfig> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(ProviderConfig::default())
        }
    }
    
    async fn save_config(&self) -> Result<()> {
        let config = self.config.read().await;
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(&*config)?;
        std::fs::write(&self.config_path, content)?;
        Ok(())
    }
    
    pub async fn list_providers(&self) -> Vec<Provider> {
        self.config.read().await.providers.clone()
    }
    
    pub async fn get_provider(&self, id: &str) -> Option<Provider> {
        self.config.read().await.providers.iter()
            .find(|p| p.id == id)
            .cloned()
    }
    
    pub async fn get_provider_by_name(&self, name: &str) -> Option<Provider> {
        self.config.read().await.providers.iter()
            .find(|p| p.name == name)
            .cloned()
    }
    
    pub async fn get_active_provider(&self) -> Option<Provider> {
        let config = self.config.read().await;
        config.providers.iter()
            .find(|p| p.id == config.active_provider_id)
            .cloned()
    }
    
    pub async fn create_provider(&self, req: CreateProviderRequest) -> Result<Provider> {
        let mut config = self.config.write().await;
        
        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        
        let provider = Provider {
            id: id.clone(),
            name: req.name,
            base_url: req.base_url,
            api_key: req.api_key,
            model: req.model,
            max_tokens: req.max_tokens,
            temperature: req.temperature,
            top_p: req.top_p,
            top_k: req.top_k,
            extra_params: req.extra_params,
            is_active: config.providers.is_empty(),
            created_at: now.clone(),
            updated_at: now,
        };
        
        if config.providers.is_empty() {
            config.active_provider_id = id.clone();
        }
        
        config.providers.push(provider.clone());
        drop(config);
        
        self.save_config().await?;
        Ok(provider)
    }
    
    pub async fn update_provider(&self, id: &str, req: UpdateProviderRequest) -> Result<Option<Provider>> {
        let mut config = self.config.write().await;
        
        if let Some(provider) = config.providers.iter_mut().find(|p| p.id == id) {
            if let Some(ref v) = req.name { provider.name = v.clone(); }
            if let Some(ref v) = req.base_url { provider.base_url = v.clone(); }
            if let Some(ref v) = req.api_key { provider.api_key = v.clone(); }
            if let Some(ref v) = req.model { provider.model = v.clone(); }
            if let Some(v) = req.max_tokens { provider.max_tokens = Some(v); }
            if let Some(v) = req.temperature { provider.temperature = Some(v); }
            if let Some(v) = req.top_p { provider.top_p = Some(v); }
            if let Some(v) = req.top_k { provider.top_k = Some(v); }
            if let Some(ref v) = req.extra_params { provider.extra_params = Some(v.clone()); }
            provider.updated_at = chrono::Utc::now().to_rfc3339();
            
            let updated = provider.clone();
            drop(config);
            self.save_config().await?;
            return Ok(Some(updated));
        }
        
        Ok(None)
    }
    
    pub async fn delete_provider(&self, id: &str) -> Result<bool> {
        let mut config = self.config.write().await;
        let initial_len = config.providers.len();
        config.providers.retain(|p| p.id != id);
        
        if config.providers.len() < initial_len {
            if config.active_provider_id == id {
                config.active_provider_id = config.providers.first()
                    .map(|p| p.id.clone())
                    .unwrap_or_default();
            }
            drop(config);
            self.save_config().await?;
            return Ok(true);
        }
        
        Ok(false)
    }
    
    pub async fn set_active_provider(&self, id: &str) -> Result<bool> {
        let mut config = self.config.write().await;
        
        if config.providers.iter().any(|p| p.id == id) {
            config.active_provider_id = id.to_string();
            for provider in &mut config.providers {
                provider.is_active = provider.id == id;
            }
            drop(config);
            self.save_config().await?;
            return Ok(true);
        }
        
        Ok(false)
    }
    
    pub async fn set_active_provider_by_name(&self, name: &str) -> Result<bool> {
        let config = self.config.read().await;
        if let Some(provider) = config.providers.iter().find(|p| p.name == name) {
            let id = provider.id.clone();
            drop(config);
            return self.set_active_provider(&id).await;
        }
        Ok(false)
    }
}