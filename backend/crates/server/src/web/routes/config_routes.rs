use crate::db::duckdb_service::{self, settings_service, vps_service};
use crate::web::{models::config_models::WebAgentConfig, AppError, AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use futures_util::SinkExt;
use nodenexus_common::agent_service::{
    message_to_agent::Payload as AgentPayload, AgentConfig, MessageToAgent, UpdateConfigRequest,
};
use std::sync::Arc;
use tracing::{error, warn};
use uuid::Uuid;

pub fn create_settings_router() -> Router<Arc<AppState>> {
    Router::new().route(
        "/agent-config",
        get(get_global_agent_config).put(update_global_agent_config),
    )
}

pub fn create_vps_config_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{id}/config-override", put(update_vps_config_override))
        .route("/{id}/retry-config", post(retry_config_push))
        .route("/{id}/push-config", post(retry_config_push))
        .route("/{id}/config-preview", get(preview_vps_config))
}

async fn get_global_agent_config(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<WebAgentConfig>, AppError> {
    let setting_model =
        duckdb_service::settings_service::get_setting(app_state.duckdb_pool.clone(), "global_agent_config")
            .await?
            .ok_or_else(|| AppError::NotFound("Global agent config not found.".to_string()))?;

    let config: AgentConfig = serde_json::from_value(setting_model.value)?;
    Ok(Json(config.into()))
}

async fn update_global_agent_config(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<WebAgentConfig>,
) -> Result<StatusCode, AppError> {
    let proto_config: AgentConfig = payload.into();
    let value = serde_json::to_value(&proto_config)?;

    settings_service::update_setting(app_state.duckdb_pool.clone(), "global_agent_config", &value).await?;

    let all_vps_models = vps_service::get_vps_by_user_id(app_state.duckdb_pool.clone(), 1).await?; // Assuming user_id 1 for admin

    for vps_model in all_vps_models {
        if vps_model.agent_config_override.is_none() {
            if let Err(e) = push_config_to_vps(app_state.clone(), vps_model.id).await {
                error!(vps_id = vps_model.id, error = ?e, "Failed to push config to VPS after global update.");
            }
        }
    }

    Ok(StatusCode::OK)
}

async fn update_vps_config_override(
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
    Json(payload): Json<serde_json::Value>,
) -> Result<StatusCode, AppError> {
    let user_id = 1; // TODO: Replace with actual authenticated user ID

    settings_service::update_vps_config_override(
        app_state.duckdb_pool.clone(),
        vps_id,
        user_id,
        &payload,
    )
    .await?;

    push_config_to_vps(app_state, vps_id).await?;
    Ok(StatusCode::OK)
}

async fn retry_config_push(
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    push_config_to_vps(app_state, vps_id).await?;
    Ok(StatusCode::OK)
}

async fn preview_vps_config(
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
) -> Result<Json<WebAgentConfig>, AppError> {
    let effective_config = get_effective_vps_config(app_state.duckdb_pool.clone(), vps_id).await?;
    Ok(Json(effective_config.into()))
}

pub async fn push_config_to_vps(app_state: Arc<AppState>, vps_id: i32) -> Result<(), AppError> {
    let effective_config = get_effective_vps_config(app_state.duckdb_pool.clone(), vps_id).await?;

    let agent_state = {
        let agents_guard = app_state.connected_agents.lock().await;
        agents_guard.find_by_vps_id(vps_id)
    };

    if let Some(mut state) = agent_state {
        let config_version_id = Uuid::new_v4().to_string();
        let update_req = UpdateConfigRequest {
            new_config: Some(effective_config),
            config_version_id,
        };
        let msg = MessageToAgent {
            server_message_id: 0,
            payload: Some(AgentPayload::UpdateConfigRequest(update_req)),
        };

        if state.sender.send(msg).await.is_ok() {
            if let Err(e) =
                settings_service::update_vps_config_status(app_state.duckdb_pool.clone(), vps_id, "pending", None)
                    .await
            {
                error!(vps_id = vps_id, error = ?e, "Failed to update VPS config status to pending.");
            }
        } else {
            let err_msg = "Failed to send config to agent (channel closed).";
            warn!(vps_id = vps_id, "{}", err_msg);
            if let Err(e) = settings_service::update_vps_config_status(
                app_state.duckdb_pool.clone(),
                vps_id,
                "failed",
                Some(err_msg),
            )
            .await
            {
                error!(vps_id = vps_id, error = ?e, "Failed to update VPS config status to failed (send error).");
            }
        }
    } else {
        let err_msg = "Agent is not connected.";
        warn!(vps_id = vps_id, "{}", err_msg);
        if let Err(e) = settings_service::update_vps_config_status(
            app_state.duckdb_pool.clone(),
            vps_id,
            "failed",
            Some(err_msg),
        )
        .await
        {
            error!(vps_id = vps_id, error = ?e, "Failed to update VPS config status to failed (not connected).");
        }
    }

    Ok(())
}

pub async fn get_effective_vps_config(
    db_pool: duckdb_service::DuckDbPool,
    vps_id: i32,
) -> Result<AgentConfig, AppError> {
    let global_config_setting = settings_service::get_setting(db_pool.clone(), "global_agent_config")
        .await?
        .ok_or_else(|| AppError::NotFound("Global agent config not found.".to_string()))?;

    let mut effective_config: AgentConfig = serde_json::from_value(global_config_setting.value)?;

    let vps_model = vps_service::get_vps_by_id(db_pool.clone(), vps_id)
        .await?
        .ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;

    if let Some(override_json) = vps_model.agent_config_override {
        let override_config: AgentConfig = serde_json::from_value(override_json)?;
        
        // Simple merge logic
        if override_config.metrics_collect_interval_seconds > 0 {
            effective_config.metrics_collect_interval_seconds =
                override_config.metrics_collect_interval_seconds;
        }
        if override_config.metrics_upload_batch_max_size > 0 {
            effective_config.metrics_upload_batch_max_size =
                override_config.metrics_upload_batch_max_size;
        }
        if override_config.metrics_upload_interval_seconds > 0 {
            effective_config.metrics_upload_interval_seconds =
                override_config.metrics_upload_interval_seconds;
        }
        if override_config.docker_info_collect_interval_seconds > 0 {
            effective_config.docker_info_collect_interval_seconds =
                override_config.docker_info_collect_interval_seconds;
        }
        if override_config.docker_info_upload_interval_seconds > 0 {
            effective_config.docker_info_upload_interval_seconds =
                override_config.docker_info_upload_interval_seconds;
        }
        if override_config.generic_metrics_upload_batch_max_size > 0 {
            effective_config.generic_metrics_upload_batch_max_size =
                override_config.generic_metrics_upload_batch_max_size;
        }
        if override_config.generic_metrics_upload_interval_seconds > 0 {
            effective_config.generic_metrics_upload_interval_seconds =
                override_config.generic_metrics_upload_interval_seconds;
        }
        if !override_config.log_level.is_empty() {
            effective_config.log_level = override_config.log_level;
        }
        effective_config
            .feature_flags
            .extend(override_config.feature_flags);
    }

    // TODO: Migrate service_monitor_service to get tasks
    // let tasks = duckdb_service::service_monitor_service::get_tasks_for_agent(db_pool, vps_id).await?;
    // effective_config.service_monitor_tasks = tasks;

    Ok(effective_config)
}
