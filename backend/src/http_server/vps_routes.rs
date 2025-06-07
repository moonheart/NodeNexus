use axum::{
    extract::{State, Extension, Path}, // Added Path
    http::StatusCode,
    routing::{get, post, put, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize}; // Added Serialize
use std::sync::Arc;
use crate::db::{models::{Vps, PerformanceMetric as DbPerformanceMetric, Tag}, services};
use super::{AppState, AppError, config_routes};
use crate::http_server::auth_logic::AuthenticatedUser;
use crate::server::update_service;

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
    // agent_secret is sensitive and should not be sent to the list view.
    // pub agent_secret: String,
    pub status: String,
    // metadata is large and not needed for the list view.
    // pub metadata: Option<serde_json::Value>,
    pub created_at: String,
    // updated_at is not currently used in the UI list.
    // pub updated_at: String,
    #[serde(rename = "group")]
    pub group: Option<String>,
    pub tags: Option<Vec<crate::websocket_models::Tag>>,
    pub latest_metrics: Option<crate::websocket_models::ServerMetricsSnapshot>,
    pub config_status: String,
    pub last_config_update_at: Option<String>,
    pub last_config_error: Option<String>,
    // agent_config_override is not needed for the list view.
    // pub agent_config_override: Option<serde_json::Value>,
}

// This converts the unified `ServerWithDetails` model (used by websockets)
// into the `VpsListItemResponse` model (used by the REST API).
// This ensures data consistency between initial load (REST) and updates (WS).
impl From<crate::websocket_models::ServerWithDetails> for VpsListItemResponse {
    fn from(details: crate::websocket_models::ServerWithDetails) -> Self {
        Self {
            id: details.basic_info.id,
            user_id: details.basic_info.user_id,
            name: details.basic_info.name,
            ip_address: details.basic_info.ip_address,
            os_type: details.os_type,
            status: details.basic_info.status,
            created_at: details.created_at.to_rfc3339(),
            group: details.basic_info.group,
            tags: details.basic_info.tags,
            latest_metrics: details.latest_metrics,
            config_status: details.basic_info.config_status,
            last_config_update_at: details.basic_info.last_config_update_at.map(|dt| dt.to_rfc3339()),
            last_config_error: details.basic_info.last_config_error,
        }
    }
}


#[derive(Deserialize)]
pub struct CreateVpsRequest {
    name: String,
}

#[derive(Deserialize)]
pub struct AddTagToVpsRequest {
    tag_id: i32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkUpdateTagsRequest {
    vps_ids: Vec<i32>,
    add_tag_ids: Vec<i32>,
    remove_tag_ids: Vec<i32>,
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
    // Use the unified query that fetches everything, including tags.
    // Use the unified query that fetches everything for the specific user, including tags.
    // Use the new, user-specific unified query that fetches everything, including tags.
    let server_details_list = services::get_all_vps_with_details_for_user(&app_state.db_pool, user_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Convert to the response type. Filtering is now correctly done in the database query.
    let response_list: Vec<VpsListItemResponse> = server_details_list
        .into_iter()
        .map(|details| details.into())
        .collect();

    Ok(Json(response_list))
}

async fn get_vps_detail_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
) -> Result<Json<VpsListItemResponse>, AppError> {
    let user_id = authenticated_user.id;
    // Use the unified query that fetches everything, including tags.
    let vps_details = services::get_vps_with_details_for_cache_by_id(&app_state.db_pool, vps_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;

    // TODO: Authorize user
    // if vps_details.basic_info.user_id != user_id {
    //     return Err(AppError::Unauthorized("Access denied".to_string()));
    // }
    
    Ok(Json(vps_details.into()))
}

#[derive(Deserialize)]
pub struct UpdateVpsRequest {
    name: Option<String>,
    group: Option<String>,
    tag_ids: Option<Vec<i32>>,
}


async fn update_vps_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
    Json(payload): Json<UpdateVpsRequest>,
) -> Result<StatusCode, AppError> {
    let user_id = authenticated_user.id;

                            // First, verify the user owns this VPS
    let vps = services::get_vps_by_id(&app_state.db_pool, vps_id).await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;

    if vps.user_id != user_id {
        return Err(AppError::Unauthorized("Access denied".to_string()));
    }

    // Proceed with the update
    let change_detected = services::update_vps(
        &app_state.db_pool,
        vps_id,
        user_id,
        payload.name,
        payload.group,
        payload.tag_ids,
    ).await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if change_detected {
        // Call the centralized broadcast function
        update_service::broadcast_full_state_update(
            &app_state.db_pool,
            &app_state.live_server_data_cache,
            &app_state.ws_data_broadcaster_tx,
        )
        .await;
        Ok(StatusCode::OK)
    } else {
        Ok(StatusCode::NOT_MODIFIED)
    }
}


// --- VPS Tag Handlers ---

async fn add_tag_to_vps_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
    Json(payload): Json<AddTagToVpsRequest>,
) -> Result<StatusCode, AppError> {
    let user_id = authenticated_user.id;
    // Authorize: Check if user owns the VPS
    let vps = services::get_vps_by_id(&app_state.db_pool, vps_id).await.map_err(|e| AppError::DatabaseError(e.to_string()))?.ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;
    if vps.user_id != user_id {
        return Err(AppError::Unauthorized("Permission denied to VPS".to_string()));
    }

    // TODO: Authorize: Check if user owns the Tag as well? For now, we assume if they can see it, they can use it.

    services::add_tag_to_vps(&app_state.db_pool, vps_id, payload.tag_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(StatusCode::CREATED)
}

async fn remove_tag_from_vps_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path((vps_id, tag_id)): Path<(i32, i32)>,
) -> Result<StatusCode, AppError> {
    let user_id = authenticated_user.id;
    // Authorize: Check if user owns the VPS
    let vps = services::get_vps_by_id(&app_state.db_pool, vps_id).await.map_err(|e| AppError::DatabaseError(e.to_string()))?.ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;
    if vps.user_id != user_id {
        return Err(AppError::Unauthorized("Permission denied".to_string()));
    }

    let rows_affected = services::remove_tag_from_vps(&app_state.db_pool, vps_id, tag_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if rows_affected > 0 {
        Ok(StatusCode::NO_CONTENT)
    } else {
        // This could also mean the tag wasn't associated in the first place
        Ok(StatusCode::NOT_FOUND)
    }
}

async fn get_tags_for_vps_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
) -> Result<Json<Vec<Tag>>, AppError> {
    let user_id = authenticated_user.id;
    // Authorize: Check if user owns the VPS
    let vps = services::get_vps_by_id(&app_state.db_pool, vps_id).await.map_err(|e| AppError::DatabaseError(e.to_string()))?.ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;
    if vps.user_id != user_id {
        return Err(AppError::Unauthorized("Permission denied".to_string()));
    }

    let tags = services::get_tags_for_vps(&app_state.db_pool, vps_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    
    Ok(Json(tags))
}


pub fn vps_tags_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(add_tag_to_vps_handler).get(get_tags_for_vps_handler))
        .route("/{tag_id}", delete(remove_tag_from_vps_handler))
}

async fn bulk_update_vps_tags_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<BulkUpdateTagsRequest>,
) -> Result<StatusCode, AppError> {
    let user_id = authenticated_user.id;

    if payload.vps_ids.is_empty() {
        return Ok(StatusCode::OK); // Nothing to do
    }

    match services::bulk_update_vps_tags(
        &app_state.db_pool,
        user_id,
        &payload.vps_ids,
        &payload.add_tag_ids,
        &payload.remove_tag_ids,
    )
    .await
    {
        Ok(_) => {
            // Call the centralized broadcast function
            update_service::broadcast_full_state_update(
                &app_state.db_pool,
                &app_state.live_server_data_cache,
                &app_state.ws_data_broadcaster_tx,
            )
            .await;
            Ok(StatusCode::OK)
        }
        Err(sqlx::Error::RowNotFound) => {
            // This custom error indicates an authorization failure (user doesn't own all VPS)
            Err(AppError::Unauthorized("Permission denied to one or more VPS".to_string()))
        }
        Err(e) => Err(AppError::DatabaseError(e.to_string())),
    }
}

pub fn vps_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_vps_handler))
        .route("/", get(get_all_vps_handler))
        .route("/bulk-actions", post(bulk_update_vps_tags_handler))
        .route("/{vps_id}", get(get_vps_detail_handler))
        .route("/{vps_id}", put(update_vps_handler))
        .nest("/{vps_id}/tags", vps_tags_router()) // Nest the tags router
        .merge(config_routes::create_vps_config_router())
}