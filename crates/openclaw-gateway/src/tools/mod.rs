pub mod agents_list;
pub mod apply_patch;
pub mod browser;
pub mod brave;
pub mod canvas;
pub mod cron;
pub mod elevenlabs;
pub mod exec;
pub mod fs;
pub mod gateway;
pub mod image;
pub mod message;
pub mod nodes;
pub mod openrouter;
pub mod perplexity;
pub mod process;
pub mod python;
pub mod sandbox;
pub mod search;
pub mod session_status;
pub mod sessions;
pub mod shell;
pub mod weather;
pub mod web_fetch;
pub mod youtube;

use openclaw_core::{AppConfig, Tool};
use std::collections::HashMap;
use std::sync::Arc;

use crate::cron::CronScheduler;
use crate::nodes::NodeManager;
use crate::state::AppState;
use crate::tools::process::ProcessManager;

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

    let canvas = canvas::CanvasTool::new(state.clone());
    tools.insert(canvas.definition().name, Box::new(canvas));

    let youtube = youtube::YouTubeTool::new();
    tools.insert(youtube.definition().name, Box::new(youtube));

    let status_tool = session_status::SessionStatusTool::new(state.clone());
    tools.insert(status_tool.definition().name, Box::new(status_tool));

    let cron_tool = cron::CronTool::new(cron);
    tools.insert(cron_tool.definition().name, Box::new(cron_tool));

    let shell = shell::ShellTool;
    tools.insert(shell.definition().name, Box::new(shell));

    let sandbox = sandbox::SandboxTool::new();
    tools.insert(sandbox.definition().name, Box::new(sandbox));

    let apply_patch = apply_patch::ApplyPatchTool;
    tools.insert(apply_patch.definition().name, Box::new(apply_patch));

    let message = message::MessageTool;
    tools.insert(message.definition().name, Box::new(message));

    let sessions = sessions::SessionsTool::new(state.clone());
    tools.insert(sessions.definition().name, Box::new(sessions));

    let nodes = nodes::NodesTool::new(Arc::new(NodeManager::new()));
    tools.insert(nodes.definition().name, Box::new(nodes));

    let web_fetch = web_fetch::WebFetchTool::new();
    tools.insert(web_fetch.definition().name, Box::new(web_fetch));

    let gateway = gateway::GatewayTool::new(state.clone());
    tools.insert(gateway.definition().name, Box::new(gateway));

    let exec = exec::ExecTool::new(state.clone());
    tools.insert(exec.definition().name, Box::new(exec));

    let agents_list = agents_list::AgentsListTool;
    tools.insert(agents_list.definition().name, Box::new(agents_list));

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

    if let Some(key) = &config.audio.openai_api_key {
        let image = image::ImageTool::new(Some(key.clone()));
        tools.insert(image.definition().name, Box::new(image));
    }

    tools
}
