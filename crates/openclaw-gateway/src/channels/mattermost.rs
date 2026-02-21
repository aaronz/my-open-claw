use crate::agent::run_agent_cycle;
use crate::state::AppState;
use async_trait::async_trait;
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{Channel, ChannelKind, Result, WsMessage};
use serde_json::json;
use std::sync::{Arc, Weak};
use uuid::Uuid;

pub struct MattermostChannel {
    state: Weak<AppState>,
    api_url: String,
    token: String,
    client: reqwest::Client,
}

impl MattermostChannel {
    pub fn new(api_url: String, token: String, state: Weak<AppState>) -> Self {
        Self {
            state,
            api_url,
            token,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Channel for MattermostChannel {
    fn kind(&self) -> ChannelKind {
        ChannelKind::Api
    }

    fn name(&self) -> &str {
        "mattermost"
    }

    async fn send_message(&self, peer_id: &str, content: &str) -> Result<()> {
        let url = format!("{}/api/v4/posts", self.api_url);
        let body = json!({
            "channel_id": peer_id,
            "message": content
        });

        let _ = self.client.post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
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

pub async fn handle_mattermost_webhook(state: Arc<AppState>, body: serde_json::Value) -> Result<()> {
    if let Some(post) = body.get("post") {
        let channel_id = post["channel_id"].as_str().unwrap_or_default();
        let text = post["message"].as_str().unwrap_or_default();
        let user_id = post["user_id"].as_str().unwrap_or_default();

        if !channel_id.is_empty() && !text.is_empty() {
            let kind = ChannelKind::Api;
            let session = state.sessions.get_or_create(kind.clone(), channel_id);
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
    Ok(())
}
