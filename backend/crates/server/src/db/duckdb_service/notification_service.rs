use std::collections::HashMap;
use std::sync::Arc;
use tokio::task;
use duckdb::{params, Connection, Result as DuckDbResult, ToSql};
use tracing::{error, info};

use crate::db::duckdb_service::DuckDbPool;
use crate::db::entities::notification_channel;
use crate::notifications::encryption::{EncryptionService, EncryptionError};
use crate::notifications::models::{ChannelConfig, CreateChannelRequest, ChannelResponse, UpdateChannelRequest};
use crate::notifications::senders::{NotificationSender, SenderError, telegram::TelegramSender, webhook::WebhookSender};
use crate::web::error::AppError;

pub async fn create_channel(
    pool: DuckDbPool,
    encryption_service: Arc<EncryptionService>,
    user_id: i32,
    payload: CreateChannelRequest,
) -> Result<ChannelResponse, AppError> {
    task::spawn_blocking(move || {
        let config_value: ChannelConfig = serde_json::from_value(payload.config)
            .map_err(|e| AppError::InvalidInput(e.to_string()))?;
        let encrypted_config = encryption_service
            .encrypt(&serde_json::to_vec(&config_value).unwrap())
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;

        let conn = pool.get().map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let model: notification_channel::Model = conn.query_row(
            "INSERT INTO notification_channels (user_id, name, channel_type, config) VALUES (?, ?, ?, ?) RETURNING *",
            params![
                user_id,
                payload.name,
                payload.channel_type,
                encrypted_config,
            ],
            row_to_channel_model,
        ).map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let decrypted_config_bytes = encryption_service.decrypt(&model.config)
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;
        let config_params: ChannelConfig = serde_json::from_slice(&decrypted_config_bytes)
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;
        let config_params_json = serde_json::to_value(config_params)
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;

        Ok(ChannelResponse {
            id: model.id,
            name: model.name,
            channel_type: model.channel_type,
            config_params: Some(config_params_json),
        })
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
}

fn row_to_channel_model(row: &duckdb::Row<'_>) -> DuckDbResult<notification_channel::Model> {
    Ok(notification_channel::Model {
        id: row.get(0)?,
        user_id: row.get(1)?,
        name: row.get(2)?,
        channel_type: row.get(3)?,
        config: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

pub async fn get_all_channels_for_user(
    pool: DuckDbPool,
    encryption_service: Arc<EncryptionService>,
    user_id: i32,
) -> Result<Vec<ChannelResponse>, AppError> {
    task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT * FROM notification_channels WHERE user_id = ? ORDER BY name ASC")
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let models = stmt
            .query_map(params![user_id], row_to_channel_model)
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut channels_response = Vec::new();
        for model in models {
            let decrypted_config_bytes = encryption_service.decrypt(&model.config)
                .map_err(|e| AppError::InternalServerError(e.to_string()))?;
            let config_params: ChannelConfig = serde_json::from_slice(&decrypted_config_bytes)
                .map_err(|e| AppError::InternalServerError(e.to_string()))?;
            let config_params_json = serde_json::to_value(config_params)
                .map_err(|e| AppError::InternalServerError(e.to_string()))?;
            
            channels_response.push(ChannelResponse {
                id: model.id,
                name: model.name,
                channel_type: model.channel_type,
                config_params: Some(config_params_json),
            });
        }
        Ok(channels_response)
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
}

pub async fn get_channel_by_id(
    pool: DuckDbPool,
    encryption_service: Arc<EncryptionService>,
    user_id: i32,
    channel_id: i32,
) -> Result<ChannelResponse, AppError> {
    task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let model: notification_channel::Model = conn.query_row(
            "SELECT * FROM notification_channels WHERE id = ? AND user_id = ?",
            params![channel_id, user_id],
            row_to_channel_model,
        ).map_err(|e| {
            if let duckdb::Error::QueryReturnedNoRows = e {
                AppError::NotFound("Notification channel not found".to_string())
            } else {
                AppError::DatabaseError(e.to_string())
            }
        })?;

        let decrypted_config_bytes = encryption_service.decrypt(&model.config)
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;
        let config_params: ChannelConfig = serde_json::from_slice(&decrypted_config_bytes)
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;
        let config_params_json = serde_json::to_value(config_params)
            .map_err(|e| AppError::InternalServerError(e.to_string()))?;

        Ok(ChannelResponse {
            id: model.id,
            name: model.name,
            channel_type: model.channel_type,
            config_params: Some(config_params_json),
        })
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
}

pub async fn update_channel(
    pool: DuckDbPool,
    encryption_service: Arc<EncryptionService>,
    user_id: i32,
    channel_id: i32,
    payload: UpdateChannelRequest,
) -> Result<ChannelResponse, AppError> {
    let pool_clone = pool.clone();
    let encryption_service_clone = encryption_service.clone();
    task::spawn_blocking(move || {
        let conn = pool_clone.get().map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut set_clauses: Vec<String> = Vec::new();
        let mut params_vec: Vec<Box<dyn ToSql>> = Vec::new();

        if let Some(name) = payload.name {
            set_clauses.push("name = ?".to_string());
            params_vec.push(Box::new(name));
        }

        let encrypted_config; 
        if let Some(new_config_value) = payload.config {
            let config_enum: ChannelConfig = serde_json::from_value(new_config_value)
                .map_err(|e| AppError::InvalidInput(e.to_string()))?;
            encrypted_config = encryption_service_clone
                .encrypt(&serde_json::to_vec(&config_enum).unwrap())
                .map_err(|e| AppError::InternalServerError(e.to_string()))?;
            set_clauses.push("config = ?".to_string());
            params_vec.push(Box::new(encrypted_config));
        }

        if !set_clauses.is_empty() {
            set_clauses.push("updated_at = ?".to_string());
            params_vec.push(Box::new(chrono::Utc::now()));

            let sql = format!(
                "UPDATE notification_channels SET {} WHERE id = ? AND user_id = ?",
                set_clauses.join(", ")
            );
            
            let mut final_params = params_vec;
            final_params.push(Box::new(channel_id));
            final_params.push(Box::new(user_id));

            let params_slice: Vec<&dyn ToSql> = final_params.iter().map(|b| b.as_ref()).collect();

            let num_updated = conn.execute(&sql, &params_slice[..]).map_err(|e| AppError::DatabaseError(e.to_string()))?;

            if num_updated == 0 {
                return Err(AppError::NotFound("Notification channel not found or not owned by user".to_string()));
            }
        }

        Ok(())
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))??;

    get_channel_by_id(pool, encryption_service, user_id, channel_id).await
}

pub async fn delete_channel(pool: DuckDbPool, user_id: i32, channel_id: i32) -> Result<(), AppError> {
    task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let rows_affected = conn.execute(
            "DELETE FROM notification_channels WHERE id = ? AND user_id = ?",
            params![channel_id, user_id],
        ).map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if rows_affected == 0 {
            Err(AppError::NotFound(
                "Notification channel not found or not owned by user".to_string(),
            ))
        } else {
            Ok(())
        }
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
}

pub async fn send_notifications_for_alert_rule(
    pool: DuckDbPool,
    encryption_service: Arc<EncryptionService>,
    rule_id: i32,
    alert_message: String,
) -> Result<(), AppError> {
    
    // Part 1: Fetch data from DB in a blocking task
    let channels_to_notify = task::spawn_blocking(move || -> Result<Vec<(ChannelConfig, notification_channel::Model)>, AppError> {
        let conn = pool.get().map_err(AppError::from)?;

        let mut stmt = conn.prepare("SELECT channel_id FROM alert_rule_channels WHERE alert_rule_id = ?")?;
        let channel_ids = stmt.query_map(params![rule_id], |row| row.get::<_, i32>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        if channel_ids.is_empty() {
            info!(rule_id = rule_id, "No notification channels linked to alert rule.");
            return Ok(Vec::new());
        }

        let mut channels_to_notify = Vec::new();
        for channel_id in channel_ids {
            match conn.query_row::<notification_channel::Model, _, _>(
                "SELECT * FROM notification_channels WHERE id = ?",
                params![channel_id],
                row_to_channel_model,
            ) {
                Ok(model) => {
                    match encryption_service.decrypt(&model.config) {
                        Ok(decrypted_bytes) => {
                            match serde_json::from_slice::<ChannelConfig>(&decrypted_bytes) {
                                Ok(config) => channels_to_notify.push((config, model)),
                                Err(e) => error!(channel_id, "Failed to deserialize channel config: {}", e),
                            }
                        },
                        Err(e) => error!(channel_id, "Failed to decrypt channel config: {}", e),
                    }
                },
                Err(e) => error!(channel_id, "Failed to fetch channel details: {}", e),
            }
        }
        Ok(channels_to_notify)
    }).await.map_err(|e| AppError::InternalServerError(e.to_string()))??;

    // Part 2: Send notifications in the async context
    let mut last_error: Option<SenderError> = None;
    let context = HashMap::new();

    for (config, model) in channels_to_notify {
        let sender: Box<dyn NotificationSender + Send + Sync> = match model.channel_type.as_str() {
            "telegram" => Box::new(TelegramSender::new()),
            "webhook" => Box::new(WebhookSender::new()),
            unsupported => {
                error!("Unsupported channel type for sending: {}", unsupported);
                continue;
            }
        };

        match sender.send(&config, &alert_message, &context).await {
            Ok(_) => info!(channel_id = model.id, rule_id = rule_id, "Successfully sent alert notification."),
            Err(e) => {
                error!(channel_id = model.id, rule_id = rule_id, error = ?e, "Failed to send alert notification.");
                last_error = Some(e);
            }
        }
    }

    if let Some(err) = last_error {
        Err(AppError::InternalServerError(err.to_string()))
    } else {
        Ok(())
    }
}

pub async fn send_test_notification(
    pool: DuckDbPool,
    encryption_service: Arc<EncryptionService>,
    user_id: i32,
    channel_id: i32,
    message: String,
) -> Result<(), AppError> {
    // Part 1: Fetch channel data in a blocking task
    let (config, model) = task::spawn_blocking(move || -> Result<(ChannelConfig, notification_channel::Model), AppError> {
        let conn = pool.get().map_err(AppError::from)?;
        let model: notification_channel::Model = conn.query_row(
            "SELECT * FROM notification_channels WHERE id = ? AND user_id = ?",
            params![channel_id, user_id],
            row_to_channel_model,
        ).map_err(|e| {
            if let duckdb::Error::QueryReturnedNoRows = e {
                AppError::NotFound("Notification channel not found".to_string())
            } else {
                AppError::DatabaseError(e.to_string())
            }
        })?;

        let decrypted_bytes = encryption_service.decrypt(&model.config)
            .map_err(|e| AppError::InternalServerError(format!("Failed to decrypt channel config: {}", e)))?;
        let config: ChannelConfig = serde_json::from_slice(&decrypted_bytes)
            .map_err(|e| AppError::InternalServerError(format!("Failed to deserialize channel config: {}", e)))?;
        
        Ok((config, model))
    }).await.map_err(|e| AppError::InternalServerError(e.to_string()))??;

    // Part 2: Send notification in the async context
    let sender: Box<dyn NotificationSender + Send + Sync> = match model.channel_type.as_str() {
        "telegram" => Box::new(TelegramSender::new()),
        "webhook" => Box::new(WebhookSender::new()),
        unsupported => {
            let err_msg = format!("Unsupported channel type for sending: {}", unsupported);
            error!("{}", err_msg);
            return Err(AppError::InternalServerError(err_msg));
        }
    };

    let context = HashMap::new(); // No context for test messages
    sender.send(&config, &message, &context).await.map_err(|e| {
        error!(channel_id = model.id, error = ?e, "Failed to send test notification.");
        AppError::InternalServerError(e.to_string())
    })?;

    info!(channel_id = model.id, "Successfully sent test notification.");
    Ok(())
}