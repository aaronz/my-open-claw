use crate::state::AppState;
use std::sync::Arc;
use tokio::process::Command;

pub async fn init_tailscale(state: Arc<AppState>) {
    if !state.config.agent.network.tailscale.enabled {
        return;
    }

    let ts_up = Command::new("tailscale")
        .arg("up")
        .output()
        .await;

    if ts_up.is_err() {
        tracing::error!("Failed to start Tailscale. Is it installed?");
        return;
    }

    tracing::info!("Tailscale is running.");
}
