use colored::Colorize;
use dialoguer::{Input, Select};
use openclaw_core::config::{ModelsConfig, ProviderConfig};
use openclaw_core::AppConfig;

#[derive(clap::Args)]
pub struct OnboardArgs {
    #[arg(long)]
    pub install_daemon: bool,
}

pub async fn run(args: OnboardArgs) -> anyhow::Result<()> {
    println!("{}", "🦞 Welcome to OpenClaw!".bold().cyan());
    println!("{}", "Let's set up your personal AI assistant.\n".dimmed());

    let providers = &["Anthropic (Claude)", "OpenAI (GPT)"];
    let provider_idx = Select::new()
        .with_prompt("Choose your AI provider")
        .items(providers)
        .default(0)
        .interact()?;

    let (provider_name, default_model) = match provider_idx {
        0 => ("anthropic", "claude-sonnet-4-20250514"),
        1 => ("openai", "gpt-4o"),
        _ => unreachable!(),
    };

    let api_key: String = Input::new()
        .with_prompt(format!("Enter your {} API key", providers[provider_idx]))
        .interact_text()?;

    let model: String = Input::new()
        .with_prompt("Default model")
        .default(default_model.to_string())
        .interact_text()?;

    let port: u16 = Input::new()
        .with_prompt("Gateway port")
        .default(18789)
        .interact_text()?;

    let enable_mcp = Select::new()
        .with_prompt("Enable Model Context Protocol (MCP) servers?")
        .items(&["No", "Yes (Enable Brave Search & Postgres presets)"])
        .default(0)
        .interact()?;

    let mut config = AppConfig::default();
    config.gateway.port = port;

    if enable_mcp == 1 {
        config.agent.mcp_servers = vec![
            openclaw_core::config::McpServerConfig {
                name: "brave-search".to_string(),
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "@modelcontextprotocol/server-brave-search".to_string()],
            },
            openclaw_core::config::McpServerConfig {
                name: "postgres".to_string(),
                command: "npx".to_string(),
                args: vec!["-y".to_string(), "@modelcontextprotocol/server-postgres".to_string()],
            },
        ];
    }
    config.models = ModelsConfig {
        default_model: model.clone(),
        providers: vec![ProviderConfig {
            name: provider_name.to_string(),
            model,
            api_key: Some(api_key),
            base_url: None,
        }],
    };

    let config_path = AppConfig::default_path();
    config.save(&config_path)?;
    println!(
        "\n{} Config saved to {}",
        "✅".green(),
        config_path.display()
    );

    if args.install_daemon {
        println!("\n{}", "Daemon installation:".bold());
        #[cfg(target_os = "macos")]
        println!("  Create a launchd plist at ~/Library/LaunchAgents/ai.openclaw.gateway.plist");
        #[cfg(target_os = "linux")]
        println!("  Create a systemd user service at ~/.config/systemd/user/openclaw-gateway.service");
        println!(
            "  {}",
            "(daemon auto-install not yet implemented — run `openclaw gateway` manually)".dimmed()
        );
    }

    println!("\n{}", "🦞 Setup complete! Run `openclaw gateway` to start.".bold().green());

    let pair_mobile = Select::new()
        .with_prompt("Pair with mobile device (Node)?")
        .items(&["No", "Yes (Show pairing QR code)"])
        .default(0)
        .interact()?;

    if pair_mobile == 1 {
        let pairing_data = format!("openclaw://pair?gateway=http://localhost:{}", port);
        println!("\n{}", "Scan this code with the OpenClaw mobile app:".bold());
        qr2term::print_qr(&pairing_data).map_err(|e| anyhow::anyhow!(e))?;
    }

    Ok(())
}
