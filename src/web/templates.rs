use super::utils::ContextMetrics;
use crate::provider::Provider;
use crate::storage::{StoredMessage, StoredSession, StoredTask};
use askama::Template;

#[derive(Template)]
#[template(path = "layout.html")]
pub struct LayoutTemplate;

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub main_session_id: String,
    pub main_session_id_short: String,
}

#[derive(Template)]
#[template(path = "session_detail.html")]
pub struct SessionDetailTemplate {
    pub session_id: String,
    pub session_id_short: String,
    pub session_purpose: String,
    pub session_state: String,
    pub messages: Vec<MessageView>,
    pub main_session_id: String,
    pub main_session_id_short: String,
    pub main_session_state: String,
    pub queries: Vec<SessionView>,
    pub tasks: Vec<SessionView>,
    pub workers: Vec<SessionView>,
    pub context: ContextMetrics,
    pub active_tasks: Vec<TaskView>,
    pub provider_name: String,
    pub provider_model: String,
}

#[derive(Template)]
#[template(path = "providers.html")]
pub struct ProvidersTemplate {
    pub providers: Vec<ProviderView>,
}

#[derive(Template)]
#[template(path = "provider_form.html")]
pub struct ProviderFormTemplate {
    pub provider: Option<ProviderEditData>,
    pub is_edit: bool,
    pub name: String,
    pub base_url: String,
    pub model: String,
    pub max_tokens: String,
    pub temperature: String,
    pub top_p: String,
    pub top_k: String,
    pub extra_params: String,
}

#[derive(Template)]
#[template(path = "config.html")]
pub struct ConfigTemplate {
    pub llm_base_url: String,
    pub llm_api_key_masked: String,
    pub llm_model: String,
    pub vision_base_url: String,
    pub vision_api_key_masked: String,
    pub vision_model: String,
    pub api_port: u16,
}

pub struct MessageView {
    pub role: String,
    pub content: String,
}

pub struct SessionView {
    pub id: String,
    pub id_short: String,
    pub state: String,
}

pub struct TaskView {
    pub status: String,
    pub content: String,
}

pub struct ProviderView {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key_masked: String,
    pub model: String,
    pub is_active: bool,
}

pub struct ProviderEditData {
    pub id: String,
}

impl MessageView {
    pub fn from_stored(msg: &StoredMessage) -> Self {
        Self {
            role: format!("{:?}", msg.role),
            content: msg.content.clone(),
        }
    }
}

impl SessionView {
    pub fn from_stored(s: &StoredSession) -> Self {
        let id_short = if s.id.len() > 8 {
            s.id[..8].to_string()
        } else {
            s.id.clone()
        };
        Self {
            id: s.id.clone(),
            id_short,
            state: s.state.clone(),
        }
    }
}

impl TaskView {
    pub fn from_stored(t: &StoredTask) -> Self {
        Self {
            status: t.status.to_lowercase(),
            content: t.content.clone(),
        }
    }
}

impl ProviderView {
    pub fn from_provider(p: &Provider, active_id: &str) -> Self {
        let api_key_masked = if p.api_key.len() > 6 {
            format!(
                "{}***{}",
                &p.api_key[..3],
                &p.api_key[p.api_key.len() - 3..]
            )
        } else if p.api_key.len() > 3 {
            format!("{}***", &p.api_key[..3])
        } else {
            "***".to_string()
        };

        Self {
            id: p.id.clone(),
            name: p.name.clone(),
            base_url: p.base_url.clone(),
            api_key_masked,
            model: p.model.clone(),
            is_active: p.id == active_id,
        }
    }
}

pub fn short_id(id: &str) -> String {
    if id.len() > 8 {
        id[..8].to_string()
    } else {
        id.to_string()
    }
}
