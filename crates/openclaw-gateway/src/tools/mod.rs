pub mod search;
pub mod weather;

use openclaw_core::{AppConfig, Tool};
use std::collections::HashMap;

pub fn default_tools(config: &AppConfig) -> HashMap<String, Box<dyn Tool>> {
    let mut tools: HashMap<String, Box<dyn Tool>> = HashMap::new();
    let weather = weather::WeatherTool;
    tools.insert(weather.definition().name, Box::new(weather));

    if let Some(key) = &config.agent.tavily_api_key {
        let search = search::SearchTool::new(key.clone());
        tools.insert(search.definition().name, Box::new(search));
    }

    tools
}
