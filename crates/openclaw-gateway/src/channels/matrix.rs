use crate::agent::run_agent_cycle;
use crate::state::AppState;
use async_trait::async_trait;
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{Channel, ChannelKind, Result, WsMessage};
use serde_json::json;
use std::sync::{Arc, Weak};
use uuid::Uuid;

pub struct MatrixChannel {
    state: Weak<AppState>,
    homeserver: String,
    access_token: String,
    client: reqwest::Client,
}

impl MatrixChannel {
    pub fn new(homeserver: String, access_token: String, state: Weak<AppState>) -> Self {
        Self {
            state,
            homeserver,
            access_token,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Channel for MatrixChannel {
    fn kind(&self) -> ChannelKind {
        ChannelKind::Matrix 
    }

    fn name(&self) -> &str {
        "matrix"
    }

    async fn send_message(&self, peer_id: &str, content: &str) -> Result<()> {
        let url = format!(
            "{}/_matrix/client/v3/rooms/{}/send/m.room.message/{}",
            self.homeserver,
            peer_id,
            Uuid::new_v4()
        );
        
        let body = json!({
            "msgtype": "m.text",
            "body": content
        });

        let _ = self.client.put(&url)
            .header("Authorization", format!("Bearer {}", self.access_token))
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

pub async fn handle_matrix_webhook(state: Arc<AppState>, body: serde_json::Value) -> Result<()> {
    // Simplified Matrix event handling (e.g. from a push gateway or bot hook)
    if let Some(room_id) = body["room_id"].as_str() {
        if let Some(content) = body["content"]["body"].as_str() {
            let sender = body["sender"].as_str().unwrap_or("unknown");
            
            // Avoid responding to ourselves
            if sender.contains("@openclaw:") {
                return Ok(());
            }

            let kind = ChannelKind::Matrix;
            let session = state.sessions.get_or_create(kind.clone(), room_id);
            let session_id = session.id;
            drop(session);

            let user_msg = ChatMessage {
                id: Uuid::new_v4(),
                role: Role::User,
                content: content.to_string(),
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
