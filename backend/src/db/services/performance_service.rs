use chrono::{TimeZone, Utc};
use sqlx::postgres::types::PgInterval;
use sqlx::{FromRow, PgPool, Result};

use crate::agent_service::PerformanceSnapshotBatch;
use crate::db::models::{AggregatedPerformanceMetric, PerformanceMetric};

// --- PerformanceMetric Service Functions ---

/// Retrieves performance metrics for a given VPS within a time range.
pub async fn get_performance_metrics_for_vps(
    pool: &PgPool,
    vps_id: i32,
    start_time: chrono::DateTime<chrono::Utc>,
    end_time: chrono::DateTime<chrono::Utc>,
    interval_seconds: Option<u32>,
) -> Result<Vec<AggregatedPerformanceMetric>> {
    if let Some(seconds) = interval_seconds {
        let interval_value = PgInterval {
            months: 0,
            days: 0,
            microseconds: (seconds.max(1) as i64) * 1_000_000,
        };

        sqlx::query_as!(
            AggregatedPerformanceMetric,
            r#"
            WITH TimeBucketed AS (
                SELECT
                    time_bucket($4::interval, time) AS bucket_time,
                    vps_id,
                    AVG(cpu_usage_percent) AS avg_cpu_usage_percent,
                    AVG(memory_usage_bytes) AS avg_memory_usage_bytes,
                    MAX(memory_total_bytes) AS max_memory_total_bytes,
                    AVG(network_rx_instant_bps) AS avg_network_rx_instant_bps,
                    AVG(network_tx_instant_bps) AS avg_network_tx_instant_bps
                FROM performance_metrics
                WHERE vps_id = $1 AND time >= $2 AND time <= $3
                GROUP BY bucket_time, vps_id
            )
            SELECT
                bucket_time AS time,
                vps_id,
                avg_cpu_usage_percent,
                avg_memory_usage_bytes::FLOAT8,
                max_memory_total_bytes,
                avg_network_rx_instant_bps::FLOAT8,
                avg_network_tx_instant_bps::FLOAT8
            FROM TimeBucketed
            ORDER BY time ASC
            "#,
            vps_id,
            start_time,
            end_time,
            interval_value
        )
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as!(
             AggregatedPerformanceMetric,
            r#"
            WITH RankedMetrics AS (
                SELECT
                    id, time, vps_id, cpu_usage_percent, memory_usage_bytes, memory_total_bytes,
                    swap_usage_bytes, swap_total_bytes,
                    disk_io_read_bps, disk_io_write_bps,
                    network_rx_bps, network_tx_bps,
                    network_rx_instant_bps, network_tx_instant_bps,
                    uptime_seconds, total_processes_count, running_processes_count,
                    tcp_established_connection_count
                FROM performance_metrics
                WHERE vps_id = $1 AND time >= $2 AND time <= $3
            )
            SELECT
                time,
                vps_id,
                cpu_usage_percent AS avg_cpu_usage_percent,
                memory_usage_bytes::FLOAT8 AS avg_memory_usage_bytes,
                memory_total_bytes AS max_memory_total_bytes,
                network_rx_instant_bps::FLOAT8 AS avg_network_rx_instant_bps,
                network_tx_instant_bps::FLOAT8 AS avg_network_tx_instant_bps
            FROM RankedMetrics
            ORDER BY time ASC
            "#,
            vps_id,
            start_time,
            end_time
        )
        .fetch_all(pool)
        .await
    }
}

/// Retrieves the latest performance metric for a given VPS.
pub async fn get_latest_performance_metric_for_vps(
    pool: &PgPool,
    vps_id: i32,
) -> Result<Option<PerformanceMetric>> {
    sqlx::query_as!(
        PerformanceMetric,
        r#"
        SELECT
            id, time, vps_id, cpu_usage_percent, memory_usage_bytes, memory_total_bytes,
            swap_usage_bytes, swap_total_bytes,
            disk_io_read_bps, disk_io_write_bps,
            network_rx_bps, network_tx_bps,
            network_rx_instant_bps, network_tx_instant_bps,
            uptime_seconds, total_processes_count, running_processes_count,
            tcp_established_connection_count
        FROM performance_metrics
        WHERE vps_id = $1
        ORDER BY time DESC
        LIMIT 1
        "#,
        vps_id
    )
    .fetch_optional(pool)
    .await
}

/// Retrieves the latest N performance metrics for a given VPS.
/// The results are sorted by time in ascending order.
pub async fn get_latest_n_performance_metrics_for_vps(
    pool: &PgPool,
    vps_id: i32,
    count: u32,
) -> Result<Vec<PerformanceMetric>> {
    let metrics = sqlx::query_as!(
        PerformanceMetric,
        r#"
        SELECT
            id, time, vps_id, cpu_usage_percent, memory_usage_bytes, memory_total_bytes,
            swap_usage_bytes, swap_total_bytes,
            disk_io_read_bps, disk_io_write_bps,
            network_rx_bps, network_tx_bps,
            network_rx_instant_bps, network_tx_instant_bps,
            uptime_seconds, total_processes_count, running_processes_count,
            tcp_established_connection_count
        FROM (
            SELECT * FROM performance_metrics
            WHERE vps_id = $1
            ORDER BY time DESC
            LIMIT $2
        ) AS latest_metrics
        ORDER BY time ASC
        "#,
        vps_id,
        count as i64 // LIMIT requires i64
    )
    .fetch_all(pool)
    .await?;
    Ok(metrics)
}

/// Saves a batch of performance snapshots for a given VPS.
/// This includes the main metrics, detailed disk usage, and detailed network interface stats.
pub async fn save_performance_snapshot_batch(
    pool: &PgPool,
    vps_id: i32,
    batch: &PerformanceSnapshotBatch,
) -> Result<(), sqlx::Error> {
    let mut tx = pool.begin().await?;

    for snapshot in &batch.snapshots {
        let timestamp = Utc
            .timestamp_millis_opt(snapshot.timestamp_unix_ms)
            .single()
            .unwrap_or_else(Utc::now);

        let metric_id = sqlx::query!(
            r#"
            INSERT INTO performance_metrics (
                time, vps_id, cpu_usage_percent, memory_usage_bytes, memory_total_bytes,
                swap_usage_bytes, swap_total_bytes,
                disk_io_read_bps, disk_io_write_bps,
                network_rx_bps, network_tx_bps,
                network_rx_instant_bps, network_tx_instant_bps,
                uptime_seconds, total_processes_count, running_processes_count,
                tcp_established_connection_count
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            RETURNING id
            "#,
            timestamp,
            vps_id,
            snapshot.cpu_overall_usage_percent as f64,
            snapshot.memory_usage_bytes as i64,
            snapshot.memory_total_bytes as i64,
            snapshot.swap_usage_bytes as i64,
            snapshot.swap_total_bytes as i64,
            snapshot.disk_total_io_read_bytes_per_sec as i64,
            snapshot.disk_total_io_write_bytes_per_sec as i64,
            snapshot.network_rx_bytes_cumulative as i64,
            snapshot.network_tx_bytes_cumulative as i64,
            snapshot.network_rx_bytes_per_sec as i64,
            snapshot.network_tx_bytes_per_sec as i64,
            snapshot.uptime_seconds as i64,
            snapshot.total_processes_count as i32,
            snapshot.running_processes_count as i32,
            snapshot.tcp_established_connection_count as i32
        )
        .fetch_one(&mut *tx)
        .await?
        .id;

        for disk_usage in &snapshot.disk_usages {
            sqlx::query!(
                r#"
                INSERT INTO performance_disk_usages (
                    performance_metric_id, mount_point, used_bytes, total_bytes, fstype, usage_percent
                )
                VALUES ($1, $2, $3, $4, $5, $6)
                "#,
                metric_id,
                disk_usage.mount_point,
                disk_usage.used_bytes as i64,
                disk_usage.total_bytes as i64,
                disk_usage.fstype,
                disk_usage.usage_percent as f64
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

/// Retrieves the summary of total and used disk space from the latest performance metric.
pub async fn get_latest_disk_usage_summary(
    pool: &PgPool,
    vps_id: i32,
) -> Result<Option<(i64, i64)>> {
    // Returns (total_bytes, used_bytes)
    let result = sqlx::query_as!(
        DiskUsageSummary,
        r#"
        WITH LatestMetric AS (
            SELECT id
            FROM performance_metrics
            WHERE vps_id = $1
            ORDER BY time DESC
            LIMIT 1
        )
        SELECT
            SUM(pdu.total_bytes)::BIGINT as total_sum_bytes,
            SUM(pdu.used_bytes)::BIGINT as used_sum_bytes
        FROM performance_disk_usages pdu
        JOIN LatestMetric lm ON pdu.performance_metric_id = lm.id
        WHERE EXISTS (SELECT 1 FROM LatestMetric)
        GROUP BY lm.id
        "#,
        vps_id
    )
    .fetch_optional(pool)
    .await?;

    match result {
        Some(summary) => {
            let total = summary.total_sum_bytes.unwrap_or(0);
            let used = summary.used_sum_bytes.unwrap_or(0);
            if total == 0 && used == 0 && summary.total_sum_bytes.is_none() {
                Ok(None)
            } else {
                Ok(Some((total, used)))
            }
        }
        None => Ok(None),
    }
}

// Helper struct for the above query
struct DiskUsageSummary {
    total_sum_bytes: Option<i64>,
    used_sum_bytes: Option<i64>,
}