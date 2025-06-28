use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router, http::StatusCode,
};
use std::sync::Arc;
// Removed: use sea_orm::DbErr; // Added DbErr
use crate::web::{AppState, AppError}; // Added AppError
use crate::agent_service::{AgentConfig, MessageToAgent, message_to_agent::Payload as AgentPayload, UpdateConfigRequest};
use crate::db::services as db_services;
use crate::db::entities::{setting, vps}; // Added setting and vps entities
use uuid::Uuid;
use tracing::{error, warn};
use futures_util::SinkExt; // Import the SinkExt trait

// This router is for global settings, mounted under /api/settings
pub fn create_settings_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/agent-config", get(get_global_agent_config).put(update_global_agent_config))
}

// This router will be merged into the main VPS router
pub fn create_vps_config_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{id}/config-override", put(update_vps_config_override))
        .route("/{id}/retry-config", post(retry_config_push))
        .route("/{id}/push-config", post(retry_config_push)) // Re-use retry logic for explicit push
        .route("/{id}/config-preview", get(preview_vps_config))
}

#[axum::debug_handler]
async fn get_global_agent_config(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<AgentConfig>, AppError> { // Changed return type
    let setting_model_option: Option<setting::Model> = db_services::get_setting(&app_state.db_pool, "global_agent_config")
        .await
        .map_err(|db_err| AppError::DatabaseError(db_err.to_string()))?;

    match setting_model_option {
        Some(s_model) => {
            let config: AgentConfig = serde_json::from_value(s_model.value) // s_model.value is already serde_json::Value
                .map_err(|e| AppError::ServerError(format!("Failed to parse global config: {e}")))?;
            Ok(Json(config))
        },
        None => Err(AppError::NotFound("Global agent config not found.".to_string())),
    }
}

#[axum::debug_handler]
async fn update_global_agent_config(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<AgentConfig>,
) -> Result<StatusCode, AppError> { // Changed return type
    let value = serde_json::to_value(&payload)
        .map_err(|e| AppError::InvalidInput(format!("Failed to serialize config: {e}")))?;
    
    db_services::update_setting(&app_state.db_pool, "global_agent_config", &value)
        .await
        .map_err(|db_err| AppError::DatabaseError(db_err.to_string()))?;

    // Trigger config push to all agents that don't have an override.
    // Assuming get_vps_by_user_id now returns Result<Vec<vps::Model>, DbErr>
    let all_vps_models: Vec<vps::Model> = db_services::get_vps_by_user_id(&app_state.db_pool, 1).await // Assuming user_id 1 for now
        .map_err(|db_err| AppError::DatabaseError(db_err.to_string()))?;

    for vps_model in all_vps_models {
        if vps_model.agent_config_override.is_none() {
            // push_config_to_vps now returns Result<(), AppError>, handle or ignore error
            if let Err(e) = push_config_to_vps(app_state.clone(), vps_model.id).await {
                error!(vps_id = vps_model.id, error = ?e, "Failed to push config to VPS after global update.");
                // Decide if this should halt the process or just log
            }
        }
    }

    Ok(StatusCode::OK)
}

#[axum::debug_handler]
async fn update_vps_config_override(
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
    // TODO: Add user auth check
    Json(payload): Json<serde_json::Value>,
) -> Result<StatusCode, AppError> { // Changed return type
    let user_id = 1; // For now, user_id is hardcoded.
    
    db_services::update_vps_config_override(&app_state.db_pool, vps_id, user_id, &payload)
        .await
        .map_err(|db_err| AppError::DatabaseError(db_err.to_string()))?;

    // Trigger config push to this specific agent.
    push_config_to_vps(app_state, vps_id).await?; // push_config_to_vps now returns AppError

    Ok(StatusCode::OK)
}

#[axum::debug_handler]
async fn retry_config_push(
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
) -> Result<StatusCode, AppError> { // Changed return type
    push_config_to_vps(app_state, vps_id).await?; // push_config_to_vps now returns AppError
    Ok(StatusCode::OK)
}

#[axum::debug_handler]
async fn preview_vps_config(
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
) -> Result<Json<AgentConfig>, AppError> {
    let effective_config = get_effective_vps_config(app_state, vps_id).await?;
    Ok(Json(effective_config))
}

/// Gets the effective config for a VPS and pushes it to the agent if connected.
pub async fn push_config_to_vps(
    app_state: Arc<AppState>,
    vps_id: i32,
) -> Result<(), AppError> {
    let effective_config = get_effective_vps_config(app_state.clone(), vps_id).await?;

    // Find agent and send message
    let agent_state = {
        let agents_guard = app_state.connected_agents.lock().await;
        agents_guard.find_by_vps_id(vps_id)
    };

    if let Some(mut state) = agent_state { // Make state mutable
        let config_version_id = Uuid::new_v4().to_string();
        let update_req = UpdateConfigRequest {
            new_config: Some(effective_config),
            config_version_id,
        };
        let msg = MessageToAgent {
            server_message_id: 0, // This should be managed better, but 0 for now.
            payload: Some(AgentPayload::UpdateConfigRequest(update_req)),
        };

        if state.sender.send(msg).await.is_ok() {
            // Assuming update_vps_config_status returns Result<_, DbErr>
            if let Err(e) = db_services::update_vps_config_status(&app_state.db_pool, vps_id, "pending", None).await {
                error!(vps_id = vps_id, error = ?e, "Failed to update VPS config status to pending.");
            }
        } else {
            let err_msg = "Failed to send config to agent (channel closed).";
            warn!(vps_id = vps_id, "Agent channel closed, could not send config.");
            if let Err(e) = db_services::update_vps_config_status(&app_state.db_pool, vps_id, "failed", Some(err_msg)).await {
                 error!(vps_id = vps_id, error = ?e, "Failed to update VPS config status to failed (send error).");
            }
        }
    } else {
        let err_msg = "Agent is not connected.";
        warn!(vps_id = vps_id, "Agent not connected, could not send config.");
        if let Err(e) = db_services::update_vps_config_status(&app_state.db_pool, vps_id, "failed", Some(err_msg)).await {
            error!(vps_id = vps_id, error = ?e, "Failed to update VPS config status to failed (not connected).");
        }
    }

    Ok(())
}

/// Calculates the effective configuration for a given VPS.
pub async fn get_effective_vps_config(
    app_state: Arc<AppState>,
    vps_id: i32,
) -> Result<AgentConfig, AppError> {
    // 1. Get global config
    let global_config_setting_model: setting::Model = db_services::get_setting(&app_state.db_pool, "global_agent_config")
        .await
        .map_err(|db_err| AppError::DatabaseError(db_err.to_string()))?
        .ok_or_else(|| AppError::NotFound("Global agent config not found.".to_string()))?;
    
    let mut effective_config: AgentConfig = serde_json::from_value(global_config_setting_model.value)
        .map_err(|e| AppError::ServerError(format!("Failed to parse global config: {e}")))?;

    // 2. Get VPS and merge override if it exists
    let vps_model: vps::Model = db_services::get_vps_by_id(&app_state.db_pool, vps_id)
        .await
        .map_err(|db_err| AppError::DatabaseError(db_err.to_string()))?
        .ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;

    if let Some(override_json) = vps_model.agent_config_override {
        let override_config: AgentConfig = serde_json::from_value(override_json)
            .map_err(|e| AppError::ServerError(format!("Failed to parse override config: {e}")))?;
        
        // This is a simple merge. A more sophisticated merge might be needed.
        if override_config.metrics_collect_interval_seconds > 0 { effective_config.metrics_collect_interval_seconds = override_config.metrics_collect_interval_seconds; }
        if override_config.metrics_upload_batch_max_size > 0 { effective_config.metrics_upload_batch_max_size = override_config.metrics_upload_batch_max_size; }
        if override_config.metrics_upload_interval_seconds > 0 { effective_config.metrics_upload_interval_seconds = override_config.metrics_upload_interval_seconds; }
        if override_config.docker_info_collect_interval_seconds > 0 { effective_config.docker_info_collect_interval_seconds = override_config.docker_info_collect_interval_seconds; }
        if override_config.docker_info_upload_interval_seconds > 0 { effective_config.docker_info_upload_interval_seconds = override_config.docker_info_upload_interval_seconds; }
        if override_config.generic_metrics_upload_batch_max_size > 0 { effective_config.generic_metrics_upload_batch_max_size = override_config.generic_metrics_upload_batch_max_size; }
        if override_config.generic_metrics_upload_interval_seconds > 0 { effective_config.generic_metrics_upload_interval_seconds = override_config.generic_metrics_upload_interval_seconds; }
        if !override_config.log_level.is_empty() { effective_config.log_level = override_config.log_level; }
        if override_config.heartbeat_interval_seconds > 0 { effective_config.heartbeat_interval_seconds = override_config.heartbeat_interval_seconds; }
        effective_config.feature_flags.extend(override_config.feature_flags);
    }

    // 3. Get service monitor tasks for this agent
    let tasks = db_services::service_monitor_service::get_tasks_for_agent(&app_state.db_pool, vps_id)
        .await
        .map_err(|e| AppError::DatabaseError(format!("Failed to get monitor tasks for agent {vps_id}: {e}")))?;
    effective_config.service_monitor_tasks = tasks;

    Ok(effective_config)
}