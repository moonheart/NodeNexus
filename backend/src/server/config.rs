use std::env;

#[derive(Clone)]
pub struct ServerConfig {
    pub frontend_url: String,
    pub jwt_secret: String,
    pub notification_encryption_key: String,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, String> {
        let frontend_url = env::var("FRONTEND_URL")
            .map_err(|_| "FRONTEND_URL must be set".to_string())?;
        
        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| "JWT_SECRET must be set".to_string())?;

        let notification_encryption_key = env::var("NOTIFICATION_ENCRYPTION_KEY")
            .map_err(|_| "NOTIFICATION_ENCRYPTION_KEY must be set".to_string())?;

        Ok(ServerConfig {
            frontend_url,
            jwt_secret,
            notification_encryption_key,
        })
    }
}