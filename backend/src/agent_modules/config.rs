use serde::Deserialize;
use std::{fs, error::Error, path::Path};

#[derive(Deserialize, Debug, Clone)]
pub struct AgentCliConfig {
    pub server_address: String,
    pub vps_id: i32,
    pub agent_secret: String,
}

pub fn load_cli_config(config_path_str: &str) -> Result<AgentCliConfig, Box<dyn Error>> {
    let config_path = Path::new(config_path_str);
    // Attempt to get absolute path for logging, but don't fail if it can't be canonicalized (e.g. if file doesn't exist yet)
    let absolute_path_display = config_path.canonicalize().unwrap_or_else(|_| config_path.to_path_buf());
    println!("[Agent] Attempting to load config from: {:?}", absolute_path_display);

    let config_str = fs::read_to_string(config_path)
        .map_err(|e| {
            eprintln!("Failed to read agent config file '{}': {}", config_path_str, e);
            Box::new(e) as Box<dyn Error>
        })?;
    
    let agent_cli_config: AgentCliConfig = toml::from_str(&config_str)
        .map_err(|e| {
            eprintln!("Failed to parse agent config file '{}': {}", config_path_str, e);
            Box::new(e) as Box<dyn Error>
        })?;

    println!("[Agent] Loaded config: {:?}", agent_cli_config);
    Ok(agent_cli_config)
}