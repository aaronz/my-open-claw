use async_trait::async_trait;
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{Channel, ChannelKind, Result, WsMessage};
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Weak};
use tokio::time::Duration;
use tracing::{error, info};
use uuid::Uuid;

use crate::agent::run_agent_cycle;
use crate::state::AppState;

pub struct TelegramChannel {
    token: String,
    client: reqwest::Client,
    state: Weak<AppState>,
    running: AtomicBool,
}

impl TelegramChannel {
    pub fn new(token: String, state: Weak<AppState>) -> Self {
        Self {
            token,
            client: reqwest::Client::new(),
            state,
            running: AtomicBool::new(false),
        }
    }
}

#[derive(Deserialize)]
struct UpdateResponse {
    result: Vec<Update>,
}

#[derive(Deserialize)]
struct Update {
    update_id: u64,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    chat: Chat,
    text: Option<String>,
    voice: Option<Voice>,
    audio: Option<Audio>,
}

#[derive(Deserialize)]
struct Voice {
    file_id: String,
}

#[derive(Deserialize)]
struct Audio {
    file_id: String,
}

#[derive(Deserialize)]
struct Chat {
    id: i64,
}

#[derive(Deserialize)]
struct FileResponse {
    result: FileInfo,
}

#[derive(Deserialize)]
struct FileInfo {
    file_path: String,
}

#[async_trait]
impl Channel for TelegramChannel {
    fn kind(&self) -> ChannelKind {
        ChannelKind::Telegram
    }

    fn name(&self) -> &str {
        "telegram"
    }

    async fn send_message(&self, peer_id: &str, content: &str) -> Result<()> {
        let url = format!("https://api.telegram.org/bot{}/sendMessage", self.token);
        let params = [("chat_id", peer_id), ("text", content)];
        
        let res = self.client.post(&url).form(&params).send().await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;
            
        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(openclaw_core::OpenClawError::Provider(format!("Telegram error: {}", err)));
        }
        Ok(())
    }

    async fn send_voice(&self, peer_id: &str, audio: Vec<u8>) -> Result<()> {
        let url = format!("https://api.telegram.org/bot{}/sendVoice", self.token);
        let part = reqwest::multipart::Part::bytes(audio).file_name("voice.ogg");
        let form = reqwest::multipart::Form::new()
            .text("chat_id", peer_id.to_string())
            .part("voice", part);

        let res = self
            .client
            .post(&url)
            .multipart(form)
            .send()
            .await
            .map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        if !res.status().is_success() {
            let err = res.text().await.unwrap_or_default();
            return Err(openclaw_core::OpenClawError::Provider(format!(
                "Telegram voice error: {}",
                err
            )));
        }
        Ok(())
    }

    async fn start(&self) -> Result<()> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Ok(()); // Already running
        }

        let token = self.token.clone();
        let client = self.client.clone();
        let state_weak = self.state.clone();
        let channel_kind = self.kind();

        info!("Starting Telegram polling...");

        tokio::spawn(async move {
            let mut offset = 0;
            
            while let Some(state) = state_weak.upgrade() {
                let url = format!("https://api.telegram.org/bot{}/getUpdates", token);
                let params = [
                    ("offset", offset.to_string()),
                    ("timeout", "30".to_string()),
                ];

                match client.post(&url).form(&params).send().await {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            if let Ok(updates) = resp.json::<UpdateResponse>().await {
                                for update in updates.result {
                                    offset = update.update_id + 1;
                                    
                                    if let Some(msg) = update.message {
                                        let mut content = msg.text.unwrap_or_default();
                                        
                                        // Handle Voice
                                        if let Some(file_id) = msg.voice.map(|v| v.file_id).or(msg.audio.map(|a| a.file_id)) {
                                            if let Some(voice_service) = &state.voice {
                                                let file_url = format!("https://api.telegram.org/bot{}/getFile?file_id={}", token, file_id);
                                                if let Ok(file_resp) = client.get(&file_url).send().await {
                                                    if let Ok(file_json) = file_resp.json::<FileResponse>().await {
                                                        let download_url = format!("https://api.telegram.org/file/bot{}/{}", token, file_json.result.file_path);
                                                        if let Ok(dl_resp) = client.get(&download_url).send().await {
                                                            if let Ok(bytes) = dl_resp.bytes().await {
                                                                // Telegram OGG or MP3
                                                                if let Ok(text) = voice_service.transcribe(bytes.to_vec(), "voice.ogg").await {
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

                                        if content.is_empty() {
                                            continue;
                                        }

                                        let peer_id = msg.chat.id.to_string();
                                        
                                        let session = state.sessions.get_or_create(channel_kind.clone(), &peer_id);
                                        let session_id = session.id;
                                        drop(session); 

                                        let user_msg = ChatMessage {
                                            id: Uuid::new_v4(),
                                            role: Role::User,
                                            content: content.clone(),
                                            timestamp: chrono::Utc::now(),
                                            channel: channel_kind.clone(),
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
                        } else {
                            error!("Telegram polling error: status {}", resp.status());
                            tokio::time::sleep(Duration::from_secs(5)).await;
                        }
                    }
                    Err(e) => {
                        error!("Telegram polling error: {}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
            info!("Telegram polling stopped (state dropped)");
        });

        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }
}
