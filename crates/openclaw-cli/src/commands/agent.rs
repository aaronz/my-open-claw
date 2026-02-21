use colored::Colorize;
use openclaw_core::{AppConfig, WsMessage};

use crate::ws_client;

#[derive(clap::Args)]
pub struct AgentArgs {
    #[arg(long)]
    pub message: String,
    #[arg(long, default_value = "medium")]
    pub thinking: String,
    #[arg(long)]
    pub voice: bool,
}

pub async fn run(args: AgentArgs, config: AppConfig) -> anyhow::Result<()> {
    let model = &config.models.default_model;
    println!(
        "{} Using model: {}  thinking: {}",
        "🦞".to_string().bold(),
        model.cyan(),
        args.thinking.yellow()
    );
    println!("{} {}", "You:".bold(), args.message);

    let url = format!("ws://127.0.0.1:{}/ws", config.gateway.port);
    let msg = WsMessage::SendMessage {
        session_id: None,
        content: args.message,
        channel: None,
        peer_id: Some("cli".to_string()),
    };

    let mut first_token = true;
    ws_client::send_and_receive(
        &url,
        msg,
        || {
            print!("{}", "\n[Agent] Thinking...".dimmed());
        },
        |token| {
            if first_token {
                print!("\r{} ", "[Agent]".bold().green());
                first_token = false;
            }
            print!("{token}");
        },
    )
    .await?;

    println!();
    Ok(())
}
