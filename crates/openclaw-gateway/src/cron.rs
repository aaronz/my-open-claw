use chrono::{DateTime, Utc};
use openclaw_core::session::{ChatMessage, Role};
use openclaw_core::{ChannelKind, WsMessage};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use uuid::Uuid;

use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct CronJob {
    pub id: String,
    pub schedule: Option<String>,
    pub target_time: Option<DateTime<Utc>>,
    pub message: String,
    pub session_id: Uuid,
}

pub struct CronScheduler {
    jobs: Arc<Mutex<Vec<CronJob>>>,
}

impl CronScheduler {
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn add_job(&self, job: CronJob) {
        self.jobs.lock().await.push(job);
    }

    pub async fn remove_job(&self, id: &str) -> bool {
        let mut jobs = self.jobs.lock().await;
        let len_before = jobs.len();
        jobs.retain(|j| j.id != id);
        jobs.len() < len_before
    }

    pub async fn list_jobs(&self) -> Vec<CronJob> {
        self.jobs.lock().await.clone()
    }

    pub fn start(self: Arc<Self>, state: Arc<AppState>) {
        let scheduler = self;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                let now = Utc::now();
                let mut jobs_to_run = Vec::new();
                let mut jobs_to_remove = Vec::new();

                {
                    let jobs = scheduler.jobs.lock().await;
                    for job in jobs.iter() {
                        if let Some(target) = job.target_time {
                            if target <= now {
                                jobs_to_run.push(job.clone());
                                jobs_to_remove.push(job.id.clone());
                            }
                        } else if let Some(schedule) = &job.schedule {
                            if should_run_now(schedule) {
                                jobs_to_run.push(job.clone());
                            }
                        }
                    }
                }

                // Remove one-offs
                if !jobs_to_remove.is_empty() {
                    let mut jobs = scheduler.jobs.lock().await;
                    jobs.retain(|j| !jobs_to_remove.contains(&j.id));
                }

                // Execute
                for job in jobs_to_run {
                    info!(job_id = %job.id, "reminder triggered");
                    
                    if let Some(session) = state.sessions.get(&job.session_id) {
                        let session_clone = session.clone();
                        drop(session); // Release lock
                        
                        let msg = ChatMessage {
                            id: Uuid::new_v4(),
                            role: Role::System,
                            content: format!("REMINDER: {}", job.message),
                            timestamp: Utc::now(),
                            channel: session_clone.channel.clone(),
                            images: vec![],
                            tool_calls: vec![],
                            tool_result: None,
                        };
                        
                        let _ = state.sessions.add_message(&job.session_id, msg.clone());
                        
                        // Notify via WS/Channel
                        let ws_msg = WsMessage::NewMessage {
                            session_id: job.session_id,
                            message: msg.clone(),
                        };
                        if let Ok(json) = serde_json::to_string(&ws_msg) {
                            state.send_to_subscribers(&job.session_id, &json);
                        }
                        
                        // Notify via outbound channel (Telegram/Discord/Slack)
                        if let Some(chan) = state.channels.get(&session_clone.channel) {
                            let peer_id = session_clone.peer_id.clone();
                            let content = msg.content.clone();
                            let chan_ref = chan.value().clone();
                            tokio::spawn(async move {
                                let _ = chan_ref.send_message(&peer_id, &content).await;
                            });
                        }
                    }
                }
            }
        });
    }
}

fn should_run_now(schedule: &str) -> bool {
    let parts: Vec<&str> = schedule.split_whitespace().collect();
    if parts.len() != 5 {
        return false;
    }

    let now = chrono::Local::now();
    let checks = [
        (parts[0], now.format("%M").to_string()),
        (parts[1], now.format("%H").to_string()),
        (parts[2], now.format("%d").to_string()),
        (parts[3], now.format("%m").to_string()),
        (parts[4], now.format("%u").to_string()),
    ];

    checks.iter().all(|(pattern, value)| {
        *pattern == "*" || *pattern == value.as_str()
    })
}
