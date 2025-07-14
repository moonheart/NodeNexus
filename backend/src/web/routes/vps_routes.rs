use crate::db::{
    entities::{service_monitor, tag, vps},
    models::PerformanceMetric as DbPerformanceMetric, // Keep DbPerformanceMetric for now
    services,
};
use crate::server::update_service;
use crate::web::models::service_monitor_models::ServiceMonitorResultDetails;
use crate::web::models::AuthenticatedUser;
use crate::web::{config_routes, AppError, AppState, routes::metrics_routes};
use axum::{
    extract::{Extension, Path, Query, State}, // Added Query
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc}; // Added for DateTime<Utc>
use sea_orm::{ColumnTrait, DbErr, EntityTrait, QueryFilter}; // Added DbErr for error handling
use serde::{Deserialize, Serialize}; // Added Serialize
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tracing::error;

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
    pub network_rx_bps: i64, // Cumulative
    pub network_tx_bps: i64, // Cumulative
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
    pub status: String,
    pub agent_version: Option<String>,
    pub created_at: String,
    #[serde(rename = "group")]
    pub group: Option<String>,
    pub tags: Option<Vec<crate::web::models::websocket_models::Tag>>,
    pub config_status: String,
    pub last_config_update_at: Option<String>,
    pub last_config_error: Option<String>,

    // Traffic monitoring fields
    pub traffic_limit_bytes: Option<i64>,
    pub traffic_billing_rule: Option<String>,
    pub traffic_current_cycle_rx_bytes: Option<i64>,
    pub traffic_current_cycle_tx_bytes: Option<i64>,
    pub traffic_last_reset_at: Option<String>,
    pub traffic_reset_config_type: Option<String>,
    pub traffic_reset_config_value: Option<String>,
    pub next_traffic_reset_at: Option<String>,

    // Renewal Info Fields (flattened)
    pub renewal_cycle: Option<String>,
    pub renewal_cycle_custom_days: Option<i32>,
    pub renewal_price: Option<f64>,
    pub renewal_currency: Option<String>,
    pub next_renewal_date: Option<String>, // DateTime<Utc> to String
    pub last_renewal_date: Option<String>, // DateTime<Utc> to String
    pub service_start_date: Option<String>, // DateTime<Utc> to String
    pub payment_method: Option<String>,
    pub auto_renew_enabled: Option<bool>,
    pub renewal_notes: Option<String>,
    pub reminder_active: Option<bool>,
    // last_reminder_generated_at is likely not needed by list view, but can be added if detail view needs it

    // Agent secret is only included in the detail view, not the list view.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_secret: Option<String>,
}

// This converts the unified `ServerWithDetails` model (used by websockets)
// into the `VpsListItemResponse` model (used by the REST API).
// This ensures data consistency between initial load (REST) and updates (WS).
// NOTE: This will require `ServerWithDetails` to be updated to include renewal fields.
impl From<crate::web::models::websocket_models::ServerWithDetails> for VpsListItemResponse {
    fn from(details: crate::web::models::websocket_models::ServerWithDetails) -> Self {
        Self {
            id: details.basic_info.id,
            user_id: details.basic_info.user_id,
            name: details.basic_info.name,
            ip_address: details.basic_info.ip_address,
            os_type: details.os_type,
            status: details.basic_info.status,
            agent_version: details.basic_info.agent_version,
            created_at: details.created_at.to_rfc3339(),
            group: details.basic_info.group,
            tags: details.basic_info.tags,
            config_status: details.basic_info.config_status,
            last_config_update_at: details
                .basic_info
                .last_config_update_at
                .map(|dt| dt.to_rfc3339()),
            last_config_error: details.basic_info.last_config_error,
            traffic_limit_bytes: details.basic_info.traffic_limit_bytes,
            traffic_billing_rule: details.basic_info.traffic_billing_rule,
            traffic_current_cycle_rx_bytes: details.basic_info.traffic_current_cycle_rx_bytes,
            traffic_current_cycle_tx_bytes: details.basic_info.traffic_current_cycle_tx_bytes,
            traffic_last_reset_at: details
                .basic_info
                .traffic_last_reset_at
                .map(|dt| dt.to_rfc3339()),
            traffic_reset_config_type: details.basic_info.traffic_reset_config_type,
            traffic_reset_config_value: details.basic_info.traffic_reset_config_value,
            next_traffic_reset_at: details
                .basic_info
                .next_traffic_reset_at
                .map(|dt| dt.to_rfc3339()),

            // Map renewal fields (assuming they will be added to ServerWithDetails or a similar joined struct)
            // For now, these will default to None or false if not present in ServerWithDetails.
            // This part will need to be updated when ServerWithDetails is modified.
            renewal_cycle: details.renewal_cycle.clone(),
            renewal_cycle_custom_days: details.renewal_cycle_custom_days,
            renewal_price: details.renewal_price,
            renewal_currency: details.renewal_currency.clone(),
            next_renewal_date: details.next_renewal_date.map(|dt| dt.to_rfc3339()),
            last_renewal_date: details.last_renewal_date.map(|dt| dt.to_rfc3339()),
            service_start_date: details.service_start_date.map(|dt| dt.to_rfc3339()),
            payment_method: details.payment_method.clone(),
            auto_renew_enabled: details.auto_renew_enabled,
            renewal_notes: details.renewal_notes.clone(),
            reminder_active: details.reminder_active,
            agent_secret: None, // Secret is never sent in the list view or via WebSocket
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkTriggerUpdateCheckRequest {
    vps_ids: Vec<i32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkActionResponse {
    message: String,
    successful_count: u32,
    failed_count: u32,
}

async fn create_vps_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<CreateVpsRequest>,
) -> Result<(StatusCode, Json<vps::Model>), AppError> {
    // Changed Vps to vps::Model
    let user_id = authenticated_user.id;
    match services::create_vps(&app_state.db_pool, user_id, &payload.name).await {
        Ok(vps_model) => {
            // After successful creation, broadcast the new state
            update_service::broadcast_full_state_update(
                &app_state.db_pool,
                &app_state.live_server_data_cache,
                &app_state.ws_data_broadcaster_tx,
            )
            .await;
            Ok((StatusCode::CREATED, Json(vps_model)))
        }
        Err(db_err) => {
            error!(error = ?db_err, "Failed to create VPS.");
            Err(AppError::DatabaseError(db_err.to_string()))
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
    let server_details_list =
        services::get_all_vps_with_details_for_user(&app_state.db_pool, user_id)
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

    // Fetch the detailed view model, which has almost everything
    let vps_details_for_view =
        services::get_vps_with_details_for_cache_by_id(&app_state.db_pool, vps_id)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;

    // Authorize user based on the fetched details
    if vps_details_for_view.basic_info.user_id != user_id {
        return Err(AppError::Unauthorized("Access denied".to_string()));
    }

    // Fetch the raw vps::Model to get the agent_secret
    let vps_model = services::get_vps_by_id(&app_state.db_pool, vps_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;

    // Convert the detailed view model to our response type
    let mut response: VpsListItemResponse = vps_details_for_view.into();

    // Securely add the agent secret to the response
    response.agent_secret = Some(vps_model.agent_secret);

    Ok(Json(response))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateVpsRequest {
    name: Option<String>,
    group: Option<String>,
    tag_ids: Option<Vec<i32>>,

    // Traffic monitoring config fields
    #[serde(default)]
    traffic_limit_bytes: Option<i64>,
    #[serde(default)]
    traffic_billing_rule: Option<String>,
    #[serde(default)]
    traffic_reset_config_type: Option<String>,
    #[serde(default)]
    traffic_reset_config_value: Option<String>,
    #[serde(default)]
    next_traffic_reset_at: Option<DateTime<Utc>>,

    // Renewal Info Fields
    #[serde(default)]
    renewal_cycle: Option<String>,
    #[serde(default)]
    renewal_cycle_custom_days: Option<i32>,
    #[serde(default)]
    renewal_price: Option<f64>,
    #[serde(default)]
    renewal_currency: Option<String>,
    #[serde(default)]
    next_renewal_date: Option<DateTime<Utc>>,
    #[serde(default)]
    last_renewal_date: Option<DateTime<Utc>>,
    #[serde(default)]
    service_start_date: Option<DateTime<Utc>>,
    #[serde(default)]
    payment_method: Option<String>,
    #[serde(default)]
    auto_renew_enabled: Option<bool>,
    #[serde(default)]
    renewal_notes: Option<String>,
    // reminder_active and last_reminder_generated_at are managed by backend, not set by client directly in update
}

async fn update_vps_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
    Json(payload): Json<UpdateVpsRequest>,
) -> Result<StatusCode, AppError> {
    let user_id = authenticated_user.id;

    // First, verify the user owns this VPS
    let vps = services::get_vps_by_id(&app_state.db_pool, vps_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;

    if vps.user_id != user_id {
        return Err(AppError::Unauthorized("Access denied".to_string()));
    }

    // Proceed with the update

    // Construct VpsRenewalDataInput from payload
    // Only create Some(VpsRenewalDataInput) if at least one renewal field is present in the payload.
    // This avoids unnecessary database operations if no renewal info is being updated.
    let renewal_input_opt = if payload.renewal_cycle.is_some()
        || payload.renewal_cycle_custom_days.is_some()
        || payload.renewal_price.is_some()
        || payload.renewal_currency.is_some()
        || payload.next_renewal_date.is_some()
        || payload.last_renewal_date.is_some()
        || payload.service_start_date.is_some()
        || payload.payment_method.is_some()
        || payload.auto_renew_enabled.is_some()
        || payload.renewal_notes.is_some()
    {
        Some(services::VpsRenewalDataInput {
            renewal_cycle: payload.renewal_cycle,
            renewal_cycle_custom_days: payload.renewal_cycle_custom_days,
            renewal_price: payload.renewal_price,
            renewal_currency: payload.renewal_currency,
            next_renewal_date: payload.next_renewal_date,
            last_renewal_date: payload.last_renewal_date,
            service_start_date: payload.service_start_date,
            payment_method: payload.payment_method,
            auto_renew_enabled: payload.auto_renew_enabled,
            renewal_notes: payload.renewal_notes,
        })
    } else {
        None
    };

    let change_detected = services::update_vps(
        &app_state.db_pool,
        vps_id,
        user_id,
        payload.name,
        payload.group,
        payload.tag_ids,
        payload.traffic_limit_bytes,
        payload.traffic_billing_rule,
        payload.traffic_reset_config_type,
        payload.traffic_reset_config_value,
        payload.next_traffic_reset_at,
        renewal_input_opt, // Pass the constructed renewal input
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

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
    let vps = services::get_vps_by_id(&app_state.db_pool, vps_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;
    if vps.user_id != user_id {
        return Err(AppError::Unauthorized(
            "Permission denied to VPS".to_string(),
        ));
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
    let vps = services::get_vps_by_id(&app_state.db_pool, vps_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;
    if vps.user_id != user_id {
        return Err(AppError::Unauthorized("Permission denied".to_string()));
    }

    let delete_result = services::remove_tag_from_vps(&app_state.db_pool, vps_id, tag_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if delete_result.rows_affected > 0 {
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
) -> Result<Json<Vec<tag::Model>>, AppError> {
    // Changed Tag to tag::Model
    let user_id = authenticated_user.id;
    // Authorize: Check if user owns the VPS
    let vps = services::get_vps_by_id(&app_state.db_pool, vps_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;
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
        .route(
            "/",
            post(add_tag_to_vps_handler).get(get_tags_for_vps_handler),
        )
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
        Err(db_err) => {
            // Changed to handle DbErr
            if let DbErr::RecordNotFound(_) = &db_err {
                // Directly match against &db_err
                // This specific mapping might need adjustment based on how `bulk_update_vps_tags` signals auth failure.
                // For now, assuming RecordNotFound might imply an issue with one of the VPS IDs not being found under user's ownership.
                Err(AppError::Unauthorized(
                    "Permission denied to one or more VPS, or VPS not found.".to_string(),
                ))
            } else {
                // db_err is still available here as it was only borrowed
                Err(AppError::DatabaseError(db_err.to_string()))
            }
        }
    }
}

async fn bulk_trigger_update_check_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<BulkTriggerUpdateCheckRequest>,
) -> Result<Json<BulkActionResponse>, AppError> {
    let user_id = authenticated_user.id;

    if payload.vps_ids.is_empty() {
        return Ok(Json(BulkActionResponse {
            message: "No VPS IDs provided.".to_string(),
            successful_count: 0,
            failed_count: 0,
        }));
    }

    // Verify user owns all VPS IDs and get the valid models
    let owned_vps_list =
        services::get_owned_vps_from_ids(&app_state.db_pool, user_id, &payload.vps_ids).await?;

    if owned_vps_list.len() != payload.vps_ids.len() {
        // This indicates a partial ownership, which we treat as a potential issue.
        // For simplicity, we'll proceed with the ones they do own, but a stricter policy might be to error out.
        // The service function already filters to only those owned.
        error!(
            "User {} attempted bulk update on VPS IDs they do not fully own. Requested: {:?}, Owned: {:?}",
            user_id,
            payload.vps_ids,
            owned_vps_list.iter().map(|v| v.id).collect::<Vec<_>>()
        );
    }

    let agents_guard = app_state.connected_agents.lock().await;
    let mut successful_sends = 0;
    let mut failed_sends = 0;

    for vps in owned_vps_list {
        if agents_guard.send_update_check_command(vps.id).await {
            successful_sends += 1;
        } else {
            failed_sends += 1;
        }
    }

    let total_requested = payload.vps_ids.len() as u32;
    let not_owned_or_failed = total_requested - successful_sends;

    Ok(Json(BulkActionResponse {
        message: format!(
            "Update commands sent. Success: {successful_sends}, Failed/Not Found: {not_owned_or_failed}."
        ),
        successful_count: successful_sends,
        failed_count: not_owned_or_failed,
    }))
}

// --- Renewal Reminder Handler ---

async fn dismiss_renewal_reminder_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    let user_id = authenticated_user.id;

    // Authorize: Check if user owns the VPS
    let vps = services::get_vps_by_id(&app_state.db_pool, vps_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;

    if vps.user_id != user_id {
        return Err(AppError::Unauthorized(
            "Permission denied to VPS".to_string(),
        ));
    }

    // Attempt to dismiss the reminder
    match services::dismiss_vps_renewal_reminder(&app_state.db_pool, vps_id).await {
        Ok(rows_affected) => {
            if rows_affected > 0 {
                // Successfully dismissed, trigger a full state update to ensure consistency
                update_service::broadcast_full_state_update(
                    &app_state.db_pool,
                    &app_state.live_server_data_cache,
                    &app_state.ws_data_broadcaster_tx,
                )
                .await;
                Ok(StatusCode::OK)
            } else {
                // No reminder was active, or VPS not found in renewal info (though ownership check should catch this)
                Ok(StatusCode::NOT_MODIFIED) // Or NOT_FOUND if we want to be more specific
            }
        }
        Err(e) => Err(AppError::DatabaseError(e.to_string())),
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MonitorTimeseriesQuery {
    pub start_time: DateTime<Utc>,
    #[serde(default = "default_end_time")]
    pub end_time: DateTime<Utc>,
    pub interval: Option<String>,
}

fn default_end_time() -> DateTime<Utc> {
    Utc::now()
}

pub fn parse_interval_to_seconds(interval: Option<String>) -> Option<i64> {
    interval.and_then(|s| {
        let s = s.trim();
        if let Some(seconds_str) = s.strip_suffix('s') {
            seconds_str.parse::<i64>().ok()
        } else if let Some(minutes_str) = s.strip_suffix('m') {
            minutes_str.parse::<i64>().ok().map(|m| m * 60)
        } else if let Some(hours_str) = s.strip_suffix('h') {
            hours_str.parse::<i64>().ok().map(|h| h * 3600)
        } else if let Some(days_str) = s.strip_suffix('d') {
            days_str.parse::<i64>().ok().map(|d| d * 86400)
        } else {
            s.parse::<i64>().ok()
        }
    })
}

async fn get_vps_monitor_results_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
    Query(query): Query<MonitorTimeseriesQuery>,
) -> Result<Json<Vec<ServiceMonitorResultDetails>>, AppError> {
    let user_id = authenticated_user.id;

    // Authorization: Verify the user owns the VPS.
    let vps = services::get_vps_by_id(&app_state.db_pool, vps_id)
        .await?
        .ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;
    if vps.user_id != user_id {
        return Err(AppError::Unauthorized("Access denied".to_string()));
    }

    let interval_seconds = parse_interval_to_seconds(query.interval);

    let points_result = services::get_monitor_results_by_vps_id(
        &app_state.db_pool,
        vps_id,
        query.start_time,
        query.end_time,
        interval_seconds,
    )
    .await;

    let points = match points_result {
        Ok(points) => points,
        Err(e) => {
            error!("Error fetching monitor results for VPS {}: {:?}", vps_id, e);
            return Err(e.into());
        }
    };

    if points.is_empty() {
        return Ok(Json(Vec::new()));
    }

    // --- Conversion from ServiceMonitorPoint to ServiceMonitorResultDetails ---
    // 1. Get all unique monitor IDs from the results
    let monitor_ids: Vec<i32> = points
        .iter()
        .map(|p| p.monitor_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    // 2. Fetch all monitor models for these IDs
    let monitors = service_monitor::Entity::find()
        .filter(service_monitor::Column::Id.is_in(monitor_ids))
        .all(&app_state.db_pool)
        .await?;
    let monitor_name_map: HashMap<i32, String> =
        monitors.into_iter().map(|m| (m.id, m.name)).collect();

    // 3. The agent name is just the VPS name, which we already have.
    let agent_name = vps.name;

    // 4. Map the points to the final details struct
    let results = points
        .into_iter()
        .map(|point| {
            let monitor_name = monitor_name_map
                .get(&point.monitor_id)
                .cloned()
                .unwrap_or_else(|| "Unknown Monitor".to_string());

            ServiceMonitorResultDetails {
                time: point.time.to_rfc3339(),
                monitor_id: point.monitor_id,
                agent_id: point.agent_id,
                agent_name: agent_name.clone(),
                monitor_name,
                // For aggregated data, is_up is a float (availability). Convert back to bool.
                // We consider > 50% availability as "up". For raw data, it will be 1.0 or 0.0.
                is_up: point.is_up.map_or(false, |v| v > 0.5),
                latency_ms: point.latency_ms.map(|f| f as i32),
                details: point.details,
            }
        })
        .collect();

    Ok(Json(results))
}

async fn get_vps_monitors_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
) -> Result<Json<Vec<service_monitor::Model>>, AppError> {
    let user_id = authenticated_user.id;

    // Authorize: Check if user owns the VPS
    let vps = services::get_vps_by_id(&app_state.db_pool, vps_id)
        .await?
        .ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;

    if vps.user_id != user_id {
        return Err(AppError::Unauthorized("Access denied".to_string()));
    }

    // Fetch all monitors associated with the VPS (directly or via tags)
    let monitors = services::get_monitors_for_vps(&app_state.db_pool, vps_id).await?;

    Ok(Json(monitors))
}

pub fn vps_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_vps_handler))
        .route("/", get(get_all_vps_handler))
        .route(
            "/bulk-actions/update-tags",
            post(bulk_update_vps_tags_handler),
        )
        .route(
            "/bulk-actions/trigger-update-check",
            post(bulk_trigger_update_check_handler),
        )
        .route("/{vps_id}", get(get_vps_detail_handler))
        .route("/{vps_id}", put(update_vps_handler))
        .route("/{vps_id}", delete(delete_vps_handler))
        .route(
            "/{vps_id}/renewal/dismiss-reminder",
            post(dismiss_renewal_reminder_handler),
        ) // New route
        .route(
            "/{vps_id}/monitors",
            get(get_vps_monitors_handler),
        ) // New route
        .route(
            "/{vps_id}/monitor-results",
            get(get_vps_monitor_results_handler),
        ) // New route
        .route(
            "/{vps_id}/trigger-update-check",
            post(trigger_update_check_handler),
        ) // New route for agent update
        .nest("/{vps_id}/tags", vps_tags_router()) // Nest the tags router
        .merge(config_routes::create_vps_config_router())
        .merge(metrics_routes::metrics_router())
}

async fn trigger_update_check_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    let user_id = authenticated_user.id;

    // Authorize: Check if user owns the VPS
    let vps = services::get_vps_by_id(&app_state.db_pool, vps_id)
        .await?
        .ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;
    if vps.user_id != user_id {
        return Err(AppError::Unauthorized("Access denied".to_string()));
    }

    // Send the command to the agent
    let agents_guard = app_state.connected_agents.lock().await;
    let sent = agents_guard.send_update_check_command(vps_id).await;

    if sent {
        Ok(StatusCode::ACCEPTED) // Accepted for processing
    } else {
        Err(AppError::NotFound(
            "Agent not connected or command could not be sent".to_string(),
        ))
    }
}

async fn delete_vps_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    let user_id = authenticated_user.id;

    // Authorize: Check if user owns the VPS
    let vps = services::get_vps_by_id(&app_state.db_pool, vps_id)
        .await?
        .ok_or_else(|| AppError::NotFound("VPS not found".to_string()))?;
    if vps.user_id != user_id {
        return Err(AppError::Unauthorized("Access denied".to_string()));
    }

    // Proceed with deletion
    services::delete_vps(&app_state.db_pool, vps_id).await?;

    // Broadcast the change
    update_service::broadcast_full_state_update(
        &app_state.db_pool,
        &app_state.live_server_data_cache,
        &app_state.ws_data_broadcaster_tx,
    )
    .await;

    Ok(StatusCode::NO_CONTENT)
}
