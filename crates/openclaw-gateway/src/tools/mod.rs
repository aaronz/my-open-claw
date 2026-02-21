pub mod browser;
pub mod canvas;
pub mod cron;
pub mod fs;
pub mod python;
pub mod search;
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

    let cron_tool = cron::CronTool::new(cron);
    tools.insert(cron_tool.definition().name, Box::new(cron_tool));

    if let Some(key) = &config.agent.tavily_api_key {
        let search = search::SearchTool::new(key.clone());
        tools.insert(search.definition().name, Box::new(search));
    }

    tools
}
