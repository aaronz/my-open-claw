use crate::agent::run_agent_cycle;
use crate::state::AppState;
use async_trait::async_trait;
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{Channel, ChannelKind, Result, WsMessage};
use serde_json::json;
use std::sync::{Arc, Weak};
use uuid::Uuid;

pub struct FeishuChannel {
    _state: Weak<AppState>,
    app_id: String,
    app_secret: String,
    client: reqwest::Client,
}

impl FeishuChannel {
    pub fn new(app_id: String, app_secret: String, state: Weak<AppState>) -> Self {
        Self {
            _state: state,
            app_id,
            app_secret,
            client: reqwest::Client::new(),
        }
    }

    async fn get_tenant_token(&self) -> Result<String, String> {
        let url = "https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal";
        let body = json!({
            "app_id": self.app_id,
            "app_secret": self.app_secret
        });

        let res = self.client.post(url).json(&body).send().await.map_err(|e| e.to_string())?;
        let json: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
        
        json["tenant_access_token"].as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "Failed to get tenant token".to_string())
    }
}

#[async_trait]
impl Channel for FeishuChannel {
    fn kind(&self) -> ChannelKind {
        ChannelKind::Api
    }

    fn name(&self) -> &str {
        "feishu"
    }

    async fn send_message(&self, peer_id: &str, content: &str) -> Result<()> {
        let token = self.get_tenant_token().await.map_err(|e| openclaw_core::OpenClawError::Provider(e))?;
        let url = "https://open.feishu.cn/open-apis/im/v1/messages?receive_id_type=open_id";
        
        let body = json!({
            "receive_id": peer_id,
            "msg_type": "text",
            "content": json!({ "text": content }).to_string()
        });

        let _ = self.client.post(url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await;

        Ok(())
    }

    async fn start(&self) -> Result<()> {
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

pub async fn handle_feishu_webhook(state: Arc<AppState>, body: serde_json::Value) -> Result<()> {
    if let Some(challenge) = body["challenge"].as_str() {
        return Ok(());
    }

    if let Some(event) = body.get("event") {
        if let Some(message) = event.get("message") {
            let open_id = event["sender"]["sender_id"]["open_id"].as_str().unwrap_or_default();
            let text_raw = message["content"].as_str().unwrap_or_default();
            
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text_raw) {
                let text = parsed["text"].as_str().unwrap_or_default();
                
                if !open_id.is_empty() && !text.is_empty() {
                    let kind = ChannelKind::Api;
                    let session = state.sessions.get_or_create(kind.clone(), open_id);
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
