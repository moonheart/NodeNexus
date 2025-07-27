use crate::db::duckdb_service::service_monitor_service;
use crate::web::config_routes::push_config_to_vps;
use crate::web::models::service_monitor_models::{
    CreateMonitor, ServiceMonitorResultDetails, UpdateMonitor,
};
use crate::web::routes::vps_routes::{parse_interval_to_seconds, MonitorTimeseriesQuery};
use crate::web::{AppError, AppState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::error;

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
        service_monitor_service::get_monitors_with_details_by_user_id(app_state.duckdb_pool.clone(), user_id)
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
        service_monitor_service::create_monitor(app_state.duckdb_pool.clone(), user_id, payload).await?;

    let affected_vps_ids =
        service_monitor_service::get_vps_ids_for_monitor(app_state.duckdb_pool.clone(), created_monitor.id)
            .await?;
    for vps_id in affected_vps_ids {
        if let Err(e) = push_config_to_vps(app_state.clone(), vps_id).await {
            error!(vps_id = vps_id, error = ?e, "Failed to push config to VPS after creating monitor.");
        }
    }

    let monitor_details =
        service_monitor_service::get_monitor_details_by_id(app_state.duckdb_pool.clone(), created_monitor.id)
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
    let monitor = service_monitor_service::get_monitor_details_by_id(app_state.duckdb_pool.clone(), id)
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
        service_monitor_service::update_monitor(app_state.duckdb_pool.clone(), id, user_id, payload).await?;

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
        service_monitor_service::get_vps_ids_for_monitor(app_state.duckdb_pool.clone(), id).await?;

    let delete_result =
        service_monitor_service::delete_monitor(app_state.duckdb_pool.clone(), id, user_id).await?;

    if delete_result == 0 {
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

#[axum::debug_handler]
async fn get_monitor_results(
    State(app_state): State<Arc<AppState>>,
    Path(id): Path<i32>,
    Query(query): Query<MonitorTimeseriesQuery>,
    // TODO: Add user extraction and authorization
) -> Result<Json<Vec<ServiceMonitorResultDetails>>, AppError> {
    // Fetch the monitor to get its name and verify existence
    let monitor = service_monitor_service::get_monitor_details_by_id(app_state.duckdb_pool.clone(), id)
        .await?
        .ok_or_else(|| AppError::NotFound("Monitor not found".to_string()))?;

    let interval_seconds = parse_interval_to_seconds(query.interval);

    let points = service_monitor_service::get_monitor_results_by_id(
        app_state.duckdb_pool.clone(),
        id,
        query.start_time,
        query.end_time,
        interval_seconds,
    )
    .await?;

    if points.is_empty() {
        return Ok(Json(Vec::new()));
    }

    // --- Conversion from ServiceMonitorPoint to ServiceMonitorResultDetails ---
    // 1. Get all unique agent IDs from the results
    let agent_ids: Vec<i32> = points.iter().map(|p| p.agent_id).collect::<Vec<_>>();

    // 2. Fetch all agent (VPS) models for these IDs
    // This part needs to be refactored to use the duckdb_service
    // For now, we'll have to query the vps service from duckdb
    let agents = crate::db::duckdb_service::vps_service::get_vps_by_ids(app_state.duckdb_pool.clone(), agent_ids).await?;
    let agent_name_map: HashMap<i32, String> =
        agents.into_iter().map(|a| (a.id, a.name)).collect();

    // 3. Map the points to the final details struct
    let results = points
        .into_iter()
        .map(|point| {
            let agent_name = agent_name_map
                .get(&point.agent_id)
                .cloned()
                .unwrap_or_else(|| "Unknown Agent".to_string());

            ServiceMonitorResultDetails {
                time: point.time.to_rfc3339(),
                monitor_id: point.monitor_id,
                agent_id: point.agent_id,
                agent_name,
                monitor_name: monitor.name.clone(),
                is_up: point.is_up.is_some_and(|v| v > 0.5),
                latency_ms: point.latency_ms.map(|f| f as i32),
                details: point.details,
            }
        })
        .collect();

    Ok(Json(results))
}
