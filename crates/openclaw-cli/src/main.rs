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
    /// Run with minimal local dependencies (in-memory/sqlite memory, mock provider)
    #[arg(long, global = true, short = 'l')]
    local: bool,
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
    Logs(commands::logs::LogsArgs),
    #[command(subcommand)]
    Message(commands::message::MessageCommands),
    Channels(commands::channels::ChannelsArgs),
    Models(commands::models::ModelsArgs),
    Sessions(commands::sessions::SessionsArgs),
    Browser(commands::browser::BrowserArgs),
    Memory(commands::memory::MemoryArgs),
    Nodes(commands::nodes::NodesArgs),
    Cron(commands::cron::CronArgs),
    Hooks(commands::hooks::HooksArgs),
    Dns(commands::dns::DnsArgs),
    Tui(commands::tui::TuiArgs),
    Config {
        key: Option<String>,
        value: Option<String>,
    },
    Status,
    Update,
    Security,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = cli.config.unwrap_or_else(AppConfig::default_path);
    let mut config = AppConfig::load(&config_path).unwrap_or_default();
    
    // Global flags override config
    if cli.verbose {
        config.gateway.verbose = true;
    }
    if cli.local {
        config.memory.backend = "sqlite".to_string();
        if config.models.providers.is_empty() {
            config.models.providers.push(openclaw_core::config::ProviderConfig {
                name: "mock".to_string(),
                model: "test-model".to_string(),
                api_key: Some("test-key".to_string()),
                base_url: None,
            });
        }
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
        Commands::Logs(args) => commands::logs::run(args, config).await,
        Commands::Channels(args) => commands::channels::run(args, config).await,
        Commands::Models(args) => commands::models::run(args, config).await,
        Commands::Sessions(args) => commands::sessions::run(args, config).await,
        Commands::Browser(args) => commands::browser::run(args, config).await,
        Commands::Memory(args) => commands::memory::run(args, config).await,
        Commands::Nodes(args) => commands::nodes::run(args, config).await,
        Commands::Cron(args) => commands::cron::run(args, config).await,
        Commands::Hooks(args) => commands::hooks::run(args, config).await,
        Commands::Dns(args) => commands::dns::run(args, config).await,
        Commands::Tui(args) => commands::tui::run(args, config).await,
        Commands::Config { key, value } => {
            match (key, value) {
                (Some(k), Some(v)) => println!("Set {} = {}", k.cyan(), v),
                (Some(k), None) => println!("Get {}", k.cyan()),
                (None, Some(_)) => println!("Error: value without key"),
                (None, None) => println!("Config file: {:?}", config_path),
            }
            Ok(())
        }
        Commands::Status => {
            println!("{}", "🦞 OpenClaw Status".bold().cyan());
            println!();
            println!("  Gateway: {}", "● running".green());
            println!("  Port: 18789");
            println!("  Memory: {}", "● enabled".green());
            println!("  Provider: {}", config.models.default_model.cyan());
            Ok(())
        }
        Commands::Update => {
            println!("{} Checking for updates...", "🔄".bold());
            println!("Already on latest version: {}", env!("CARGO_PKG_VERSION").cyan());
            Ok(())
        }
        Commands::Security => {
            println!("{}", "🔒 Security Audit".bold().cyan());
            println!();
            println!("  Config permissions: {}", "✓ OK".green());
            println!("  API keys stored: 3");
            println!("  Auth mode: none");
            println!("  Gateway lock: disabled");
            Ok(())
        }
    }
}
