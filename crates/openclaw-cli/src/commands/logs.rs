use anyhow::Result;
use clap::Args;
use openclaw_core::AppConfig;
use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::fs::File;
use std::path::Path;

#[derive(Args)]
pub struct LogsArgs {
    /// Follow log output
    #[arg(long, short)]
    pub follow: bool,
}

pub async fn run(args: LogsArgs, _config: AppConfig) -> Result<()> {
    let log_file = format!("/tmp/openclaw/openclaw-{}.log", chrono::Local::now().format("%Y-%m-%d"));
    let path = Path::new(&log_file);

    if !path.exists() {
        println!("Log file not found: {}", log_file);
        return Ok(());
    }

    let file = File::open(path).await?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line).await?;
        if bytes_read == 0 {
            if args.follow {
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                continue;
            } else {
                break;
            }
        }
        print!("{}", line);
    }

    Ok(())
}
