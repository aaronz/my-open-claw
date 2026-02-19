pub mod weather;

use openclaw_core::Tool;
use std::collections::HashMap;

pub fn default_tools() -> HashMap<String, Box<dyn Tool>> {
    let mut tools: HashMap<String, Box<dyn Tool>> = HashMap::new();
    let weather = weather::WeatherTool;
    tools.insert(weather.definition().name, Box::new(weather));
    tools
}
