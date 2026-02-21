mod commands;
pub(crate) mod ws_client;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;
use openclaw_core::AppConfig;
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "openclaw",
    about = "Personal AI assistant. The lobster way. 🦞",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    #[arg(long, global = true)]
    verbose: bool,
    #[arg(long, global = true)]
    config: Option<PathBuf>,
}

#[derive(Subcommand)]
enum Commands {
    Gateway(commands::gateway::GatewayArgs),
    Onboard(commands::onboard::OnboardArgs),
    Ingest(commands::ingest::IngestArgs),
    Doctor(commands::doctor::DoctorArgs),
    Agent(commands::agent::AgentArgs),
    Dev(commands::gateway::GatewayArgs),
    Listen,
    Plugins(commands::plugins::PluginsArgs),
    #[command(subcommand)]
    Message(commands::message::MessageCommands),
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = cli.config.unwrap_or_else(AppConfig::default_path);
    let mut config = AppConfig::load(&config_path).unwrap_or_default();
    
    if cli.verbose {
        config.gateway.verbose = true;
    }

    match cli.command {
        Commands::Gateway(args) => commands::gateway::run(args, config).await,
        Commands::Onboard(args) => commands::onboard::run(args).await,
        Commands::Ingest(args) => commands::ingest::run(args, config).await,
        Commands::Doctor(args) => commands::doctor::run(args, config).await,
        Commands::Agent(args) => commands::agent::run(args, config).await,
        Commands::Dev(args) => {
            config.models.providers = vec![openclaw_core::config::ProviderConfig {
                name: "mock".to_string(),
                model: "test-model".to_string(),
                api_key: Some("test-key".to_string()),
                base_url: None,
            }];
            config.memory.qdrant_url = "in-memory".to_string();
            config.gateway.verbose = true;
            commands::gateway::run(args, config).await
        }
        Commands::Listen => {
            println!("{}", "🎙️ Listening for wake word 'Lobster'...".bold().yellow());
            println!("{}", "(Voice wake simulation mode)".dimmed());
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            println!("{}", "🔔 Wake word detected!".bold().green());
            
            let args = commands::agent::AgentArgs {
                message: "Hello Lobster, tell me a joke".to_string(),
                thinking: "medium".to_string(),
                voice: true,
            };
            commands::agent::run(args, config).await
        }
        Commands::Plugins(args) => commands::plugins::run(args, config).await,
        Commands::Message(sub) => match sub {
            commands::message::MessageCommands::Send(args) => {
                commands::message::run_send(args, config).await
            }
        },
    }
}
