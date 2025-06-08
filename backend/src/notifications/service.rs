use std::collections::HashMap;
use std::sync::Arc;
use sqlx::PgPool;
use thiserror::Error;

use super::encryption::{EncryptionService, EncryptionError};
use super::models::{ChannelConfig, ChannelTemplate, ChannelTemplateField};
use super::senders::{
    NotificationSender, SenderError,
    telegram::TelegramSender,
    webhook::WebhookSender,
};
use crate::db::models::NotificationChannel;

#[derive(Error, Debug)]
pub enum NotificationError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),
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
    db_pool: PgPool,
    encryption_service: Arc<EncryptionService>,
}

impl NotificationService {
    pub fn new(db_pool: PgPool, encryption_service: Arc<EncryptionService>) -> Self {
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
        let channel = sqlx::query_as::<_, NotificationChannel>(
            "SELECT * FROM notification_channels WHERE id = $1",
        )
        .bind(channel_id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => NotificationError::NotFound(channel_id),
            _ => NotificationError::DatabaseError(e),
        })?;

        let decrypted_config_bytes = self.encryption_service.decrypt(&channel.config)?;
        let config: ChannelConfig = serde_json::from_slice(&decrypted_config_bytes)?;

        match channel.channel_type.as_str() {
            "telegram" => {
                let sender = TelegramSender::new();
                sender.send(&config, message, context).await?;
            }
            "webhook" => {
                let sender = WebhookSender::new();
                sender.send(&config, message, context).await?;
            }
            _ => return Err(NotificationError::UnsupportedChannel(channel.channel_type)),
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

        let channel = sqlx::query_as::<_, NotificationChannel>(
            "INSERT INTO notification_channels (user_id, name, channel_type, config) VALUES ($1, $2, $3, $4) RETURNING *"
        )
        .bind(user_id)
        .bind(payload.name)
        .bind(payload.channel_type)
        .bind(encrypted_config)
        .fetch_one(&self.db_pool)
        .await?;
        
        // Decrypt config for response
        let decrypted_config_bytes = self.encryption_service.decrypt(&channel.config)?;
        let config_params: ChannelConfig = serde_json::from_slice(&decrypted_config_bytes)?;
        let config_params_json = serde_json::to_value(config_params)?;

        Ok(super::models::ChannelResponse {
            id: channel.id,
            name: channel.name,
            channel_type: channel.channel_type,
            config_params: Some(config_params_json),
        })
    }

    pub async fn get_all_channels_for_user(
        &self,
        user_id: i32,
    ) -> Result<Vec<super::models::ChannelResponse>, NotificationError> {
        let channels_db = sqlx::query_as::<_, NotificationChannel>(
            "SELECT * FROM notification_channels WHERE user_id = $1 ORDER BY name",
        )
        .bind(user_id)
        .fetch_all(&self.db_pool)
        .await?;

        let mut channels_response = Vec::new();
        for c_db in channels_db {
            let decrypted_config_bytes = self.encryption_service.decrypt(&c_db.config)?;
            let config_params: ChannelConfig = serde_json::from_slice(&decrypted_config_bytes)?;
            let config_params_json = serde_json::to_value(config_params)?;
            channels_response.push(super::models::ChannelResponse {
                id: c_db.id,
                name: c_db.name,
                channel_type: c_db.channel_type,
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
        let channel = sqlx::query_as::<_, NotificationChannel>(
            "SELECT * FROM notification_channels WHERE id = $1 AND user_id = $2",
        )
        .bind(channel_id)
        .bind(user_id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(|_| NotificationError::NotFound(channel_id))?;

        let decrypted_config_bytes = self.encryption_service.decrypt(&channel.config)?;
        let config_params: ChannelConfig = serde_json::from_slice(&decrypted_config_bytes)?;
        let config_params_json = serde_json::to_value(config_params)?;

        Ok(super::models::ChannelResponse {
            id: channel.id,
            name: channel.name,
            channel_type: channel.channel_type,
            config_params: Some(config_params_json),
        })
    }

    pub async fn update_channel(
        &self,
        user_id: i32,
        channel_id: i32,
        payload: super::models::UpdateChannelRequest,
    ) -> Result<super::models::ChannelResponse, NotificationError> {
        // First, verify ownership
        let channel = self.get_channel_by_id_internal(user_id, channel_id).await?;

        let name = payload.name.unwrap_or(channel.name);
        let config = if let Some(new_config_value) = payload.config {
            let config_enum: ChannelConfig = serde_json::from_value(new_config_value)?;
            self.encryption_service.encrypt(&serde_json::to_vec(&config_enum)?)?
        } else {
            channel.config
        };

        let updated_channel = sqlx::query_as::<_, NotificationChannel>(
            "UPDATE notification_channels SET name = $1, config = $2, updated_at = NOW() WHERE id = $3 RETURNING *"
        )
        .bind(name)
        .bind(config)
        .bind(channel_id)
        .fetch_one(&self.db_pool)
        .await?;

        Ok(super::models::ChannelResponse {
            id: updated_channel.id,
            name: updated_channel.name,
            channel_type: updated_channel.channel_type,
            config_params: Some(serde_json::to_value(serde_json::from_slice::<ChannelConfig>(&self.encryption_service.decrypt(&updated_channel.config)?)?)?),
        })
    }

    pub async fn delete_channel(&self, user_id: i32, channel_id: i32) -> Result<(), NotificationError> {
        let result = sqlx::query(
            "DELETE FROM notification_channels WHERE id = $1 AND user_id = $2",
        )
        .bind(channel_id)
        .bind(user_id)
        .execute(&self.db_pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(NotificationError::NotFound(channel_id));
        }
        Ok(())
    }
    
    pub async fn test_channel(&self, user_id: i32, channel_id: i32, message: Option<String>) -> Result<(), NotificationError> {
        let channel = self.get_channel_by_id_internal(user_id, channel_id).await?;
        let test_message = message.unwrap_or_else(|| format!("This is a test message from channel '{}'.", channel.name));
        let context = HashMap::new(); // Empty context for test
        self.send_notification(channel.id, &test_message, &context).await
    }

    // Internal helper to fetch a channel and verify ownership
    async fn get_channel_by_id_internal(&self, user_id: i32, channel_id: i32) -> Result<NotificationChannel, NotificationError> {
        sqlx::query_as::<_, NotificationChannel>(
            "SELECT * FROM notification_channels WHERE id = $1 AND user_id = $2",
        )
        .bind(channel_id)
        .bind(user_id)
        .fetch_one(&self.db_pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => NotificationError::PermissionDenied,
            _ => NotificationError::DatabaseError(e),
        })
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
        user_id: i32, // user_id from the alert rule, for context or future permission checks
        alert_message: &str,
    ) -> Result<(), NotificationError> {
        // 1. Get all channel IDs associated with the rule_id
        // Ensure crate::db::models::AlertRuleChannel is in scope or use full path
        let linked_channels = sqlx::query_as!(
            crate::db::models::AlertRuleChannel,
            "SELECT alert_rule_id, channel_id FROM alert_rule_channels WHERE alert_rule_id = $1",
            rule_id
        )
        .fetch_all(&self.db_pool) // Use self.db_pool directly as it's PgPool
        .await
        .map_err(|e| NotificationError::Generic(format!("Failed to fetch linked channels for rule {}: {}", rule_id, e)))?;

        if linked_channels.is_empty() {
            println!("No notification channels linked to alert rule ID: {}. No notifications sent.", rule_id);
            return Ok(());
        }

        let mut last_error: Option<NotificationError> = None;
        let context = HashMap::new(); // Create an empty context for now

        // 2. For each channel_id, send the notification
        for linked_channel in linked_channels {
            // Call the existing send_notification method
            // Note: The existing send_notification method takes channel_id, message, and context.
            // user_id is not directly passed to it but is available here if needed for other logic.
            match self.send_notification(linked_channel.channel_id, alert_message, &context).await {
                Ok(_) => {
                    println!("Successfully sent alert notification via channel ID: {} for rule ID: {}", linked_channel.channel_id, rule_id);
                }
                Err(e) => {
                    eprintln!(
                        "Failed to send alert notification via channel ID {}: {} for rule ID: {}",
                        linked_channel.channel_id, e, rule_id
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