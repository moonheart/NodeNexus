use axum::{
    extract::{State, Path, Query},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use chrono::{DateTime, Utc};

use crate::db::models::{PerformanceMetric, AggregatedPerformanceMetric}; // Added AggregatedPerformanceMetric
use crate::db::services as db_services;
use super::AppState;
use crate::http_server::AppError;



#[derive(Deserialize)]
pub struct MetricsTimeseriesQuery {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub interval: Option<String>, // e.g., "1m", "5m", "1h", or "72s"
}

#[derive(Deserialize)]
pub struct LatestNMetricsQuery {
    pub count: u32,
}

async fn get_latest_n_vps_metrics_handler(
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
    Query(params): Query<LatestNMetricsQuery>,
) -> Result<Json<Vec<PerformanceMetric>>, AppError> {
    let metrics = db_services::get_latest_n_performance_metrics_for_vps(&app_state.db_pool, vps_id, params.count).await
        .map_err(|e| AppError::ServerError(e.to_string()))?;
    Ok(Json(metrics))
}

async fn get_latest_vps_metrics_handler(
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
) -> Result<Json<Option<PerformanceMetric>>, AppError> {
    let metrics = db_services::get_latest_performance_metric_for_vps(&app_state.db_pool, vps_id).await
        .map_err(|e| AppError::ServerError(e.to_string()))?;
    Ok(Json(metrics))
}

async fn get_vps_metrics_timeseries_handler(
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
    Query(params): Query<MetricsTimeseriesQuery>,
) -> Result<Json<Vec<AggregatedPerformanceMetric>>, AppError> { // Changed return type
    if params.start_time >= params.end_time {
        return Err(AppError::InvalidInput(
            "start_time must be before end_time".to_string()
        ));
    }

    // Updated interval parsing to support seconds ('s'), minutes ('m'), and hours ('h')
    let interval_seconds: Option<u32> = params.interval.as_ref().and_then(|s| {
        if s.ends_with('s') {
            s.trim_end_matches('s').parse().ok()
        } else if s.ends_with('m') {
            s.trim_end_matches('m').parse::<u32>().ok().map(|m| m * 60)
        } else if s.ends_with('h') {
            s.trim_end_matches('h').parse::<u32>().ok().map(|h| h * 3600)
        } else {
            None
        }
    });

    let metrics = db_services::get_performance_metrics_for_vps(
        &app_state.db_pool,
        vps_id,
        params.start_time,
        params.end_time,
        interval_seconds, // Pass the parsed interval in seconds
    )
    .await.map_err(|e| AppError::ServerError(e.to_string()))?;
    Ok(Json(metrics))
}

pub fn metrics_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/vps/{vps_id}/metrics/latest", get(get_latest_vps_metrics_handler))
        .route("/api/vps/{vps_id}/metrics/timeseries", get(get_vps_metrics_timeseries_handler))
        .route("/api/vps/{vps_id}/metrics/latest-n", get(get_latest_n_vps_metrics_handler))
}