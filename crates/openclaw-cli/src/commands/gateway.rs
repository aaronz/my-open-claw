use openclaw_core::AppConfig;

#[derive(clap::Args)]
pub struct GatewayArgs {
    #[arg(long, default_value = "18789")]
    pub port: u16,
    #[arg(long)]
    pub verbose: bool,
    #[arg(long)]
    pub reset: bool,
}

pub async fn run(args: GatewayArgs, mut config: AppConfig) -> anyhow::Result<()> {
    config.gateway.port = args.port;
    if args.verbose {
        config.gateway.verbose = true;
    }
    if args.reset {
        eprintln!("⚠️  --reset: all sessions will start fresh");
    }
    openclaw_gateway::start_gateway(config).await?;
    Ok(())
}
