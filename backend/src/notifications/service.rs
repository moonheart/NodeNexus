use std::collections::HashMap;
use std::sync::Arc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, Set}; // Added SeaORM imports
use thiserror::Error;

use super::encryption::{EncryptionService, EncryptionError};
use super::models::{ChannelConfig, ChannelTemplate, ChannelTemplateField};
use super::senders::{
    NotificationSender, SenderError,
    telegram::TelegramSender,
    webhook::WebhookSender,
};
use crate::db::entities::notification_channel; // Changed to use entity
// Import for AlertRuleChannel if needed later
use crate::db::entities::alert_rule_channel;


#[derive(Error, Debug)]
pub enum NotificationError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] DbErr), // Changed from sqlx::Error
    #[error("Encryption error: {0}")]
    EncryptionError(#[from] EncryptionError),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Channel not found: {0}")]
    NotFound(i32),
    #[error("Unsupported channel type: {0}")]
    UnsupportedChannel(String),
    #[error("Sender error: {0}")]
    SenderError(#[from] SenderError),
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Generic error: {0}")]
    Generic(String),
}

pub struct NotificationService {
    db_pool: DatabaseConnection, // Changed PgPool to DatabaseConnection
    encryption_service: Arc<EncryptionService>,
}

impl NotificationService {
    pub fn new(db_pool: DatabaseConnection, encryption_service: Arc<EncryptionService>) -> Self {
        Self {
            db_pool,
            encryption_service,
        }
    }

    /// Dispatches a notification to a specific channel.
    pub async fn send_notification(
        &self,
        channel_id: i32,
        message: &str,
        context: &HashMap<String, String>,
    ) -> Result<(), NotificationError> {
        let channel_model = notification_channel::Entity::find_by_id(channel_id)
            .one(&self.db_pool)
            .await?
            .ok_or_else(|| NotificationError::NotFound(channel_id))?;

        let decrypted_config_bytes = self.encryption_service.decrypt(&channel_model.config)?;
        let config: ChannelConfig = serde_json::from_slice(&decrypted_config_bytes)?;

        match channel_model.channel_type.as_str() {
            "telegram" => {
                let sender = TelegramSender::new();
                sender.send(&config, message, context).await?;
            }
            "webhook" => {
                let sender = WebhookSender::new();
                sender.send(&config, message, context).await?;
            }
            _ => return Err(NotificationError::UnsupportedChannel(channel_model.channel_type)),
        }

        Ok(())
    }

    pub async fn create_channel(
        &self,
        user_id: i32,
        payload: super::models::CreateChannelRequest,
    ) -> Result<super::models::ChannelResponse, NotificationError> {
        let config_value: ChannelConfig = serde_json::from_value(payload.config)?;
        let encrypted_config = self.encryption_service.encrypt(&serde_json::to_vec(&config_value)?)?;

        let new_channel = notification_channel::ActiveModel {
            user_id: Set(user_id),
            name: Set(payload.name),
            channel_type: Set(payload.channel_type),
            config: Set(encrypted_config),
            ..Default::default()
        };

        let channel_model = new_channel.insert(&self.db_pool).await?;
        
        // Decrypt config for response
        let decrypted_config_bytes = self.encryption_service.decrypt(&channel_model.config)?;
        let config_params: ChannelConfig = serde_json::from_slice(&decrypted_config_bytes)?;
        let config_params_json = serde_json::to_value(config_params)?;

        Ok(super::models::ChannelResponse {
            id: channel_model.id,
            name: channel_model.name,
            channel_type: channel_model.channel_type,
            config_params: Some(config_params_json),
        })
    }

    pub async fn get_all_channels_for_user(
        &self,
        user_id: i32,
    ) -> Result<Vec<super::models::ChannelResponse>, NotificationError> {
        let channel_models = notification_channel::Entity::find()
            .filter(notification_channel::Column::UserId.eq(user_id))
            .order_by_asc(notification_channel::Column::Name)
            .all(&self.db_pool)
            .await?;

        let mut channels_response = Vec::new();
        for c_model in channel_models {
            let decrypted_config_bytes = self.encryption_service.decrypt(&c_model.config)?;
            let config_params: ChannelConfig = serde_json::from_slice(&decrypted_config_bytes)?;
            let config_params_json = serde_json::to_value(config_params)?;
            channels_response.push(super::models::ChannelResponse {
                id: c_model.id,
                name: c_model.name,
                channel_type: c_model.channel_type,
                config_params: Some(config_params_json),
            });
        }
        Ok(channels_response)
    }

    pub async fn get_channel_by_id(
        &self,
        user_id: i32,
        channel_id: i32,
    ) -> Result<super::models::ChannelResponse, NotificationError> {
        let channel_model = notification_channel::Entity::find()
            .filter(notification_channel::Column::Id.eq(channel_id))
            .filter(notification_channel::Column::UserId.eq(user_id))
            .one(&self.db_pool)
            .await?
            .ok_or_else(|| NotificationError::NotFound(channel_id))?;

        let decrypted_config_bytes = self.encryption_service.decrypt(&channel_model.config)?;
        let config_params: ChannelConfig = serde_json::from_slice(&decrypted_config_bytes)?;
        let config_params_json = serde_json::to_value(config_params)?;

        Ok(super::models::ChannelResponse {
            id: channel_model.id,
            name: channel_model.name,
            channel_type: channel_model.channel_type,
            config_params: Some(config_params_json),
        })
    }

    pub async fn update_channel(
        &self,
        user_id: i32,
        channel_id: i32,
        payload: super::models::UpdateChannelRequest,
    ) -> Result<super::models::ChannelResponse, NotificationError> {
        // First, verify ownership and get the active model
        let channel_active_model: notification_channel::ActiveModel = self.get_channel_by_id_internal(user_id, channel_id)
            .await?
            .into_active_model();

        let mut updated_model = channel_active_model.clone(); // Clone to modify

        if let Some(name) = payload.name {
            updated_model.name = Set(name);
        }
        if let Some(new_config_value) = payload.config {
            let config_enum: ChannelConfig = serde_json::from_value(new_config_value)?;
            updated_model.config = Set(self.encryption_service.encrypt(&serde_json::to_vec(&config_enum)?)?);
        }
        // updated_at is handled by SeaORM's BeforeSave or default value in DB

        let saved_channel_model = updated_model.update(&self.db_pool).await?;

        Ok(super::models::ChannelResponse {
            id: saved_channel_model.id,
            name: saved_channel_model.name,
            channel_type: saved_channel_model.channel_type,
            config_params: Some(serde_json::to_value(serde_json::from_slice::<ChannelConfig>(&self.encryption_service.decrypt(&saved_channel_model.config)?)?)?),
        })
    }

    pub async fn delete_channel(&self, user_id: i32, channel_id: i32) -> Result<(), NotificationError> {
        let result = notification_channel::Entity::delete_many()
            .filter(notification_channel::Column::Id.eq(channel_id))
            .filter(notification_channel::Column::UserId.eq(user_id))
            .exec(&self.db_pool)
            .await?;

        if result.rows_affected == 0 {
            return Err(NotificationError::NotFound(channel_id));
        }
        Ok(())
    }
    
    pub async fn test_channel(&self, user_id: i32, channel_id: i32, message: Option<String>) -> Result<(), NotificationError> {
        let channel_model = self.get_channel_by_id_internal(user_id, channel_id).await?;
        let test_message = message.unwrap_or_else(|| format!("This is a test message from channel '{}'.", channel_model.name));
        let context = HashMap::new(); // Empty context for test
        self.send_notification(channel_model.id, &test_message, &context).await
    }

    // Internal helper to fetch a channel and verify ownership
    async fn get_channel_by_id_internal(&self, user_id: i32, channel_id: i32) -> Result<notification_channel::Model, NotificationError> {
        notification_channel::Entity::find()
            .filter(notification_channel::Column::Id.eq(channel_id))
            .filter(notification_channel::Column::UserId.eq(user_id))
            .one(&self.db_pool)
            .await?
            .ok_or(NotificationError::PermissionDenied) // If not found under this user, it's a permission issue or not found
    }
    
    // Placeholder for getting channel templates
    pub fn get_channel_templates(&self) -> Vec<ChannelTemplate> {
        vec![
            ChannelTemplate {
                channel_type: "telegram".to_string(),
                name: "Telegram".to_string(),
                fields: vec![
                    ChannelTemplateField { name: "bot_token".to_string(), field_type: "password".to_string(), required: true, label: "Bot Token".to_string(), help_text: Some("Your Telegram Bot Token.".to_string()) },
                    ChannelTemplateField { name: "chat_id".to_string(), field_type: "text".to_string(), required: true, label: "Chat ID".to_string(), help_text: Some("The target chat ID (user, group, or channel).".to_string()) },
                ],
            },
            ChannelTemplate {
                channel_type: "webhook".to_string(),
                name: "Custom Webhook".to_string(),
                fields: vec![
                    ChannelTemplateField { name: "url".to_string(), field_type: "text".to_string(), required: true, label: "Webhook URL".to_string(), help_text: None },
                    ChannelTemplateField { name: "method".to_string(), field_type: "text".to_string(), required: true, label: "HTTP Method".to_string(), help_text: Some("Usually POST or GET.".to_string()) },
                    ChannelTemplateField { name: "headers".to_string(), field_type: "textarea".to_string(), required: false, label: "Headers (JSON)".to_string(), help_text: Some("A JSON object of key-value pairs for headers.".to_string()) },
                    ChannelTemplateField { name: "body_template".to_string(), field_type: "textarea".to_string(), required: false, label: "Body Template (for POST)".to_string(), help_text: Some("JSON body with Tera template variables like {{ vps_name }}.".to_string()) },
                ],
            },
        ]
    }

    pub async fn send_notifications_for_alert_rule(
        &self,
        rule_id: i32,
        _user_id: i32, // Renamed user_id as it's not currently used
        alert_message: &str,
    ) -> Result<(), NotificationError> {
        // 1. Get all channel IDs associated with the rule_id
        let linked_channel_models = alert_rule_channel::Entity::find()
            .filter(alert_rule_channel::Column::AlertRuleId.eq(rule_id))
            .all(&self.db_pool)
            .await?;

        if linked_channel_models.is_empty() {
            println!("No notification channels linked to alert rule ID: {}. No notifications sent.", rule_id);
            return Ok(());
        }

        let mut last_error: Option<NotificationError> = None;
        let context = HashMap::new(); // Create an empty context for now

        // 2. For each channel_id, send the notification
        for linked_channel_model in linked_channel_models {
            // Call the existing send_notification method
            // Note: The existing send_notification method takes channel_id, message, and context.
            // user_id is not directly passed to it but is available here if needed for other logic.
            match self.send_notification(linked_channel_model.channel_id, alert_message, &context).await {
                Ok(_) => {
                    println!("Successfully sent alert notification via channel ID: {} for rule ID: {}", linked_channel_model.channel_id, rule_id);
                }
                Err(e) => {
                    eprintln!(
                        "Failed to send alert notification via channel ID {}: {} for rule ID: {}",
                        linked_channel_model.channel_id, e, rule_id
                    );
                    last_error = Some(e); // Store the last error, but continue trying other channels
                }
            }
        }

        if let Some(err) = last_error {
            Err(err)
        } else {
            Ok(())
        }
    }
}