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

use crate::agent_service::PerformanceSnapshotBatch;
use crate::db::entities::{performance_disk_usage, performance_metric};
use crate::db::services::vps_traffic_service; // Corrected direct import

// --- PerformanceMetric Service Functions ---

/// Represents an aggregated performance metric, typically used for time-bucketed queries.
#[derive(FromQueryResult, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregatedPerformanceMetric {
    pub time: Option<DateTime<Utc>>,
    pub vps_id: i32,
    pub avg_cpu_usage_percent: Option<f64>,
    pub avg_memory_usage_bytes: Option<f64>,
    pub max_memory_total_bytes: Option<i64>,
    pub avg_network_rx_instant_bps: Option<f64>,
    pub avg_network_tx_instant_bps: Option<f64>,
    pub avg_disk_io_read_bps: Option<f64>, // Added
    pub avg_disk_io_write_bps: Option<f64>, // Added
}

/// Retrieves performance metrics for a given VPS within a time range.
pub async fn get_performance_metrics_for_vps(
    db: &DatabaseConnection,
    vps_id: i32,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    interval_seconds: Option<u32>,
) -> Result<Vec<AggregatedPerformanceMetric>, DbErr> {
    if let Some(seconds) = interval_seconds {
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
                "avg_cpu_usage_percent",
            )
            .column_as(
                Expr::expr(Func::avg(Expr::col(
                    performance_metric::Column::MemoryUsageBytes,
                ))),
                "avg_memory_usage_bytes",
            )
            .column_as(
                Expr::expr(Func::max(Expr::col(
                    performance_metric::Column::MemoryTotalBytes,
                ))),
                "max_memory_total_bytes",
            )
            .column_as(
                Expr::expr(Func::avg(Expr::col(
                    performance_metric::Column::NetworkRxInstantBps,
                ))),
                "avg_network_rx_instant_bps",
            )
            .column_as(
                Expr::expr(Func::avg(Expr::col(
                    performance_metric::Column::NetworkTxInstantBps,
                ))),
                "avg_network_tx_instant_bps",
            )
            .column_as( // Added for disk read BPS
                Expr::expr(Func::avg(Expr::col( // Changed from AVG to MAX
                    performance_metric::Column::DiskIoReadBps,
                ))),
                "avg_disk_io_read_bps",
            )
            .column_as( // Added for disk write BPS
                Expr::expr(Func::avg(Expr::col( // Changed from AVG to MAX
                    performance_metric::Column::DiskIoWriteBps,
                ))),
                "avg_disk_io_write_bps",
            )
            .filter(performance_metric::Column::VpsId.eq(vps_id))
            .filter(performance_metric::Column::Time.gte(start_time))
            .filter(performance_metric::Column::Time.lte(end_time))
            .group_by(time_bucket_expr)
            .group_by(performance_metric::Column::VpsId)
            .order_by(Expr::col(Alias::new("time")), Order::Asc)
            .into_model::<AggregatedPerformanceMetric>()
            .all(db)
            .await
    } else {
        performance_metric::Entity::find()
            .select_only()
            .column_as(performance_metric::Column::Time, "time")
            .column(performance_metric::Column::VpsId)
            .column_as(
                performance_metric::Column::CpuUsagePercent,
                "avg_cpu_usage_percent",
            )
            .column_as(
                Expr::col(performance_metric::Column::MemoryUsageBytes),
                "avg_memory_usage_bytes",
            )
            .column_as(
                performance_metric::Column::MemoryTotalBytes,
                "max_memory_total_bytes",
            )
            .column_as(
                Expr::col(performance_metric::Column::NetworkRxInstantBps),
                "avg_network_rx_instant_bps",
            )
            .column_as(
                Expr::col(performance_metric::Column::NetworkTxInstantBps),
                "avg_network_tx_instant_bps",
            )
            .column_as( // Added for disk read BPS
                Expr::col(performance_metric::Column::DiskIoReadBps),
                "avg_disk_io_read_bps",
            )
            .column_as( // Added for disk write BPS
                Expr::col(performance_metric::Column::DiskIoWriteBps),
                "avg_disk_io_write_bps",
            )
            .filter(performance_metric::Column::VpsId.eq(vps_id))
            .filter(performance_metric::Column::Time.gte(start_time))
            .filter(performance_metric::Column::Time.lte(end_time))
            .order_by(performance_metric::Column::Time, Order::Asc)
            .into_model::<AggregatedPerformanceMetric>()
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

/// Retrieves the latest N performance metrics for a given VPS.
/// The results are sorted by time in ascending order.
pub async fn get_latest_n_performance_metrics_for_vps(
    db: &DatabaseConnection,
    vps_id: i32,
    count: u32,
) -> Result<Vec<performance_metric::Model>, DbErr> {
    let mut metrics = performance_metric::Entity::find()
        .filter(performance_metric::Column::VpsId.eq(vps_id))
        .order_by_desc(performance_metric::Column::Time)
        .limit(count as u64)
        .all(db)
        .await?;
    metrics.reverse(); // To sort by time ASC
    Ok(metrics)
}

/// Saves a batch of performance snapshots for a given VPS.
/// This includes the main metrics, detailed disk usage, and detailed network interface stats.
pub async fn save_performance_snapshot_batch(
    db: &DatabaseConnection,
    vps_id: i32,
    batch: &PerformanceSnapshotBatch,
) -> Result<(), DbErr> {
    let txn = db.begin().await?;

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
            network_rx_bps: Set(snapshot.network_rx_bytes_cumulative as i64),
            network_tx_bps: Set(snapshot.network_tx_bytes_cumulative as i64),
            network_rx_instant_bps: Set(snapshot.network_rx_bytes_per_sec as i64),
            network_tx_instant_bps: Set(snapshot.network_tx_bytes_per_sec as i64),
            uptime_seconds: Set(snapshot.uptime_seconds as i64),
            total_processes_count: Set(snapshot.total_processes_count as i32),
            running_processes_count: Set(snapshot.running_processes_count as i32),
            tcp_established_connection_count: Set(snapshot.tcp_established_connection_count as i32),
            ..Default::default() // For id
        };
        let metric_model = metric_active_model.insert(&txn).await?;

        for disk_usage in &snapshot.disk_usages {
            let disk_usage_active_model = performance_disk_usage::ActiveModel {
                performance_metric_id: Set(metric_model.id),
                mount_point: Set(disk_usage.mount_point.clone()),
                used_bytes: Set(disk_usage.used_bytes as i64),
                total_bytes: Set(disk_usage.total_bytes as i64),
                fstype: Set(Some(disk_usage.fstype.clone())), // Corrected: Wrapped with Some()
                usage_percent: Set(disk_usage.usage_percent as f64),
                ..Default::default() // For id
            };
            disk_usage_active_model.insert(&txn).await?;
        }

        // After saving the metric and its related disk usages, update VPS traffic stats
        // Assuming vps_traffic_service is correctly imported and its function signature is updated
        if let Err(e) = vps_traffic_service::update_vps_traffic_stats_after_metric(
            &txn, // Pass the transaction
            vps_id,
            snapshot.network_rx_bytes_cumulative as i64,
            snapshot.network_tx_bytes_cumulative as i64,
        )
        .await
        {
            eprintln!(
                "Failed to update VPS traffic stats for vps_id {}: {}. Rolling back metric save.",
                vps_id, e
            );
            txn.rollback().await?;
            return Err(e);
        }
    }

    txn.commit().await?;
    Ok(())
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
