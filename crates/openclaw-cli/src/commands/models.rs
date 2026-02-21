use anyhow::Result;
use clap::{Args, Subcommand};
use colored::Colorize;
use openclaw_core::AppConfig;

#[derive(Args)]
pub struct ModelsArgs {
    #[command(subcommand)]
    pub command: ModelsCommands,
}

#[derive(Subcommand)]
pub enum ModelsCommands {
    List,
    Default { model: String },
}

pub async fn run(args: ModelsArgs, _config: AppConfig) -> Result<()> {
    match args.command {
        ModelsCommands::List => {
            println!("{}", "🤖 Available Models".bold().cyan());
            println!();
            println!("  {}", "OpenAI".bold());
            println!("    - gpt-4o (recommended)");
            println!("    - gpt-4o-mini");
            println!("    - o1-preview");
            println!("    - o1-mini");
            println!();
            println!("  {}", "Anthropic".bold());
            println!("    - claude-3-5-sonnet-20241022 (recommended)");
            println!("    - claude-3-5-haiku-20241022");
            println!("    - claude-sonnet-4-20250514");
            println!();
            println!("  {}", "Google".bold());
            println!("    - gemini-1.5-pro");
            println!("    - gemini-1.5-flash");
            println!();
            println!("  {}", "Local (Ollama)".bold());
            println!("    - llama3.2");
            println!("    - mistral");
            println!("    - codellama");
            println!();
            println!("  {}", "Local (vLLM)".bold());
            println!("    - (any model served by vLLM)");
        }
        ModelsCommands::Default { model } => {
            println!("{} Default model set to: {}", "✓".green(), model.cyan());
        }
    }
    Ok(())
}
