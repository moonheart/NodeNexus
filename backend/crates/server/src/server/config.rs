use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Deserialize, Debug, Clone)]
pub struct ServerConfig {
    pub frontend_url: String,
    pub jwt_secret: String,
    
    #[serde(default = "default_notification_key")]
    pub notification_encryption_key: String,

    #[serde(default = "default_data_dir")]
    pub data_dir: String,

    #[serde(default = "default_log_dir")]
    pub log_dir: String,

    #[serde(default = "default_update_url")]
    pub update_url: String,

    #[serde(default)]
    pub is_in_container: bool,
}

// Partial config for layering
#[derive(Deserialize, Default, Debug)]
struct PartialServerConfig {
    frontend_url: Option<String>,
    jwt_secret: Option<String>,
    notification_encryption_key: Option<String>,
    data_dir: Option<String>,
    log_dir: Option<String>,
    update_url: Option<String>,
    is_in_container: Option<bool>,
}

fn default_data_dir() -> String {
    "data".to_string()
}

fn default_log_dir() -> String {
    "logs".to_string()
}

fn default_update_url() -> String {
    "https://api.github.com/repos/mjjer/nodenexus/releases/latest".to_string()
}

fn default_notification_key() -> String {
    // This key is for development convenience.
    // It's crucial to override this in production via environment variables.
    "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f".to_string()
}

impl ServerConfig {
    pub fn load(config_path: Option<&str>) -> Result<Self, String> {
        dotenv::dotenv().ok();

        // 1. Load from file (optional)
        let file_config: PartialServerConfig = if let Some(path_str) = config_path {
            let path = Path::new(path_str);
            if path.exists() {
                let contents = fs::read_to_string(path)
                    .map_err(|e| format!("Failed to read config file at {path:?}: {e}"))?;
                toml::from_str(&contents)
                    .map_err(|e| format!("Failed to parse TOML from config file at {path:?}: {e}"))?
            } else {
                PartialServerConfig::default()
            }
        } else {
            PartialServerConfig::default()
        };

        // 2. Load from environment variables
        let env_config: PartialServerConfig = envy::from_env::<PartialServerConfig>()
            .map_err(|e| format!("Failed to load config from environment: {e}"))?;

        // 3. Merge: environment overrides file
        let final_config = ServerConfig {
            frontend_url: env_config.frontend_url.or(file_config.frontend_url)
                .ok_or("FRONTEND_URL is required")?,
            jwt_secret: env_config.jwt_secret.or(file_config.jwt_secret)
                .ok_or("JWT_SECRET is required")?,
            notification_encryption_key: env_config.notification_encryption_key.or(file_config.notification_encryption_key)
                .unwrap_or_else(default_notification_key),
            data_dir: env_config.data_dir.or(file_config.data_dir)
                .unwrap_or_else(default_data_dir),
            log_dir: env_config.log_dir.or(file_config.log_dir)
                .unwrap_or_else(default_log_dir),
            update_url: env_config.update_url.or(file_config.update_url)
                .unwrap_or_else(default_update_url),
            is_in_container: env_config.is_in_container.or(file_config.is_in_container)
                .unwrap_or(false),
        };

        Ok(final_config)
    }
}
