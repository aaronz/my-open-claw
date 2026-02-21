pub mod oauth;
pub mod pairing;

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
    State(_state): State<Arc<AppState>>,
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
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> impl IntoResponse {
    next.run(req).await
}
