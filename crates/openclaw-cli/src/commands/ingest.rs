use anyhow::Result;
use clap::Args;
use openclaw_core::AppConfig;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Args)]
pub struct IngestArgs {
    /// Directory or file to ingest
    path: PathBuf,
}

pub async fn run(args: IngestArgs, config: AppConfig) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/memory", config.gateway.port);

    println!("Ingesting from {:?}...", args.path);

    for entry in WalkDir::new(args.path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let path = entry.path();
            // Filter extensions
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy();
                match ext_str.as_ref() {
                    "md" | "txt" | "rs" | "py" | "js" | "ts" | "json" | "yaml" => {
                        println!("Processing {:?}", path);
                        if let Ok(content) = std::fs::read_to_string(path) {
                            let body = serde_json::json!({
                                "content": content,
                                "metadata": {
                                    "source": path.to_string_lossy(),
                                    "type": "file"
                                }
                            });
                            
                            match client.post(&url).json(&body).send().await {
                                Ok(res) => {
                                    if !res.status().is_success() {
                                        println!("Failed to ingest {:?}: {}", path, res.status());
                                    }
                                }
                                Err(e) => println!("Error sending {:?}: {}", path, e),
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    println!("Ingestion complete.");
    Ok(())
}
