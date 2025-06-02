use axum::{
    extract::{State, Path, Query},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use chrono::{DateTime, Utc};

use crate::db::models::PerformanceMetric;
use crate::db::services as db_services;
use super::AppState;
use crate::http_server::AppError;



#[derive(Deserialize)]
pub struct MetricsTimeseriesQuery {
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    // TODO: Add interval for aggregation later if needed
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
) -> Result<Json<Vec<PerformanceMetric>>, AppError> {
    if params.start_time >= params.end_time {
        return Err(AppError::InvalidInput(
            "start_time must be before end_time".to_string()
        ));
    }
    let metrics = db_services::get_performance_metrics_for_vps(
        &app_state.db_pool,
        vps_id,
        params.start_time,
        params.end_time,
    )
    .await.map_err(|e| AppError::ServerError(e.to_string()))?;
    Ok(Json(metrics))
}

pub fn metrics_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/vps/{vps_id}/metrics/latest", get(get_latest_vps_metrics_handler))
        .route("/api/vps/{vps_id}/metrics/timeseries", get(get_vps_metrics_timeseries_handler))
}