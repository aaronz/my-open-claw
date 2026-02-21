pub mod agent;
pub mod auth;
pub mod channels;
pub mod cron;
pub mod mcp;
pub mod memory;
pub mod provider;
pub mod routes;
pub mod skills;
pub mod state;
pub mod tools;
pub mod voice;
pub mod ws;

use crate::channels::discord::DiscordChannel;
use crate::channels::feishu::FeishuChannel;
use crate::channels::matrix::MatrixChannel;
use crate::channels::mattermost::MattermostChannel;
use crate::channels::signal::SignalChannel;
use crate::channels::slack::SlackChannel;
use crate::channels::telegram::TelegramChannel;
use crate::channels::whatsapp::WhatsAppChannel;
use axum::middleware;
use axum::routing::get;
use axum::Router;
use openclaw_core::config::{AuthMode, BindMode};
use openclaw_core::{AppConfig, Channel, ChannelKind};
use state::AppState;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;
use opentelemetry::global;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace as sdktrace;
use opentelemetry_sdk::Resource;
use opentelemetry::KeyValue;

pub fn init_telemetry(config: &AppConfig) {
    if config.diagnostics.otel.enabled {
        let exporter = opentelemetry_otlp::new_exporter()
            .http()
            .with_endpoint(&config.diagnostics.otel.endpoint);
        
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(exporter)
            .with_trace_config(
                sdktrace::config().with_resource(Resource::new(vec![KeyValue::new(
                    "service.name",
                    "openclaw-gateway",
                )])),
            )
            .install_batch(opentelemetry_sdk::runtime::Tokio)
            .expect("Failed to initialize OTLP tracer");

        let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        
        let subscriber = tracing_subscriber::Registry::default().with(telemetry_layer);
        tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

    } else if config.gateway.verbose {
        tracing_subscriber::fmt()
            .with_env_filter("openclaw=debug,tower_http=debug")
            .with_ansi(true)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter("openclaw=info")
            .with_ansi(true)
            .init();
    }
}

pub async fn start_gateway(config: AppConfig) -> openclaw_core::Result<()> {
    init_telemetry(&config);

    info!("Logging to /tmp/openclaw/");

    let port = config.gateway.port;
    let bind_addr = match config.gateway.bind {
        BindMode::Loopback => "127.0.0.1",
        BindMode::All => "0.0.0.0",
    };

    let needs_auth = !matches!(config.gateway.auth.mode, AuthMode::None);
    let state = AppState::new(config).await;

    // Start Cron Scheduler
    state.cron.clone().start(state.clone());

    // Initialize Channels
    if let Some(telegram_config) = &state.config.channels.telegram {
        if telegram_config.enabled {
            if let Some(token) = &telegram_config.token {
                let channel = TelegramChannel::new(token.clone(), Arc::downgrade(&state));
                match channel.start().await {
                    Ok(_) => {
                        info!("Telegram channel started");
                        state
                            .channels
                            .insert(ChannelKind::Telegram, Arc::new(channel));
                    }
                    Err(e) => {
                        tracing::error!("Failed to start Telegram channel: {}", e);
                    }
                }
            }
        }
    }

    if let Some(discord_config) = &state.config.channels.discord {
        if discord_config.enabled {
            if let Some(token) = &discord_config.token {
                let channel = DiscordChannel::new(token.clone(), Arc::downgrade(&state));
                match channel.start().await {
                    Ok(_) => {
                        info!("Discord channel started");
                        state
                            .channels
                            .insert(ChannelKind::Discord, Arc::new(channel));
                    }
                    Err(e) => {
                        tracing::error!("Failed to start Discord channel: {}", e);
                    }
                }
            }
        }
    }

    if let Some(slack_config) = &state.config.channels.slack {
        if slack_config.enabled {
            if let (Some(token), Some(app_token)) =
                (&slack_config.token, &slack_config.app_token)
            {
                let channel = SlackChannel::new(
                    token.clone(),
                    app_token.clone(),
                    Arc::downgrade(&state),
                );
                match channel.start().await {
                    Ok(_) => {
                        info!("Slack channel started");
                        state.channels.insert(ChannelKind::Slack, Arc::new(channel));
                    }
                    Err(e) => {
                        tracing::error!("Failed to start Slack channel: {}", e);
                    }
                }
            }
        }
    }

    if let Some(whatsapp_config) = &state.config.channels.whatsapp {
        if whatsapp_config.enabled {
            if let (Some(token), Some(phone_id)) =
                (&whatsapp_config.token, &whatsapp_config.app_token)
            {
                let channel = WhatsAppChannel::new(
                    token.clone(),
                    phone_id.clone(),
                    Arc::downgrade(&state),
                );
                match channel.start().await {
                    Ok(_) => {
                        info!("WhatsApp channel started");
                        state
                            .channels
                            .insert(ChannelKind::WhatsApp, Arc::new(channel));
                    }
                    Err(e) => {
                        tracing::error!("Failed to start WhatsApp channel: {}", e);
                    }
                }
            }
        }
    }

    if let Some(signal_config) = &state.config.channels.signal {
        if signal_config.enabled {
            if let Some(url) = &signal_config.token {
                let channel = SignalChannel::new(url.clone(), Arc::downgrade(&state));
                match channel.start().await {
                    Ok(_) => {
                        info!("Signal channel started");
                        state.channels.insert(ChannelKind::Signal, Arc::new(channel));
                    }
                    Err(e) => {
                        tracing::error!("Failed to start Signal channel: {}", e);
                    }
                }
            }
        }
    }

    if let Some(matrix_config) = &state.config.channels.matrix {
        if matrix_config.enabled {
            if let (Some(homeserver), Some(token)) =
                (&matrix_config.token, &matrix_config.app_token)
            {
                let channel = MatrixChannel::new(
                    homeserver.clone(),
                    token.clone(),
                    Arc::downgrade(&state),
                );
                match channel.start().await {
                    Ok(_) => {
                        info!("Matrix channel started");
                        state.channels.insert(ChannelKind::Matrix, Arc::new(channel));
                    }
                    Err(e) => {
                        tracing::error!("Failed to start Matrix channel: {}", e);
                    }
                }
            }
        }
    }

    if let Some(feishu_config) = &state.config.channels.telegram {
        if feishu_config.enabled && feishu_config.dm_policy == openclaw_core::config::DmPolicy::Open {
             if let (Some(app_id), Some(app_secret)) = (&feishu_config.token, &feishu_config.app_token) {
                let channel = FeishuChannel::new(app_id.clone(), app_secret.clone(), Arc::downgrade(&state));
                info!("Feishu channel started");
                state.channels.insert(ChannelKind::Api, Arc::new(channel));
             }
        }
    }

    if let Some(mm_config) = &state.config.channels.slack {
        if mm_config.enabled && mm_config.dm_policy == openclaw_core::config::DmPolicy::Open {
             if let (Some(url), Some(token)) = (&mm_config.token, &mm_config.app_token) {
                let channel = MattermostChannel::new(url.clone(), token.clone(), Arc::downgrade(&state));
                info!("Mattermost channel started");
                state.channels.insert(ChannelKind::Api, Arc::new(channel));
             }
        }
    }

    let mut app = Router::new()
        .route("/ws", get(ws::ws_handler))
        .merge(routes::api_router());

    if needs_auth {
        app = app.layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ));
    }

    let app = app
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("{bind_addr}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(openclaw_core::OpenClawError::Io)?;

    info!("🦞 OpenClaw Gateway v{} listening on ws://{}", env!("CARGO_PKG_VERSION"), addr);

    axum::serve(listener, app)
        .await
        .map_err(openclaw_core::OpenClawError::Io)?;

    Ok(())
}
