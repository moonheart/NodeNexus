use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::sync::Arc;

use crate::db::duckdb_service::performance_service::{self};
use crate::web::AppError;
use crate::web::AppState;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetricsTimeseriesQuery {
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub interval: Option<String>, // e.g., "1m", "5m", "1h", or "72s"
}

async fn get_vps_metrics_timeseries_handler(
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
    Query(params): Query<MetricsTimeseriesQuery>,
) -> Result<Json<Vec<performance_service::PerformanceMetricPoint>>, AppError> {
    let end_time = params.end_time.unwrap_or_else(Utc::now);

    if params.start_time >= end_time {
        return Err(AppError::InvalidInput(
            "start_time must be before end_time".to_string(),
        ));
    }

    // Updated interval parsing to support seconds ('s'), minutes ('m'), and hours ('h')
    let interval_seconds: Option<u32> = params.interval.as_ref().and_then(|s| {
        if s.ends_with('s') {
            s.trim_end_matches('s').parse().ok()
        } else if s.ends_with('m') {
            s.trim_end_matches('m').parse::<u32>().ok().map(|m| m * 60)
        } else if s.ends_with('h') {
            s.trim_end_matches('h')
                .parse::<u32>()
                .ok()
                .map(|h| h * 3600)
        } else {
            None
        }
    });

    let results = performance_service::get_performance_metrics_for_vps(
        &app_state.duckdb_pool,
        vps_id,
        params.start_time,
        end_time,
        interval_seconds, // Pass the parsed interval in seconds
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(Json(results))
}

pub fn metrics_router() -> Router<Arc<AppState>> {
    Router::new().route(
        "/{vps_id}/metrics/timeseries",
        get(get_vps_metrics_timeseries_handler),
    )
}

