use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use crate::state::AppState;

#[derive(Debug, Clone)]
pub struct CronJob {
    pub id: String,
    pub schedule: String,
    pub message: String,
    pub peer_id: String,
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
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                let jobs = scheduler.jobs.lock().await.clone();
                for job in &jobs {
                    if should_run_now(&job.schedule) {
                        info!(job_id = %job.id, "cron job triggered");
                        let msg = openclaw_core::WsMessage::SendMessage {
                            session_id: None,
                            content: job.message.clone(),
                            channel: None,
                            peer_id: Some(job.peer_id.clone()),
                        };
                        if let Ok(json) = serde_json::to_string(&msg) {
                            state.broadcast(&json);
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
