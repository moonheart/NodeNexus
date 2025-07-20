use nodenexus_common::agent_service::AgentConfig;
use serde::{Deserialize, Serialize};
use std::{error::Error, fs, path::Path};
use tracing::{error, info};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentCliConfig {
    pub server_address: String,
    pub vps_id: i32,
    pub agent_secret: String,
    pub agent_grpc_listen_address: Option<String>, // Address for the agent's own gRPC service
    #[serde(skip)]
    pub config_path: String,
}

pub fn load_cli_config(config_path_str: &str) -> Result<AgentCliConfig, Box<dyn Error>> {
    let config_path = Path::new(config_path_str);
    // Attempt to get absolute path for logging, but don't fail if it can't be canonicalized (e.g. if file doesn't exist yet)
    let absolute_path_display = config_path
        .canonicalize()
        .unwrap_or_else(|_| config_path.to_path_buf());
    info!(path = ?absolute_path_display, "Attempting to load config.");

    let config_str = fs::read_to_string(config_path).map_err(|e| {
        error!(path = %config_path_str, error = %e, "Failed to read agent config file.");
        Box::new(e) as Box<dyn Error>
    })?;

    let agent_cli_config: AgentCliConfig = toml::from_str(&config_str).map_err(|e| {
        error!(path = %config_path_str, error = %e, "Failed to parse agent config file.");
        Box::new(e) as Box<dyn Error>
    })?;

    info!(config = ?agent_cli_config, "Loaded config successfully.");
    Ok(agent_cli_config)
}

pub fn save_agent_config(
    config: &AgentConfig,
    config_path_str: &str,
) -> Result<(), Box<dyn Error>> {
    let config_path = Path::new(config_path_str);

    // 1. Read the existing file content. If it doesn't exist or is empty, default to an empty string.
    let existing_content = fs::read_to_string(config_path).unwrap_or_default();

    // 2. Parse it into a generic TOML Value. If empty, it will be an empty table.
    let mut existing_toml: toml::Value = toml::from_str(&existing_content)?;

    // 3. Convert the new protobuf-generated config into a TOML string first.
    let new_config_str = toml::to_string(config)?;
    // Then parse that string into a TOML Value.
    let new_toml: toml::Value = toml::from_str(&new_config_str)?;

    // 4. Merge the new values into the existing TOML structure
    if let (Some(existing_table), Some(new_table)) =
        (existing_toml.as_table_mut(), new_toml.as_table())
    {
        for (key, value) in new_table {
            existing_table.insert(key.clone(), value.clone());
        }
    } else {
        // If the existing file is not a table (e.g., empty or invalid), just use the new config.
        existing_toml = new_toml;
    }

    // 5. Serialize the merged TOML value back to a string
    let updated_content = toml::to_string_pretty(&existing_toml)?;

    // 6. Write the updated content back to the file
    fs::write(config_path, updated_content)?;

    info!(path = ?config_path, "Successfully merged and saved configuration.");
    Ok(())
}
