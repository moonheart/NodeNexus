use crate::services::encryption_service::{decrypt, encrypt};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Model {
    pub id: i32,
    pub provider_name: String,
    pub client_id: String,
    pub client_secret: String, // Encrypted
    pub auth_url: String,
    pub token_url: String,
    pub user_info_url: String,
    pub scopes: Option<String>,
    pub user_info_mapping: Option<serde_json::Value>,
    pub icon_url: Option<String>,
    pub enabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Model {
    pub fn encrypt_client_secret(&mut self, key: &str) -> Result<(), String> {
        self.client_secret = encrypt(&self.client_secret, key)?;
        Ok(())
    }

    pub fn decrypt_client_secret(&mut self, key: &str) -> Result<(), String> {
        self.client_secret = decrypt(&self.client_secret, key)?;
        Ok(())
    }
}
