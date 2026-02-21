pub mod browser;
pub mod brave;
pub mod canvas;
pub mod cron;
pub mod elevenlabs;
pub mod fs;
pub mod perplexity;
pub mod openrouter;
pub mod python;
pub mod search;
pub mod session_status;
pub mod shell;
pub mod weather;
pub mod youtube;

use openclaw_core::{AppConfig, Tool};
use std::collections::HashMap;
use std::sync::Arc;

use crate::cron::CronScheduler;
use crate::state::AppState;

pub fn default_tools(config: &AppConfig, cron: Arc<CronScheduler>, state: Arc<AppState>) -> HashMap<String, Box<dyn Tool>> {
    let mut tools: HashMap<String, Box<dyn Tool>> = HashMap::new();
    let weather = weather::WeatherTool;
    tools.insert(weather.definition().name, Box::new(weather));

    let python = python::PythonTool;
    tools.insert(python.definition().name, Box::new(python));

    let fs = fs::FileSystemTool::new(config);
    tools.insert(fs.definition().name, Box::new(fs));

    let browser = browser::BrowserTool::new();
    tools.insert(browser.definition().name, Box::new(browser));

    let canvas = canvas::CanvasTool::new(state);
    tools.insert(canvas.definition().name, Box::new(canvas));

    let youtube = youtube::YouTubeTool::new();
    tools.insert(youtube.definition().name, Box::new(youtube));

    let status_tool = session_status::SessionStatusTool::new(state.clone());
    tools.insert(status_tool.definition().name, Box::new(status_tool));

    let cron_tool = cron::CronTool::new(cron);
    tools.insert(cron_tool.definition().name, Box::new(cron_tool));

    let shell = shell::ShellTool;
    tools.insert(shell.definition().name, Box::new(shell));

    if let Some(key) = &config.agent.elevenlabs_api_key {
        let el = elevenlabs::ElevenLabsTool::new(key.clone());
        tools.insert(el.definition().name, Box::new(el));
    }

    if let Some(key) = &config.agent.brave_api_key {
        let brave = brave::BraveSearchTool::new(key.clone());
        tools.insert(brave.definition().name, Box::new(brave));
    }

    if let Some(key) = &config.agent.perplexity_api_key {
        let pplx = perplexity::PerplexityTool::new(key.clone());
        tools.insert(pplx.definition().name, Box::new(pplx));
    }

    if let Some(key) = &config.agent.openrouter_api_key {
        let or = openrouter::OpenRouterTool::new(key.clone());
        tools.insert(or.definition().name, Box::new(or));
    }

    if let Some(key) = &config.agent.tavily_api_key {
        let search = search::SearchTool::new(key.clone());
        tools.insert(search.definition().name, Box::new(search));
    }

    tools
}
