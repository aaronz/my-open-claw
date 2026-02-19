use colored::Colorize;
use openclaw_core::{AppConfig, WsMessage};

use crate::ws_client;

#[derive(clap::Subcommand)]
pub enum MessageCommands {
    Send(SendArgs),
}

#[derive(clap::Args)]
pub struct SendArgs {
    #[arg(long)]
    pub to: String,
    #[arg(long)]
    pub message: String,
    #[arg(long)]
    pub channel: Option<String>,
}

pub async fn run_send(args: SendArgs, config: AppConfig) -> anyhow::Result<()> {
    let channel = args.channel.as_deref().unwrap_or("api");
    let port = config.gateway.port;
    println!(
        "{} Sending to {} via {} (gateway ws://127.0.0.1:{}/ws)",
        "→".blue(),
        args.to.bold(),
        channel.cyan(),
        port
    );

    let url = format!("ws://127.0.0.1:{}/ws", port);
    let msg = WsMessage::SendMessage {
        session_id: None,
        content: args.message.clone(),
        channel: None,
        peer_id: Some(args.to.clone()),
    };

    ws_client::send_and_receive(
        &url,
        msg,
        || {
            print!("{}", "Waiting for response...".dimmed());
        },
        |token| {
            print!("{token}");
        },
    )
    .await?;

    println!();
    Ok(())
}
