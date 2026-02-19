mod commands;
pub(crate) mod ws_client;

use clap::{Parser, Subcommand};
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
    #[command(subcommand)]
    Message(commands::message::MessageCommands),
    Agent(commands::agent::AgentArgs),
    Doctor(commands::doctor::DoctorArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let config_path = cli
        .config
        .unwrap_or_else(AppConfig::default_path);

    let mut config = AppConfig::load(&config_path).unwrap_or_default();
    if cli.verbose {
        config.gateway.verbose = true;
    }

    match cli.command {
        Commands::Gateway(args) => commands::gateway::run(args, config).await,
        Commands::Onboard(args) => commands::onboard::run(args).await,
        Commands::Message(sub) => match sub {
            commands::message::MessageCommands::Send(args) => {
                commands::message::run_send(args, config).await
            }
        },
        Commands::Agent(args) => commands::agent::run(args, config).await,
        Commands::Doctor(args) => commands::doctor::run(args, config).await,
    }
}
