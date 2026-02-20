use crate::agent::run_agent_cycle;
use crate::state::AppState;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{Channel, ChannelKind, Result, WsMessage};
use serde_json::{json, Value};
use std::sync::{Arc, Weak};
use tokio::time::{interval, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message as TungsteniteMessage};
use uuid::Uuid;

pub struct DiscordChannel {
    token: String,
    client: reqwest::Client,
    state: Weak<AppState>,
}

impl DiscordChannel {
    pub fn new(token: String, state: Weak<AppState>) -> Self {
        Self {
            token,
            client: reqwest::Client::new(),
            state,
        }
    }
}

#[async_trait]
impl Channel for DiscordChannel {
    fn kind(&self) -> ChannelKind {
        ChannelKind::Discord
    }

    fn name(&self) -> &str {
        "discord"
    }

    async fn send_message(&self, peer_id: &str, content: &str) -> Result<()> {
        let url = format!("https://discord.com/api/v10/channels/{}/messages", peer_id);
        let body = serde_json::json!({
            "content": content
        });

        let res = self
            .client
            .post(&url)
            .header("Authorization", format!("Bot {}", self.token))
            .json(&body)
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(openclaw_core::OpenClawError::Provider(format!(
                "Discord error: {}",
                err
            )));
        }
        Ok(())
    }

    async fn start(&self) -> Result<()> {
        let token = self.token.clone();
        let client = self.client.clone();
        let state_weak = self.state.clone();
        let channel_kind = self.kind();

        tokio::spawn(async move {
            loop {
                if let Err(e) = connect_discord(&token, client.clone(), &state_weak, channel_kind.clone()).await {
                    tracing::error!("Discord Gateway error: {}", e);
                    tokio::time::sleep(Duration::from_secs(5)).await;
                } else {
                    tracing::warn!("Discord Gateway disconnected, reconnecting...");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        });
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }
}

async fn connect_discord(
    token: &str,
    client: reqwest::Client,
    state_weak: &Weak<AppState>,
    kind: ChannelKind,
) -> Result<()> {
    let (ws_stream, _) = connect_async("wss://gateway.discord.gg/?v=10&encoding=json")
        .await
        .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

    let (mut write, mut read) = ws_stream.split();

    // 1. Hello
    let heartbeat_interval;
    if let Some(msg) = read.next().await {
        let msg =
            msg.map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
        if let TungsteniteMessage::Text(text) = msg {
            let v: Value = serde_json::from_str(&text)
                .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
            if v["op"].as_u64() == Some(10) {
                heartbeat_interval = v["d"]["heartbeat_interval"].as_u64().unwrap_or(45000);
            } else {
                return Err(openclaw_core::OpenClawError::Provider(
                    "Expected Hello Opcode 10".to_string(),
                ));
            }
        } else {
            return Err(openclaw_core::OpenClawError::Provider(
                "Expected Text Hello".to_string(),
            ));
        }
    } else {
        return Err(openclaw_core::OpenClawError::Provider(
            "Stream closed during Handshake".to_string(),
        ));
    }

    // 2. Identify
    let identify = json!({
        "op": 2,
        "d": {
            "token": token,
            "intents": 33281, 
            "properties": {
                "os": "linux",
                "browser": "openclaw",
                "device": "openclaw"
            }
        }
    });
    write
        .send(TungsteniteMessage::Text(identify.to_string().into()))
        .await
        .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

    // 3. Heartbeat Loop & Dispatch
    let (ws_tx, mut ws_rx) = tokio::sync::mpsc::channel::<String>(32);

    tokio::spawn(async move {
        let mut interval = interval(Duration::from_millis(heartbeat_interval));
        loop {
            interval.tick().await;
            let hb = json!({ "op": 1, "d": null });
            if ws_tx.send(hb.to_string()).await.is_err() {
                break;
            }
        }
    });

    loop {
        tokio::select! {
            Some(msg_str) = ws_rx.recv() => {
                write.send(TungsteniteMessage::Text(msg_str.into())).await
                    .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
            }
            Some(msg) = read.next() => {
                let msg = msg.map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
                match msg {
                    TungsteniteMessage::Text(text) => {
                        let v: Value = serde_json::from_str(&text).unwrap_or(Value::Null);
                        let op = v["op"].as_u64();

                        if op == Some(0) { // Dispatch
                            let t = v["t"].as_str().unwrap_or("");
                            if t == "MESSAGE_CREATE" {
                                let d = &v["d"];
                                let author = &d["author"];
                                if author["bot"].as_bool().unwrap_or(false) {
                                    continue;
                                }

                                let mut content = d["content"].as_str().unwrap_or("").to_string();
                                let channel_id = d["channel_id"].as_str().unwrap_or("");

                                let mut images = Vec::new();
                                if let Some(attachments) = d["attachments"].as_array() {
                                    for att in attachments {
                                        if let Some(ctype) = att["content_type"].as_str() {
                                            if ctype.starts_with("image/") {
                                                if let Some(url) = att["url"].as_str() {
                                                    images.push(url.to_string());
                                                }
                                            } else if ctype.starts_with("audio/") || ctype == "application/ogg" {
                                                // Handle voice message
                                                if let Some(url) = att["url"].as_str() {
                                                    if let Some(state) = state_weak.upgrade() {
                                                        if let Some(voice) = &state.voice {
                                                            if let Ok(resp) = client.get(url).send().await {
                                                                if let Ok(bytes) = resp.bytes().await {
                                                                    let filename = att["filename"].as_str().unwrap_or("voice.ogg");
                                                                    if let Ok(text) = voice.transcribe(bytes.to_vec(), filename).await {
                                                                        if !content.is_empty() {
                                                                            content.push('\n');
                                                                        }
                                                                        content.push_str(&format!("[Voice Transcription]: {}", text));
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                if let Some(state) = state_weak.upgrade() {
                                    let session =
                                        state.sessions.get_or_create(kind.clone(), channel_id);
                                    let session_id = session.id;
                                    drop(session);

                                    let user_msg = ChatMessage {
                                        id: Uuid::new_v4(),
                                        role: Role::User,
                                        content,
                                        timestamp: chrono::Utc::now(),
                                        channel: kind.clone(),
                                        images,
                                        tool_calls: vec![],
                                        tool_result: None,
                                    };
                                    let _ = state
                                        .sessions
                                        .add_message(&session_id, user_msg.clone());

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
                                } else {
                                    break;
                                }
                            }
                        } else if op == Some(11) {
                            // Heartbeat ACK
                        }
                    }
                    TungsteniteMessage::Close(_) => break,
                    _ => {}
                }
            }
            else => break,
        }
    }

    Ok(())
}
