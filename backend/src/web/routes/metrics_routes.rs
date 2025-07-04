use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::sync::Arc;

use crate::db::entities::performance_metric; // SeaORM model
use crate::db::models::{
    // PerformanceMetric as DtoPerformanceMetric, // No longer used directly by latest-n
    AggregatedPerformanceMetric as DtoAggregatedPerformanceMetric,
    RawPerformanceMetricPointDto, // Added for latest-n endpoint
};
use crate::db::services as db_services;
use crate::web::AppError;
use crate::web::AppState;

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
) -> Result<Json<Vec<RawPerformanceMetricPointDto>>, AppError> {
    // Changed to RawPerformanceMetricPointDto
    let models: Vec<performance_metric::Model> =
        db_services::get_latest_n_performance_metrics_for_vps(
            &app_state.db_pool,
            vps_id,
            params.count,
        )
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let dtos: Vec<RawPerformanceMetricPointDto> = models
        .into_iter()
        .map(|model| RawPerformanceMetricPointDto {
            // id is not part of RawPerformanceMetricPointDto as it's not typically needed for chart points
            time: model.time,
            vps_id: model.vps_id, // This will be serialized as vpsId due to #[serde(rename_all = "camelCase")]
            cpu_usage_percent: model.cpu_usage_percent,
            memory_usage_bytes: model.memory_usage_bytes,
            memory_total_bytes: model.memory_total_bytes,
            swap_usage_bytes: model.swap_usage_bytes,
            swap_total_bytes: model.swap_total_bytes,
            disk_io_read_bps: model.disk_io_read_bps,
            disk_io_write_bps: model.disk_io_write_bps,
            // network_rx_bps and network_tx_bps (cumulative) are omitted
            network_rx_instant_bps: model.network_rx_instant_bps,
            network_tx_instant_bps: model.network_tx_instant_bps,
            uptime_seconds: model.uptime_seconds,
            total_processes_count: model.total_processes_count,
            running_processes_count: model.running_processes_count,
            tcp_established_connection_count: model.tcp_established_connection_count,
        })
        .collect();
    Ok(Json(dtos))
}

async fn get_latest_vps_metrics_handler(
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
) -> Result<Json<Option<RawPerformanceMetricPointDto>>, AppError> {
    // Changed to RawPerformanceMetricPointDto for consistency if needed, or keep DtoPerformanceMetric if it's fine
    let model_option: Option<performance_metric::Model> =
        db_services::get_latest_performance_metric_for_vps(&app_state.db_pool, vps_id)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    // Using RawPerformanceMetricPointDto for /latest as well for consistency in field names (camelCase)
    // if this endpoint is also consumed by frontend expecting camelCase for these raw points.
    let dto_option: Option<RawPerformanceMetricPointDto> =
        model_option.map(|model| RawPerformanceMetricPointDto {
            time: model.time,
            vps_id: model.vps_id,
            cpu_usage_percent: model.cpu_usage_percent,
            memory_usage_bytes: model.memory_usage_bytes,
            memory_total_bytes: model.memory_total_bytes,
            swap_usage_bytes: model.swap_usage_bytes,
            swap_total_bytes: model.swap_total_bytes,
            disk_io_read_bps: model.disk_io_read_bps,
            disk_io_write_bps: model.disk_io_write_bps,
            network_rx_instant_bps: model.network_rx_instant_bps,
            network_tx_instant_bps: model.network_tx_instant_bps,
            uptime_seconds: model.uptime_seconds,
            total_processes_count: model.total_processes_count,
            running_processes_count: model.running_processes_count,
            tcp_established_connection_count: model.tcp_established_connection_count,
        });
    Ok(Json(dto_option))
}

async fn get_vps_metrics_timeseries_handler(
    State(app_state): State<Arc<AppState>>,
    Path(vps_id): Path<i32>,
    Query(params): Query<MetricsTimeseriesQuery>,
) -> Result<Json<Vec<DtoAggregatedPerformanceMetric>>, AppError> {
    // Changed to DtoAggregatedPerformanceMetric
    if params.start_time >= params.end_time {
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

    // The service returns Vec<performance_service::AggregatedPerformanceMetric>
    // We need to map it to Vec<DtoAggregatedPerformanceMetric>
    let service_metrics: Vec<
        crate::db::services::performance_service::AggregatedPerformanceMetric,
    > = db_services::get_performance_metrics_for_vps(
        &app_state.db_pool,
        vps_id,
        params.start_time,
        params.end_time,
        interval_seconds, // Pass the parsed interval in seconds
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let dtos: Vec<DtoAggregatedPerformanceMetric> = service_metrics
        .into_iter()
        .map(|sm| {
            DtoAggregatedPerformanceMetric {
                time: sm.time,
                vps_id: sm.vps_id,
                avg_cpu_usage_percent: sm.avg_cpu_usage_percent,
                avg_memory_usage_bytes: sm.avg_memory_usage_bytes,
                max_memory_total_bytes: sm.max_memory_total_bytes,
                avg_network_rx_instant_bps: sm.avg_network_rx_instant_bps,
                avg_network_tx_instant_bps: sm.avg_network_tx_instant_bps,
                avg_disk_io_read_bps: sm.avg_disk_io_read_bps, // Added
                avg_disk_io_write_bps: sm.avg_disk_io_write_bps, // Added
                                                               // Ensure all fields from DtoAggregatedPerformanceMetric are covered
                                                               // and that they exist in performance_service::AggregatedPerformanceMetric
            }
        })
        .collect();
    Ok(Json(dtos))
}

pub fn metrics_router() -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/api/vps/{vps_id}/metrics/latest",
            get(get_latest_vps_metrics_handler),
        )
        .route(
            "/api/vps/{vps_id}/metrics/timeseries",
            get(get_vps_metrics_timeseries_handler),
        )
        .route(
            "/api/vps/{vps_id}/metrics/latest-n",
            get(get_latest_n_vps_metrics_handler),
        )
}
