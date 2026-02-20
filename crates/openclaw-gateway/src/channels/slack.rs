use crate::agent::run_agent_cycle;
use crate::state::AppState;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{Channel, ChannelKind, Result, WsMessage};
use serde_json::{json, Value};
use std::sync::{Arc, Weak};
use tokio::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message as TungsteniteMessage};
use uuid::Uuid;

pub struct SlackChannel {
    bot_token: String,
    app_token: String,
    client: reqwest::Client,
    state: Weak<AppState>,
}

impl SlackChannel {
    pub fn new(bot_token: String, app_token: String, state: Weak<AppState>) -> Self {
        Self {
            bot_token,
            app_token,
            client: reqwest::Client::new(),
            state,
        }
    }
}

#[async_trait]
impl Channel for SlackChannel {
    fn kind(&self) -> ChannelKind {
        ChannelKind::Slack
    }

    fn name(&self) -> &str {
        "slack"
    }

    async fn send_message(&self, peer_id: &str, content: &str) -> Result<()> {
        let url = "https://slack.com/api/chat.postMessage";
        let body = json!({
            "channel": peer_id,
            "text": content
        });

        let res = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .json(&body)
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        let json: Value = res
            .json()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
        if !json["ok"].as_bool().unwrap_or(false) {
            return Err(openclaw_core::OpenClawError::Provider(format!(
                "Slack error: {}",
                json["error"]
            )));
        }
        Ok(())
    }

    async fn send_typing(&self, peer_id: &str) -> Result<()> {
        let url = "https://slack.com/api/chat.postMessage";
        let body = serde_json::json!({
            "channel": peer_id,
            "text": "",
            "username": "OpenClaw",
            "icon_emoji": ":hourglass_flowing_sand:",
            "mrkdwn": false
        });

        let res = self
            .client
            .post(url)
            .header("Authorization", format!("Bearer {}", self.bot_token))
            .json(&body)
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        let json: Value = res
            .json()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
        if !json["ok"].as_bool().unwrap_or(false) {
            return Err(openclaw_core::OpenClawError::Provider(format!(
                "Slack typing error: {}",
                json["error"]
            )));
        }
        Ok(())
    }

    async fn start(&self) -> Result<()> {
        let app_token = self.app_token.clone();
        let client = self.client.clone();
        let state_weak = self.state.clone();
        let kind = self.kind();

        tokio::spawn(async move {
            loop {
                // Get WSS URL
                let res = client
                    .post("https://slack.com/api/apps.connections.open")
                    .header("Authorization", format!("Bearer {}", app_token))
                    .send()
                    .await;

                let url = match res {
                    Ok(r) => {
                        if let Ok(json) = r.json::<Value>().await {
                            if json["ok"].as_bool() == Some(true) {
                                json["url"].as_str().map(|s| s.to_string())
                            } else {
                                tracing::error!("Slack apps.connections.open failed: {:?}", json);
                                None
                            }
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        tracing::error!("Slack API error: {}", e);
                        None
                    }
                };

                if let Some(wss_url) = url {
                    if let Err(e) = connect_slack_socket(&wss_url, &state_weak, kind.clone()).await
                    {
                        tracing::error!("Slack Socket Mode error: {}", e);
                    }
                }

                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

async fn connect_slack_socket(
    url: &str,
    state_weak: &Weak<AppState>,
    kind: ChannelKind,
) -> Result<()> {
    let (ws_stream, _) = connect_async(url)
        .await
        .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        let msg =
            msg.map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
        if let TungsteniteMessage::Text(text) = msg {
            let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);

            // Acknowledge envelope
            if let Some(envelope_id) = v["envelope_id"].as_str() {
                let ack = json!({ "envelope_id": envelope_id });
                write
                    .send(TungsteniteMessage::Text(ack.to_string().into()))
                    .await
                    .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
            }

            if v["type"] == "events_api" {
                let payload = &v["payload"];
                let event = &payload["event"];
                if event["type"] == "message" && event.get("bot_id").is_none() {
                    let text = event["text"].as_str().unwrap_or("");
                    let channel_id = event["channel"].as_str().unwrap_or("");

                    if let Some(state) = state_weak.upgrade() {
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
            }
        }
    }
    Ok(())
}
