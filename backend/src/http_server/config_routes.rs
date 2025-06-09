use axum::{
    extract::{Path, State},
    routing::{get, post, put},
    Json, Router, http::StatusCode,
};
use std::sync::Arc;
use crate::http_server::AppState;
use crate::agent_service::{AgentConfig, MessageToAgent, message_to_agent::Payload as AgentPayload, UpdateConfigRequest};
use crate::db::services as db_services;
use uuid::Uuid;

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
}

#[axum::debug_handler]
async fn get_global_agent_config(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<AgentConfig>, (StatusCode, String)> {
    let setting = db_services::get_setting(&app_state.db_pool, "global_agent_config")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    match setting {
        Some(s) => {
            let config: AgentConfig = serde_json::from_value(s.value)
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse global config: {}", e)))?;
            Ok(Json(config))
        },
        None => Err((StatusCode::NOT_FOUND, "Global agent config not found.".to_string())),
    }
}

#[axum::debug_handler]
async fn update_global_agent_config(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<AgentConfig>,
) -> Result<StatusCode, (StatusCode, String)> {
    let value = serde_json::to_value(&payload)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Failed to serialize config: {}", e)))?;
    
    db_services::update_setting(&app_state.db_pool, "global_agent_config", &value)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Trigger config push to all agents that don't have an override.
    let all_vps = db_services::get_vps_by_user_id(&app_state.db_pool, 1).await // Assuming user_id 1 for now
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    for vps in all_vps {
        if vps.agent_config_override.is_none() {
            let _ = push_config_to_vps(app_state.clone(), vps.id).await;
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
) -> Result<StatusCode, (StatusCode, String)> {
    let user_id = 1; // For now, user_id is hardcoded.
    
    db_services::update_vps_config_override(&app_state.db_pool, vps_id, user_id, &payload)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Trigger config push to this specific agent.
    push_config_to_vps(app_state, vps_id).await?;

    Ok(StatusCode::OK)
}

#[axum::debug_handler]
async fn retry_config_push(
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
) -> Result<StatusCode, (StatusCode, String)> {
    push_config_to_vps(app_state, vps_id).await?;
    Ok(StatusCode::OK)
}

/// Gets the effective config for a VPS and pushes it to the agent if connected.
async fn push_config_to_vps(
    app_state: Arc<AppState>,
    vps_id: i32,
) -> Result<(), (StatusCode, String)> {
    // 1. Get global config
    let global_config_setting = db_services::get_setting(&app_state.db_pool, "global_agent_config")
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::INTERNAL_SERVER_ERROR, "Global agent config not found.".to_string()))?;
    
    let mut effective_config: AgentConfig = serde_json::from_value(global_config_setting.value)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse global config: {}", e)))?;

    // 2. Get VPS and merge override if it exists
    let vps = db_services::get_vps_by_id(&app_state.db_pool, vps_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "VPS not found".to_string()))?;

    if let Some(override_json) = vps.agent_config_override {
        let override_config: AgentConfig = serde_json::from_value(override_json)
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to parse override config: {}", e)))?;
        
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

    // 3. Find agent and send message
    let agent_state = {
        let agents_guard = app_state.connected_agents.lock().await;
        agents_guard.find_by_vps_id(vps_id)
    };

    if let Some(state) = agent_state {
        let config_version_id = Uuid::new_v4().to_string();
        let update_req = UpdateConfigRequest {
            new_config: Some(effective_config),
            config_version_id,
        };
        let msg = MessageToAgent {
            server_message_id: 0, // This should be managed better, but 0 for now.
            payload: Some(AgentPayload::UpdateConfigRequest(update_req)),
        };

        if state.sender.send(Ok(msg)).await.is_ok() {
            db_services::update_vps_config_status(&app_state.db_pool, vps_id, "pending", None).await.ok();
        } else {
            let err_msg = "Failed to send config to agent (channel closed).";
            db_services::update_vps_config_status(&app_state.db_pool, vps_id, "failed", Some(err_msg)).await.ok();
        }
    } else {
        let err_msg = "Agent is not connected.";
        db_services::update_vps_config_status(&app_state.db_pool, vps_id, "failed", Some(err_msg)).await.ok();
    }

    Ok(())
}