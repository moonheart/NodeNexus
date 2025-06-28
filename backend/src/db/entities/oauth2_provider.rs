use sea_orm::entity::prelude::*;
use crate::services::encryption_service::{decrypt, encrypt};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "oauth2_providers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)]
    pub provider_name: String,
    pub client_id: String,
    pub client_secret: String, // Encrypted
    pub auth_url: String,
    pub token_url: String,
    pub user_info_url: String,
    pub scopes: Option<String>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub user_info_mapping: Option<serde_json::Value>,
    pub icon_url: Option<String>,
    pub enabled: bool,
    pub created_at: ChronoDateTimeUtc,
    pub updated_at: ChronoDateTimeUtc,
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

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

impl ActiveModel {
    pub fn encrypt_client_secret(&mut self, key: &str) -> Result<(), String> {
        if let sea_orm::ActiveValue::Set(secret) = &self.client_secret {
            let encrypted_secret = encrypt(secret, key)?;
            self.client_secret = sea_orm::ActiveValue::Set(encrypted_secret);
        }
        // If it's NotSet or Unchanged, we don't need to do anything.
        Ok(())
    }
}