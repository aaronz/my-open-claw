use colored::Colorize;
use openclaw_core::AppConfig;
use std::net::TcpStream;
use std::time::Duration;

#[derive(clap::Args)]
pub struct DoctorArgs {}

pub async fn run(_args: DoctorArgs, config: AppConfig) -> anyhow::Result<()> {
    println!("{}", "🦞 OpenClaw Doctor".bold());
    println!("{}", "Running diagnostics...\n".dimmed());

    check_config();
    check_gateway(&config);
    check_channels(&config);

    println!();
    Ok(())
}

fn check_config() {
    let path = AppConfig::default_path();
    if path.exists() {
        println!("{} Config file found at {}", "✅".green(), path.display());
    } else {
        println!(
            "{} Config file not found at {} — run `openclaw onboard`",
            "❌".red(),
            path.display()
        );
    }
}

fn check_gateway(config: &AppConfig) {
    let addr = format!("127.0.0.1:{}", config.gateway.port);
    match TcpStream::connect_timeout(
        &addr.parse().expect("valid addr"),
        Duration::from_secs(2),
    ) {
        Ok(_) => println!("{} Gateway reachable at {}", "✅".green(), addr),
        Err(_) => println!(
            "{} Gateway not reachable at {} — is it running?",
            "❌".red(),
            addr
        ),
    }
}

fn check_channels(config: &AppConfig) {
    let channels = [
        ("telegram", &config.channels.telegram),
        ("discord", &config.channels.discord),
        ("slack", &config.channels.slack),
        ("whatsapp", &config.channels.whatsapp),
        ("signal", &config.channels.signal),
        ("webchat", &config.channels.webchat),
    ];

    for (name, ch_config) in &channels {
        match ch_config {
            Some(ch) if ch.enabled => {
                if ch.token.is_some() {
                    println!("{} {} channel configured", "✅".green(), name);
                } else {
                    println!(
                        "{} {} channel enabled but missing token",
                        "⚠️ ".yellow(),
                        name
                    );
                }
                if matches!(ch.dm_policy, openclaw_core::config::DmPolicy::Open) {
                    println!(
                        "  {} {} has open DM policy — consider using 'pairing'",
                        "⚠️ ".yellow(),
                        name
                    );
                }
            }
            Some(_) => println!("{} {} channel disabled", "➖".dimmed(), name),
            None => {}
        }
    }
}
