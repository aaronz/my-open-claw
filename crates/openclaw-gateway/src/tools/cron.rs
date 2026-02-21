use async_trait::async_trait;
use chrono::{DateTime, Utc, Duration};
use openclaw_core::provider::ToolDefinition;
use openclaw_core::{Tool, Result as CoreResult};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use crate::cron::{CronScheduler, CronJob};

pub struct CronTool {
    scheduler: Arc<CronScheduler>,
}

impl CronTool {
    pub fn new(scheduler: Arc<CronScheduler>) -> Self {
        Self { scheduler }
    }
}

#[async_trait]
impl Tool for CronTool {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "schedule_task".to_string(),
            description: "Schedule a reminder or a message to be sent at a specific time or on a recurring basis.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["add", "remove", "list"],
                        "description": "The action to perform"
                    },
                    "message": {
                        "type": "string",
                        "description": "The reminder message to send"
                    },
                    "time_rel": {
                        "type": "string",
                        "description": "Relative time (e.g. 'in 5 minutes', 'tomorrow at 9am')"
                    },
                    "cron_expression": {
                        "type": "string",
                        "description": "Standard cron expression (min hour day month dow) for recurring tasks"
                    },
                    "job_id": {
                        "type": "string",
                        "description": "Unique ID for removing a task"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn execute(&self, args: Value) -> CoreResult<String> {
        let action = args["action"].as_str().ok_or_else(|| openclaw_core::OpenClawError::Provider("Missing action".to_string()))?;
        
        let session_id_str = args["_session_id"].as_str().ok_or_else(|| openclaw_core::OpenClawError::Provider("Missing session_id".to_string()))?;
        let session_id = Uuid::parse_str(session_id_str).map_err(|e| openclaw_core::OpenClawError::Provider(e.to_string()))?;

        match action {
            "add" => {
                let message = args["message"].as_str().ok_or_else(|| openclaw_core::OpenClawError::Provider("Missing message".to_string()))?;
                
                let mut target_time = None;
                if let Some(rel) = args["time_rel"].as_str() {
                    if rel.contains("minute") {
                        let mins: i64 = rel.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(1);
                        target_time = Some(Utc::now() + Duration::minutes(mins));
                    } else if rel.contains("hour") {
                        let hours: i64 = rel.split_whitespace().next().and_then(|s| s.parse().ok()).unwrap_or(1);
                        target_time = Some(Utc::now() + Duration::hours(hours));
                    } else {
                        return Err(openclaw_core::OpenClawError::Provider("Unknown time format. Use 'in X minutes' or 'in Y hours'".to_string()));
                    }
                }

                let schedule = args["cron_expression"].as_str().map(|s| s.to_string());
                let id = format!("job-{}", Uuid::new_v4().to_string()[..8].to_string());

                let job = CronJob {
                    id: id.clone(),
                    schedule,
                    target_time,
                    message: message.to_string(),
                    session_id,
                };

                self.scheduler.add_job(job).await;
                Ok(format!("Task scheduled with ID: {}", id))
            }
            "remove" => {
                let id = args["job_id"].as_str().ok_or_else(|| openclaw_core::OpenClawError::Provider("Missing job_id".to_string()))?;
                if self.scheduler.remove_job(id).await {
                    Ok(format!("Task {} removed.", id))
                } else {
                    Ok(format!("Task {} not found.", id))
                }
            }
            "list" => {
                let jobs = self.scheduler.list_jobs().await;
                let filtered: Vec<_> = jobs.into_iter().filter(|j| j.session_id == session_id).collect();
                if filtered.is_empty() {
                    Ok("No scheduled tasks for this session.".to_string())
                } else {
                    let list = filtered.iter().map(|j| format!("- {}: {} ({:?})", j.id, j.message, j.target_time.or(None))).collect::<Vec<_>>().join("\n");
                    Ok(format!("Scheduled tasks:\n{}", list))
                }
            }
            _ => Err(openclaw_core::OpenClawError::Provider(format!("Unknown action: {}", action))),
        }
    }
}
