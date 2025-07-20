use serde::Deserialize;
use std::env;
use std::fs;
use std::path::Path;

#[derive(Clone, Deserialize)]
pub struct ServerConfig {
    pub frontend_url: String,
    pub jwt_secret: String,
    pub notification_encryption_key: String,
}

impl ServerConfig {
    /// Loads configuration by layering sources: file -> environment variables.
    /// Environment variables have the highest priority.
    pub fn load(config_path: Option<&str>) -> Result<Self, String> {
        // Start with a partial config from a file if a path is provided.
        let mut file_config: PartialServerConfig = if let Some(path_str) = config_path {
            let path = Path::new(path_str);
            let contents = fs::read_to_string(path)
                .map_err(|e| format!("Failed to read config file at {path:?}: {e}"))?;
            toml::from_str(&contents)
                .map_err(|e| format!("Failed to parse TOML from config file at {path:?}: {e}"))?
        } else {
            PartialServerConfig::default()
        };

        // Override with environment variables if they are set.
        if let Ok(val) = env::var("FRONTEND_URL") {
            file_config.frontend_url = Some(val);
        }
        if let Ok(val) = env::var("JWT_SECRET") {
            file_config.jwt_secret = Some(val);
        }
        if let Ok(val) = env::var("NOTIFICATION_ENCRYPTION_KEY") {
            file_config.notification_encryption_key = Some(val);
        }

        // Finalize the configuration, ensuring all fields are present.
        file_config.try_into()
    }
}

// A temporary structure to hold layered configuration. Fields are optional.
#[derive(Deserialize, Default)]
struct PartialServerConfig {
    frontend_url: Option<String>,
    jwt_secret: Option<String>,
    notification_encryption_key: Option<String>,
}

impl TryFrom<PartialServerConfig> for ServerConfig {
    type Error = String;

    fn try_from(partial: PartialServerConfig) -> Result<Self, Self::Error> {
        Ok(ServerConfig {
            frontend_url: partial.frontend_url.ok_or_else(|| {
                "Missing required config: `frontend_url` or `FRONTEND_URL`".to_string()
            })?,
            jwt_secret: partial.jwt_secret.ok_or_else(|| {
                "Missing required config: `jwt_secret` or `JWT_SECRET`".to_string()
            })?,
            notification_encryption_key: partial.notification_encryption_key.ok_or_else(
                || {
                    "Missing required config: `notification_encryption_key` or `NOTIFICATION_ENCRYPTION_KEY`"
                        .to_string()
                },
            )?,
        })
    }
}
