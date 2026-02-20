pub mod oauth;

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use serde::Deserialize;
use std::sync::Arc;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct AuthCallback {
    pub code: String,
    pub state: String,
}

pub async fn oauth_callback(
    State(state): State<Arc<AppState>>,
    Query(callback): Query<AuthCallback>,
) -> impl IntoResponse {
    let parts: Vec<&str> = callback.state.split(':').collect();
    if parts.len() != 2 {
        return "Invalid state parameter".into_response();
    }

    let _provider = parts[0];
    let _session_id = parts[1];

    "Authentication successful! You can close this window.".into_response()
}

pub async fn auth_middleware(
    _state: State<Arc<AppState>>,
    _req: axum::extract::Request,
    _next: axum::middleware::Next,
) -> impl IntoResponse {
    _next.run(_req).await
}
