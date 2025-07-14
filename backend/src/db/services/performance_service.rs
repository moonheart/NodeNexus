use chrono::{DateTime, TimeZone, Utc};
use sea_orm::{
    ActiveModelTrait,
    ColumnTrait,
    DatabaseConnection,
    DbErr,
    EntityTrait,
    FromQueryResult,
    Order,
    QueryFilter,
    QueryOrder,
    QuerySelect,
    Set, // Removed IntoActiveModel, ModelTrait, PaginatorTrait, Value
    TransactionTrait,
    sea_query::{Alias, Expr, Func},
};
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::agent_service::PerformanceSnapshotBatch;
use crate::db::entities::{performance_disk_usage, performance_metric};
use crate::db::services::vps_traffic_service; // Corrected direct import

// --- PerformanceMetric Service Functions ---

/// Represents a unified performance metric point for API responses.
/// It can hold either raw or aggregated data, but presents it with consistent field names.
#[derive(FromQueryResult, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceMetricPoint {
    pub time: DateTime<Utc>,
    pub vps_id: i32,
    pub cpu_usage_percent: Option<f64>,
    pub memory_usage_bytes: Option<f64>, // f64 for potential AVG
    pub memory_total_bytes: Option<i64>, // i64 for MAX or raw value
    pub network_rx_instant_bps: Option<f64>, // f64 for potential AVG
    pub network_tx_instant_bps: Option<f64>, // f64 for potential AVG
    pub disk_io_read_bps: Option<f64>,       // f64 for potential AVG
    pub disk_io_write_bps: Option<f64>,      // f64 for potential AVG
}

/// Retrieves performance metrics for a given VPS within a time range.
/// If `interval_seconds` is provided, data is aggregated into time buckets.
/// Otherwise, raw data points are returned.
/// The returned `PerformanceMetricPoint` has a consistent structure for both cases.
pub async fn get_performance_metrics_for_vps(
    db: &DatabaseConnection,
    vps_id: i32,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    interval_seconds: Option<u32>,
) -> Result<Vec<PerformanceMetricPoint>, DbErr> {
    if let Some(seconds) = interval_seconds {
        // AGGREGATED QUERY
        let time_bucket_expr = Expr::cust(format!(
            "time_bucket(INTERVAL '{} seconds', \"time\")",
            seconds.max(1)
        ));

        performance_metric::Entity::find()
            .select_only()
            .column_as(time_bucket_expr.clone(), "time")
            .column(performance_metric::Column::VpsId)
            .column_as(
                Expr::expr(Func::avg(Expr::col(
                    performance_metric::Column::CpuUsagePercent,
                ))),
                "cpu_usage_percent", // Map AVG to the base field name
            )
            .column_as(
                Expr::expr(Func::avg(Expr::col(
                    performance_metric::Column::MemoryUsageBytes,
                ))).cast_as(Alias::new("double precision")),
                "memory_usage_bytes", // Map AVG to the base field name
            )
            .column_as(
                Expr::expr(Func::max(Expr::col(
                    performance_metric::Column::MemoryTotalBytes,
                ))),
                "memory_total_bytes", // Map MAX to the base field name
            )
            .column_as(
                Expr::expr(Func::avg(Expr::col(
                    performance_metric::Column::NetworkRxInstantBps,
                ))).cast_as(Alias::new("double precision")),
                "network_rx_instant_bps", // Map AVG to the base field name
            )
            .column_as(
                Expr::expr(Func::avg(Expr::col(
                    performance_metric::Column::NetworkTxInstantBps,
                ))).cast_as(Alias::new("double precision")),
                "network_tx_instant_bps", // Map AVG to the base field name
            )
            .column_as(
                Expr::expr(Func::avg(Expr::col(
                    performance_metric::Column::DiskIoReadBps,
                ))).cast_as(Alias::new("double precision")),
                "disk_io_read_bps", // Map AVG to the base field name
            )
            .column_as(
                Expr::expr(Func::avg(Expr::col(
                    performance_metric::Column::DiskIoWriteBps,
                ))).cast_as(Alias::new("double precision")),
                "disk_io_write_bps", // Map AVG to the base field name
            )
            .filter(performance_metric::Column::VpsId.eq(vps_id))
            .filter(performance_metric::Column::Time.gte(start_time))
            .filter(performance_metric::Column::Time.lte(end_time))
            .group_by(time_bucket_expr)
            .group_by(performance_metric::Column::VpsId)
            .order_by(Expr::col(Alias::new("time")), Order::Asc)
            .into_model::<PerformanceMetricPoint>()
            .all(db)
            .await
    } else {
        // RAW DATA QUERY
        performance_metric::Entity::find()
            .select_only()
            .column(performance_metric::Column::Time)
            .column(performance_metric::Column::VpsId)
            .column_as(
                performance_metric::Column::CpuUsagePercent,
                "cpu_usage_percent",
            )
            .column_as(
                Expr::col(performance_metric::Column::MemoryUsageBytes)
                    .cast_as(Alias::new("float")),
                "memory_usage_bytes",
            )
            .column_as(
                performance_metric::Column::MemoryTotalBytes,
                "memory_total_bytes",
            )
            .column_as(
                Expr::col(performance_metric::Column::NetworkRxInstantBps)
                    .cast_as(Alias::new("float")),
                "network_rx_instant_bps",
            )
            .column_as(
                Expr::col(performance_metric::Column::NetworkTxInstantBps)
                    .cast_as(Alias::new("float")),
                "network_tx_instant_bps",
            )
            .column_as(
                Expr::col(performance_metric::Column::DiskIoReadBps).cast_as(Alias::new("float")),
                "disk_io_read_bps",
            )
            .column_as(
                Expr::col(performance_metric::Column::DiskIoWriteBps).cast_as(Alias::new("float")),
                "disk_io_write_bps",
            )
            .filter(performance_metric::Column::VpsId.eq(vps_id))
            .filter(performance_metric::Column::Time.gte(start_time))
            .filter(performance_metric::Column::Time.lte(end_time))
            .order_by(performance_metric::Column::Time, Order::Asc)
            .into_model::<PerformanceMetricPoint>()
            .all(db)
            .await
    }
}

/// Retrieves the latest performance metric for a given VPS.
pub async fn get_latest_performance_metric_for_vps(
    db: &DatabaseConnection,
    vps_id: i32,
) -> Result<Option<performance_metric::Model>, DbErr> {
    performance_metric::Entity::find()
        .filter(performance_metric::Column::VpsId.eq(vps_id))
        .order_by_desc(performance_metric::Column::Time)
        .one(db)
        .await
}

/// Saves a batch of performance snapshots for a given VPS.
/// This includes the main metrics, detailed disk usage, and detailed network interface stats.
pub async fn save_performance_snapshot_batch(
    db: &DatabaseConnection,
    vps_id: i32,
    batch: &PerformanceSnapshotBatch,
) -> Result<Vec<(performance_metric::Model, Vec<performance_disk_usage::Model>)>, DbErr> {
    if batch.snapshots.is_empty() {
        return Ok(Vec::new());
    }

    let txn = db.begin().await?;
    let mut saved_metrics = Vec::with_capacity(batch.snapshots.len());

    for snapshot in &batch.snapshots {
        let timestamp = Utc
            .timestamp_millis_opt(snapshot.timestamp_unix_ms)
            .single()
            .unwrap_or_else(Utc::now);

        let metric_active_model = performance_metric::ActiveModel {
            time: Set(timestamp),
            vps_id: Set(vps_id),
            cpu_usage_percent: Set(snapshot.cpu_overall_usage_percent as f64),
            memory_usage_bytes: Set(snapshot.memory_usage_bytes as i64),
            memory_total_bytes: Set(snapshot.memory_total_bytes as i64),
            swap_usage_bytes: Set(snapshot.swap_usage_bytes as i64),
            swap_total_bytes: Set(snapshot.swap_total_bytes as i64),
            disk_io_read_bps: Set(snapshot.disk_total_io_read_bytes_per_sec as i64),
            disk_io_write_bps: Set(snapshot.disk_total_io_write_bytes_per_sec as i64),
            network_rx_cumulative: Set(snapshot.network_rx_bytes_cumulative as i64),
            network_tx_cumulative: Set(snapshot.network_tx_bytes_cumulative as i64),
            network_rx_instant_bps: Set(snapshot.network_rx_bytes_per_sec as i64),
            network_tx_instant_bps: Set(snapshot.network_tx_bytes_per_sec as i64),
            uptime_seconds: Set(snapshot.uptime_seconds as i64),
            total_processes_count: Set(snapshot.total_processes_count as i32),
            running_processes_count: Set(snapshot.running_processes_count as i32),
            tcp_established_connection_count: Set(snapshot.tcp_established_connection_count as i32),
            ..Default::default() // For id
        };
        let metric_model = metric_active_model.insert(&txn).await?;

        let mut saved_disk_usages = Vec::with_capacity(snapshot.disk_usages.len());
        for disk_usage in &snapshot.disk_usages {
            let disk_usage_active_model = performance_disk_usage::ActiveModel {
                performance_metric_id: Set(metric_model.id),
                mount_point: Set(disk_usage.mount_point.clone()),
                used_bytes: Set(disk_usage.used_bytes as i64),
                total_bytes: Set(disk_usage.total_bytes as i64),
                fstype: Set(Some(disk_usage.fstype.clone())),
                usage_percent: Set(disk_usage.usage_percent),
                ..Default::default() // For id
            };
            let disk_usage_model = disk_usage_active_model.insert(&txn).await?;
            saved_disk_usages.push(disk_usage_model);
        }
        saved_metrics.push((metric_model, saved_disk_usages));
    }

    // After saving all metrics in the batch, update the traffic stats ONCE
    // using the LATEST cumulative values from the last snapshot in the batch.
    if let Some(last_snapshot) = batch.snapshots.last() {
        if let Err(e) = vps_traffic_service::update_vps_traffic_stats_after_metric(
            &txn,
            vps_id,
            last_snapshot.network_rx_bytes_cumulative as i64,
            last_snapshot.network_tx_bytes_cumulative as i64,
        )
        .await
        {
            error!(
                vps_id = vps_id,
                error = %e,
                "Failed to update VPS traffic stats. Rolling back metric save."
            );
            txn.rollback().await?;
            return Err(e);
        }
    }

    txn.commit().await?;
    Ok(saved_metrics)
}

/// Helper struct for the disk usage summary query
#[derive(FromQueryResult, Debug)]
struct DiskUsageSummary {
    total_sum_bytes: Option<i64>,
    used_sum_bytes: Option<i64>,
}

/// Retrieves the summary of total and used disk space from the latest performance metric.
pub async fn get_latest_disk_usage_summary(
    db: &DatabaseConnection,
    vps_id: i32,
) -> Result<Option<(i64, i64)>, DbErr> {
    let latest_metric_opt = performance_metric::Entity::find()
        .filter(performance_metric::Column::VpsId.eq(vps_id))
        .order_by_desc(performance_metric::Column::Time)
        .one(db)
        .await?;

    if let Some(latest_metric) = latest_metric_opt {
        let summary_opt: Option<DiskUsageSummary> = performance_disk_usage::Entity::find()
            .select_only()
            .column_as(
                Expr::expr(Func::sum(Expr::col(
                    performance_disk_usage::Column::TotalBytes,
                ))),
                "total_sum_bytes",
            )
            .column_as(
                Expr::expr(Func::sum(Expr::col(
                    performance_disk_usage::Column::UsedBytes,
                ))),
                "used_sum_bytes",
            )
            .filter(performance_disk_usage::Column::PerformanceMetricId.eq(latest_metric.id))
            .into_model::<DiskUsageSummary>()
            .one(db)
            .await?;

        match summary_opt {
            Some(summary) => {
                // If SUM results in NULL (no rows or all values NULL), FromQueryResult sets Option to None.
                // If there are no disk usage records for the latest metric, both sums will be None.
                if summary.total_sum_bytes.is_none() && summary.used_sum_bytes.is_none() {
                    Ok(None)
                } else {
                    let total = summary.total_sum_bytes.unwrap_or(0);
                    let used = summary.used_sum_bytes.unwrap_or(0);
                    Ok(Some((total, used)))
                }
            }
            None => Ok(None), // Should not happen if latest_metric exists and query is structured correctly,
                              // but implies no disk usage records.
        }
    } else {
        Ok(None) // No performance metrics for this VPS
    }
}
