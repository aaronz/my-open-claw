use crate::agent::run_agent_cycle;
use crate::state::AppState;
use async_trait::async_trait;
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{Channel, ChannelKind, Result, WsMessage};
use serde_json::{json, Value};
use std::sync::{Arc, Weak};
use uuid::Uuid;

pub struct WhatsAppChannel {
    token: String,
    phone_number_id: String,
    client: reqwest::Client,
    _state: Weak<AppState>,
}

impl WhatsAppChannel {
    pub fn new(token: String, phone_number_id: String, state: Weak<AppState>) -> Self {
        Self {
            token,
            phone_number_id,
            client: reqwest::Client::new(),
            _state: state,
        }
    }
}

#[async_trait]
impl Channel for WhatsAppChannel {
    fn kind(&self) -> ChannelKind {
        ChannelKind::WhatsApp
    }

    fn name(&self) -> &str {
        "whatsapp"
    }

    async fn send_message(&self, peer_id: &str, content: &str) -> Result<()> {
        let url = format!("https://graph.facebook.com/v21.0/{}/messages", self.phone_number_id);
        let body = json!({
            "messaging_product": "whatsapp",
            "recipient_type": "individual",
            "to": peer_id,
            "type": "text",
            "text": { "body": content }
        });

        let res = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&body)
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(openclaw_core::OpenClawError::Provider(format!("WhatsApp error: {}", err)));
        }
        Ok(())
    }

    async fn start(&self) -> Result<()> {
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

pub async fn handle_whatsapp_webhook(state: Arc<AppState>, body: Value) -> Result<()> {
    if let Some(entry) = body["entry"].as_array().and_then(|a| a.first()) {
        if let Some(change) = entry["changes"].as_array().and_then(|a| a.first()) {
            let value = &change["value"];
            if let Some(message) = value["messages"].as_array().and_then(|a| a.first()) {
                let from = message["from"].as_str().unwrap_or_default();
                let text = message["text"]["body"].as_str().unwrap_or_default();
                
                if !from.is_empty() && !text.is_empty() {
                    let kind = ChannelKind::WhatsApp;
                    let session = state.sessions.get_or_create(kind.clone(), from);
                    let session_id = session.id;
                    drop(session);

                    let user_msg = ChatMessage {
                        id: Uuid::new_v4(),
                        role: Role::User,
                        content: text.to_string(),
                        timestamp: chrono::Utc::now(),
                        channel: kind.clone(),
                        images: vec![],
                        tool_calls: vec![],
                        tool_result: None,
                    };
                    let _ = state.sessions.add_message(&session_id, user_msg.clone());

                    let new_msg = WsMessage::NewMessage {
                        session_id,
                        message: user_msg,
                    };
                    if let Ok(json) = serde_json::to_string(&new_msg) {
                        state.broadcast(&json);
                    }

                    let spawn_state = Arc::clone(&state);
                    tokio::spawn(async move {
                        run_agent_cycle(spawn_state, session_id).await;
                    });
                }
            }
        }
    }
    Ok(())
}
