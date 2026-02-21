use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub id: String,
    pub path: String,
    pub secret: Option<String>,
    pub agent_id: Option<String>,
    pub channel: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub source: Option<String>,
    pub event: Option<String>,
    pub data: serde_json::Value,
}

pub async fn handle_webhook(
    Path(hook_id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<WebhookPayload>,
) -> Result<StatusCode, StatusCode> {
    tracing::info!("Webhook {} received: {:?}", hook_id, payload);

    let content = if let Some(data_str) = payload.data.as_str() {
        data_str.to_string()
    } else {
        serde_json::to_string(&payload.data).unwrap_or_else(|_| "Webhook received".to_string())
    };

    let source = payload.source.unwrap_or_else(|| "webhook".to_string());
    let peer_id = format!("webhook:{}/{}", hook_id, source);

    let msg = openclaw_core::session::ChatMessage {
        id: uuid::Uuid::new_v4(),
        role: openclaw_core::session::Role::User,
        content,
        images: vec![],
        tool_calls: vec![],
        tool_result: None,
        timestamp: chrono::Utc::now(),
        channel: openclaw_core::ChannelKind::Api,
    };

    let session = state.sessions.get_or_create(openclaw_core::ChannelKind::Api, &peer_id);
    let _ = state.sessions.add_message(&session.id, msg);
    
    tokio::spawn(async move {
        crate::agent::run_agent_cycle(state, session.id).await;
    });

    Ok(StatusCode::OK)
}

pub async fn list_webhooks() -> Json<Vec<WebhookConfig>> {
    Json(vec![
        WebhookConfig {
            id: "default".to_string(),
            path: "/webhook/default".to_string(),
            secret: None,
            agent_id: None,
            channel: None,
            enabled: true,
        }
    ])
}
