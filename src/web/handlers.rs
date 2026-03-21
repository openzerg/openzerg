use axum::{
    extract::{Path, State, Form},
    response::{Html, Redirect},
};
use askama::Template;
use std::sync::Arc;
use serde::Deserialize;
use crate::api_server::ApiState;
use crate::storage::StoredSession;
use super::templates::*;
use super::utils::{mask_api_key, calculate_context};

#[derive(Deserialize)]
pub struct SendMessageForm {
    pub content: String,
}

#[derive(Deserialize)]
pub struct ProviderForm {
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: Option<i32>,
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub top_k: Option<i32>,
    pub extra_params: Option<String>,
}

#[derive(Deserialize)]
pub struct ConfigForm {
    pub llm_base_url: Option<String>,
    pub llm_api_key: Option<String>,
    pub llm_model: Option<String>,
    pub vision_base_url: Option<String>,
    pub vision_api_key: Option<String>,
    pub vision_model: Option<String>,
    pub api_port: Option<u16>,
}

pub async fn dashboard(
    State(state): State<Arc<ApiState>>,
) -> Html<String> {
    let sessions = state.storage.load_visible_sessions().await.unwrap_or_default();
    let main_session = sessions.iter().find(|s| s.purpose == "Main");
    let active_count = sessions.iter().filter(|s| 
        s.state != "Completed" && s.state != "Failed" && s.state != "Cancelled"
    ).count();
    
    let main_session_id = main_session.map(|s| s.id.clone()).unwrap_or_default();
    let main_session_id_short = short_id(&main_session_id);
    
    let template = DashboardTemplate {
        total_sessions: sessions.len(),
        active_sessions: active_count,
        main_session_id,
        main_session_id_short,
    };
    
    Html(template.render().unwrap_or_default())
}

pub async fn session_detail(
    Path(id): Path<String>,
    State(state): State<Arc<ApiState>>,
) -> Html<String> {
    let sessions = state.storage.load_visible_sessions().await.unwrap_or_default();
    let session = sessions.iter().find(|s| s.id == id);
    
    let session_purpose = session.map(|s| s.purpose.as_str()).unwrap_or("Unknown").to_string();
    let session_state = session.map(|s| s.state.as_str()).unwrap_or("Unknown").to_string();
    let session_id_short = short_id(&id);
    let system_prompt = session.map(|s| s.system_prompt.clone()).unwrap_or_default();
    
    let stored_messages = state.storage.load_messages(Some(&id)).await.unwrap_or_default();
    let messages: Vec<_> = stored_messages.iter().map(MessageView::from_stored).collect();
    
    let main_session = sessions.iter().find(|s| s.purpose == "Main");
    let main_session_id = main_session.map(|s| s.id.clone()).unwrap_or_default();
    let main_session_id_short = short_id(&main_session_id);
    let main_session_state = main_session.map(|s| s.state.as_str()).unwrap_or("Unknown").to_string();
    
    let queries: Vec<_> = sessions.iter()
        .filter(|s| s.purpose == "Query")
        .map(SessionView::from_stored)
        .collect();
    let dispatchers: Vec<_> = sessions.iter()
        .filter(|s| s.purpose == "Dispatcher")
        .map(SessionView::from_stored)
        .collect();
    let tasks: Vec<_> = sessions.iter()
        .filter(|s| s.purpose == "Task")
        .map(SessionView::from_stored)
        .collect();
    let workers: Vec<_> = sessions.iter()
        .filter(|s| s.purpose == "Worker")
        .map(SessionView::from_stored)
        .collect();
    
    let context = calculate_context(&stored_messages, Some(&system_prompt));
    
    let tasks_storage = state.storage.load_tasks().await.unwrap_or_default();
    let active_tasks: Vec<_> = tasks_storage.iter()
        .filter(|t| t.status == "InProgress" || t.status == "Pending")
        .map(TaskView::from_stored)
        .collect();
    
    let template = SessionDetailTemplate {
        session_id: id,
        session_id_short,
        session_purpose,
        session_state,
        system_prompt,
        messages,
        main_session_id,
        main_session_id_short,
        main_session_state,
        dispatchers,
        queries,
        tasks,
        workers,
        context,
        active_tasks,
    };
    
    Html(template.render().unwrap_or_default())
}

pub async fn send_message(
    Path(id): Path<String>,
    State(state): State<Arc<ApiState>>,
    Form(form): Form<SendMessageForm>,
) -> Redirect {
    let event = crate::protocol::AgentEvent::Message {
        content: form.content,
        from: "user".to_string(),
    };
    let _ = state.event_tx.send(event);
    
    Redirect::to(&format!("/ui/sessions/{}", id))
}

pub async fn providers_list(
    State(state): State<Arc<ApiState>>,
) -> Html<String> {
    let manager = crate::provider::ProviderManager::new(
        state.storage.base_path().parent().unwrap_or(state.storage.base_path()).to_path_buf()
    );
    
    let providers = manager.list_providers().await;
    let active_provider_id = manager.get_active_provider().await.map(|p| p.id).unwrap_or_default();
    
    let provider_views: Vec<_> = providers.iter()
        .map(|p| ProviderView::from_provider(p, &active_provider_id))
        .collect();
    
    let template = ProvidersTemplate {
        providers: provider_views,
    };
    
    Html(template.render().unwrap_or_default())
}

pub async fn provider_new() -> Html<String> {
    let template = ProviderFormTemplate {
        provider: None,
        is_edit: false,
        name: String::new(),
        base_url: String::new(),
        model: String::new(),
        max_tokens: String::new(),
        temperature: String::new(),
        top_p: String::new(),
        top_k: String::new(),
        extra_params: String::new(),
    };
    
    Html(template.render().unwrap_or_default())
}

pub async fn provider_edit(
    Path(id): Path<String>,
    State(state): State<Arc<ApiState>>,
) -> Html<String> {
    let manager = crate::provider::ProviderManager::new(
        state.storage.base_path().parent().unwrap_or(state.storage.base_path()).to_path_buf()
    );
    
    let provider = match manager.get_provider(&id).await {
        Some(p) => Some(p),
        None => manager.get_provider_by_name(&id).await,
    };
    
    let template = if let Some(p) = &provider {
        ProviderFormTemplate {
            provider: Some(ProviderEditData { id: p.id.clone() }),
            is_edit: true,
            name: p.name.clone(),
            base_url: p.base_url.clone(),
            model: p.model.clone(),
            max_tokens: p.max_tokens.map(|v| v.to_string()).unwrap_or_default(),
            temperature: p.temperature.map(|v| v.to_string()).unwrap_or_default(),
            top_p: p.top_p.map(|v| v.to_string()).unwrap_or_default(),
            top_k: p.top_k.map(|v| v.to_string()).unwrap_or_default(),
            extra_params: p.extra_params.as_ref().map(|v| v.to_string()).unwrap_or_default(),
        }
    } else {
        ProviderFormTemplate {
            provider: None,
            is_edit: false,
            name: String::new(),
            base_url: String::new(),
            model: String::new(),
            max_tokens: String::new(),
            temperature: String::new(),
            top_p: String::new(),
            top_k: String::new(),
            extra_params: String::new(),
        }
    };
    
    Html(template.render().unwrap_or_default())
}

pub async fn provider_create(
    State(state): State<Arc<ApiState>>,
    Form(form): Form<ProviderForm>,
) -> Redirect {
    let manager = crate::provider::ProviderManager::new(
        state.storage.base_path().parent().unwrap_or(state.storage.base_path()).to_path_buf()
    );
    
    let extra_params = form.extra_params.as_ref()
        .and_then(|s| serde_json::from_str(s).ok());
    
    let req = crate::provider::CreateProviderRequest {
        name: form.name,
        base_url: form.base_url,
        api_key: form.api_key,
        model: form.model,
        max_tokens: form.max_tokens,
        temperature: form.temperature,
        top_p: form.top_p,
        top_k: form.top_k,
        extra_params,
    };
    
    let _ = manager.create_provider(req).await;
    
    Redirect::to("/ui/providers")
}

pub async fn provider_update(
    Path(id): Path<String>,
    State(state): State<Arc<ApiState>>,
    Form(form): Form<ProviderForm>,
) -> Redirect {
    let manager = crate::provider::ProviderManager::new(
        state.storage.base_path().parent().unwrap_or(state.storage.base_path()).to_path_buf()
    );
    
    let extra_params = form.extra_params.as_ref()
        .and_then(|s| serde_json::from_str(s).ok());
    
    let req = crate::provider::UpdateProviderRequest {
        name: Some(form.name),
        base_url: Some(form.base_url),
        api_key: if form.api_key.is_empty() { None } else { Some(form.api_key) },
        model: Some(form.model),
        max_tokens: form.max_tokens,
        temperature: form.temperature,
        top_p: form.top_p,
        top_k: form.top_k,
        extra_params,
    };
    
    let _ = manager.update_provider(&id, req).await;
    
    Redirect::to("/ui/providers")
}

pub async fn provider_delete(
    Path(id): Path<String>,
    State(state): State<Arc<ApiState>>,
) -> Redirect {
    let manager = crate::provider::ProviderManager::new(
        state.storage.base_path().parent().unwrap_or(state.storage.base_path()).to_path_buf()
    );
    
    let _ = manager.delete_provider(&id).await;
    
    Redirect::to("/ui/providers")
}

pub async fn config_page(
    State(state): State<Arc<ApiState>>,
) -> Html<String> {
    let config = state.config.read();
    
    let llm_api_key_masked = if config.llm_api_key().is_empty() {
        "(not set)".to_string()
    } else {
        mask_api_key(config.llm_api_key())
    };
    
    let vision_api_key_masked = config.runtime.vision.api_key.as_deref()
        .map(mask_api_key)
        .unwrap_or_else(|| "(not set)".to_string());
    
    let template = ConfigTemplate {
        llm_base_url: config.llm_base_url().to_string(),
        llm_api_key_masked,
        llm_model: config.llm_model().to_string(),
        vision_base_url: config.runtime.vision.base_url.clone().unwrap_or_default(),
        vision_api_key_masked,
        vision_model: config.runtime.vision.model.clone().unwrap_or_default(),
        api_port: config.api_port(),
    };
    
    Html(template.render().unwrap_or_default())
}

pub async fn config_update(
    State(state): State<Arc<ApiState>>,
    Form(form): Form<ConfigForm>,
) -> Redirect {
    let mut config = state.config.write();
    
    if let Some(v) = form.llm_base_url {
        config.runtime.llm.base_url = v;
    }
    if let Some(v) = form.llm_api_key {
        if !v.is_empty() {
            config.runtime.llm.api_key = v;
        }
    }
    if let Some(v) = form.llm_model {
        config.runtime.llm.model = v;
    }
    if let Some(v) = form.vision_base_url {
        config.runtime.vision.base_url = Some(v);
    }
    if let Some(v) = form.vision_api_key {
        config.runtime.vision.api_key = Some(v);
    }
    if let Some(v) = form.vision_model {
        config.runtime.vision.model = Some(v);
    }
    if let Some(v) = form.api_port {
        config.runtime.api_port = v;
    }
    
    let _ = config.save_runtime_config();
    
    Redirect::to("/ui/config")
}