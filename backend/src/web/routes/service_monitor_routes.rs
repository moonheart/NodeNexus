use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::sync::Arc;
use tracing::error;

use crate::db::services::service_monitor_service;
use crate::web::config_routes::push_config_to_vps;
use crate::web::models::service_monitor_models::{CreateMonitor, UpdateMonitor};
use crate::web::{AppError, AppState};

pub fn create_service_monitor_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(list_monitors).post(create_monitor))
        .route(
            "/{id}",
            get(get_monitor).put(update_monitor).delete(delete_monitor),
        )
        .route("/{id}/results", get(get_monitor_results))
}

#[axum::debug_handler]
async fn list_monitors(
    State(app_state): State<Arc<AppState>>,
    // TODO: Add user extraction
) -> Result<Json<Vec<crate::web::models::service_monitor_models::ServiceMonitorDetails>>, AppError>
{
    let user_id = 1; // Hardcoded user_id
    let monitors =
        service_monitor_service::get_monitors_with_details_by_user_id(&app_state.db_pool, user_id)
            .await?;
    Ok(Json(monitors))
}

#[axum::debug_handler]
async fn create_monitor(
    State(app_state): State<Arc<AppState>>,
    // TODO: Add user extraction
    Json(payload): Json<CreateMonitor>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let user_id = 1; // Hardcoded user_id
    let created_monitor =
        service_monitor_service::create_monitor(&app_state.db_pool, user_id, payload).await?;

    let affected_vps_ids =
        service_monitor_service::get_vps_ids_for_monitor(&app_state.db_pool, created_monitor.id)
            .await?;
    for vps_id in affected_vps_ids {
        if let Err(e) = push_config_to_vps(app_state.clone(), vps_id).await {
            error!(vps_id = vps_id, error = ?e, "Failed to push config to VPS after creating monitor.");
        }
    }

    let monitor_details =
        service_monitor_service::get_monitor_details_by_id(&app_state.db_pool, created_monitor.id)
            .await?
            .ok_or_else(|| {
                AppError::NotFound("Failed to fetch created monitor details".to_string())
            })?;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::to_value(monitor_details).unwrap()),
    ))
}

#[axum::debug_handler]
async fn get_monitor(
    State(app_state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    // TODO: Add user extraction and authorization
) -> Result<Json<crate::web::models::service_monitor_models::ServiceMonitorDetails>, AppError> {
    let monitor = service_monitor_service::get_monitor_details_by_id(&app_state.db_pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound("Monitor not found".to_string()))?;
    Ok(Json(monitor))
}

#[axum::debug_handler]
async fn update_monitor(
    State(app_state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    // TODO: Add user extraction
    Json(payload): Json<UpdateMonitor>,
) -> Result<Json<crate::web::models::service_monitor_models::ServiceMonitorDetails>, AppError> {
    let user_id = 1; // Hardcoded user_id
    let (updated_details, affected_vps_ids) =
        service_monitor_service::update_monitor(&app_state.db_pool, id, user_id, payload).await?;

    for vps_id in affected_vps_ids {
        if let Err(e) = push_config_to_vps(app_state.clone(), vps_id).await {
            error!(vps_id = vps_id, error = ?e, "Failed to push config to VPS after updating monitor.");
        }
    }

    Ok(Json(updated_details))
}

#[axum::debug_handler]
async fn delete_monitor(
    State(app_state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    // TODO: Add user extraction
) -> Result<StatusCode, AppError> {
    let user_id = 1; // Hardcoded user_id

    let affected_vps_ids =
        service_monitor_service::get_vps_ids_for_monitor(&app_state.db_pool, id).await?;

    let delete_result =
        service_monitor_service::delete_monitor(&app_state.db_pool, id, user_id).await?;

    if delete_result.rows_affected == 0 {
        return Err(AppError::NotFound(
            "Monitor not found or permission denied".to_string(),
        ));
    }

    for vps_id in affected_vps_ids {
        if let Err(e) = push_config_to_vps(app_state.clone(), vps_id).await {
            error!(vps_id = vps_id, error = ?e, "Failed to push config to VPS after deleting monitor.");
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
pub struct MonitorResultsQuery {
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub limit: Option<u64>,
}

#[axum::debug_handler]
async fn get_monitor_results(
    State(app_state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Query(query): Query<MonitorResultsQuery>,
    // TODO: Add user extraction and authorization
) -> Result<
    Json<Vec<crate::web::models::service_monitor_models::ServiceMonitorResultDetails>>,
    AppError,
> {
    let results = service_monitor_service::get_monitor_results_by_id(
        &app_state.db_pool,
        id,
        query.start_time,
        query.end_time,
        query.limit,
    )
    .await?;
    Ok(Json(results))
}
