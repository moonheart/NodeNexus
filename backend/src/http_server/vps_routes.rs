use axum::{
    extract::{State, Extension, Path}, // Added Path
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize}; // Added Serialize
use std::sync::Arc;
use crate::db::{models::{Vps, PerformanceMetric as DbPerformanceMetric}, services};
use super::{AppState, AppError};
use crate::http_server::auth_logic::AuthenticatedUser;

// Frontend expects this structure for latest metrics
#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LatestPerformanceMetricResponse {
    pub id: i32,
    pub time: String, // DateTime<Utc>
    pub vps_id: i32,
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: i64,
    pub memory_total_bytes: i64,
    pub swap_usage_bytes: i64,
    pub swap_total_bytes: i64,
    pub disk_io_read_bps: i64,
    pub disk_io_write_bps: i64,
    pub network_rx_bps: i64,      // Cumulative
    pub network_tx_bps: i64,      // Cumulative
    pub network_rx_instant_bps: i64,
    pub network_tx_instant_bps: i64,
    pub uptime_seconds: i64,
    pub total_processes_count: i32,
    pub running_processes_count: i32,
    pub tcp_established_connection_count: i32,
    pub disk_total_bytes: Option<i64>,
    pub disk_used_bytes: Option<i64>,
}

impl From<(DbPerformanceMetric, Option<(i64, i64)>)> for LatestPerformanceMetricResponse {
    fn from((metric, disk_summary): (DbPerformanceMetric, Option<(i64, i64)>)) -> Self {
        Self {
            id: metric.id,
            time: metric.time.to_rfc3339(),
            vps_id: metric.vps_id,
            cpu_usage_percent: metric.cpu_usage_percent,
            memory_usage_bytes: metric.memory_usage_bytes,
            memory_total_bytes: metric.memory_total_bytes,
            swap_usage_bytes: metric.swap_usage_bytes,
            swap_total_bytes: metric.swap_total_bytes,
            disk_io_read_bps: metric.disk_io_read_bps,
            disk_io_write_bps: metric.disk_io_write_bps,
            network_rx_bps: metric.network_rx_bps,
            network_tx_bps: metric.network_tx_bps,
            network_rx_instant_bps: metric.network_rx_instant_bps,
            network_tx_instant_bps: metric.network_tx_instant_bps,
            uptime_seconds: metric.uptime_seconds,
            total_processes_count: metric.total_processes_count,
            running_processes_count: metric.running_processes_count,
            tcp_established_connection_count: metric.tcp_established_connection_count,
            disk_total_bytes: disk_summary.map(|(total, _)| total),
            disk_used_bytes: disk_summary.map(|(_, used)| used),
        }
    }
}


#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct VpsListItemResponse {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub ip_address: Option<String>,
    pub os_type: Option<String>,
    pub agent_secret: String,
    pub status: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
    pub latest_metrics: Option<LatestPerformanceMetricResponse>,
}

impl From<(Vps, Option<LatestPerformanceMetricResponse>)> for VpsListItemResponse {
    fn from((vps, latest_metrics): (Vps, Option<LatestPerformanceMetricResponse>)) -> Self {
        Self {
            id: vps.id,
            user_id: vps.user_id,
            name: vps.name,
            ip_address: vps.ip_address,
            os_type: vps.os_type,
            agent_secret: vps.agent_secret,
            status: vps.status,
            metadata: vps.metadata,
            created_at: vps.created_at.to_rfc3339(),
            updated_at: vps.updated_at.to_rfc3339(),
            latest_metrics,
        }
    }
}


#[derive(Deserialize)]
pub struct CreateVpsRequest {
    name: String,
}

async fn create_vps_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<CreateVpsRequest>,
) -> Result<(StatusCode, Json<Vps>), AppError> { // Create still returns the base Vps model
    let user_id = authenticated_user.id;
    match services::create_vps(&app_state.db_pool, user_id, &payload.name).await {
        Ok(vps) => Ok((StatusCode::CREATED, Json(vps))),
        Err(sqlx_error) => {
            eprintln!("Failed to create VPS: {:?}", sqlx_error);
            Err(AppError::DatabaseError(sqlx_error.to_string()))
        }
    }
}

async fn get_all_vps_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Vec<VpsListItemResponse>>, AppError> {
    let user_id = authenticated_user.id;
    let vps_list_db = services::get_all_vps_for_user(&app_state.db_pool, user_id).await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let mut response_list = Vec::new();
    for vps_db in vps_list_db {
        let latest_metric_db = services::get_latest_performance_metric_for_vps(&app_state.db_pool, vps_db.id).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let latest_disk_summary = services::get_latest_disk_usage_summary(&app_state.db_pool, vps_db.id).await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let latest_metrics_response = latest_metric_db.map(|m| (m, latest_disk_summary).into());
        response_list.push((vps_db, latest_metrics_response).into());
    }

    Ok(Json(response_list))
}

async fn get_vps_detail_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
) -> Result<Json<VpsListItemResponse>, AppError> {
    let user_id = authenticated_user.id;
    let vps_db = services::get_vps_by_id(&app_state.db_pool, vps_id).await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;

    if vps_db.user_id != user_id {
        return Err(AppError::Unauthorized("Access denied".to_string()));
    }

    let latest_metric_db = services::get_latest_performance_metric_for_vps(&app_state.db_pool, vps_db.id).await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let latest_disk_summary = services::get_latest_disk_usage_summary(&app_state.db_pool, vps_db.id).await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    let latest_metrics_response = latest_metric_db.map(|m| (m, latest_disk_summary).into());
    
    Ok(Json((vps_db, latest_metrics_response).into()))
}


pub fn vps_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_vps_handler))
        .route("/", get(get_all_vps_handler))
        .route("/{vps_id}", get(get_vps_detail_handler)) // Added detail route
}