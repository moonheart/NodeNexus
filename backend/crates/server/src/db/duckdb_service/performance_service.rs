use chrono::{DateTime, Duration, Utc};
use duckdb::params;
use serde::{Deserialize, Serialize};
use tracing::debug;

use super::Error;
use db::duckdb_service::DuckDbPool;
use nodenexus_common::agent_service::PerformanceSnapshotBatch;
use crate::db::{self, entities::performance_metric};

// --- Data Structures for API Response ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceMetricPoint {
    pub time: DateTime<Utc>,
    pub vps_id: i32,
    // Base metrics (from AVG)
    pub cpu_usage_percent: Option<f64>,
    pub memory_usage_bytes: Option<f64>,
    pub memory_total_bytes: Option<f64>,
    pub swap_usage_bytes: Option<f64>,
    pub disk_io_read_bps: Option<f64>,
    pub disk_io_write_bps: Option<f64>,
    pub network_rx_instant_bps: Option<f64>,
    pub network_tx_instant_bps: Option<f64>,
    // Aggregated disk usages
    pub used_disk_space_bytes: Option<f64>,
    pub total_disk_space_bytes: Option<f64>,
}

/// Retrieves performance metrics for a given VPS within a time range from DuckDB.
pub async fn get_performance_metrics_for_vps(
    pool: &DuckDbPool,
    vps_id: i32,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    interval_seconds: Option<u32>,
) -> Result<Vec<PerformanceMetricPoint>, Error> {
    let conn = pool.get()?;

    // If no interval is specified, return raw data points.
    if interval_seconds.is_none() {
        debug!("No interval specified, fetching raw performance_metrics from DuckDB.");
        let mut stmt = conn.prepare(
            "SELECT * FROM performance_metrics WHERE vps_id = ? AND time >= ? AND time <= ? ORDER BY time ASC"
        )?;

        let results = stmt.query_map(params![vps_id, start_time, end_time], |row| {
            let m: performance_metric::Model = performance_metric::Model {
                time: row.get(0)?,
                vps_id: row.get(1)?,
                cpu_usage_percent: row.get(2)?,
                memory_usage_bytes: row.get(3)?,
                memory_total_bytes: row.get(4)?,
                swap_usage_bytes: row.get(5)?,
                swap_total_bytes: row.get(6)?,
                disk_io_read_bps: row.get(7)?,
                disk_io_write_bps: row.get(8)?,
                network_rx_cumulative: row.get(9)?,
                network_tx_cumulative: row.get(10)?,
                network_rx_instant_bps: row.get(11)?,
                network_tx_instant_bps: row.get(12)?,
                uptime_seconds: row.get(13)?,
                total_processes_count: row.get(14)?,
                running_processes_count: row.get(15)?,
                tcp_established_connection_count: row.get(16)?,
                total_disk_space_bytes: row.get(17)?,
                used_disk_space_bytes: row.get(18)?,
            };
            Ok(PerformanceMetricPoint {
                time: m.time,
                vps_id: m.vps_id,
                cpu_usage_percent: Some(m.cpu_usage_percent),
                memory_usage_bytes: Some(m.memory_usage_bytes as f64),
                memory_total_bytes: Some(m.memory_total_bytes as f64),
                swap_usage_bytes: Some(m.swap_usage_bytes as f64),
                disk_io_read_bps: Some(m.disk_io_read_bps as f64),
                disk_io_write_bps: Some(m.disk_io_write_bps as f64),
                network_rx_instant_bps: Some(m.network_rx_instant_bps as f64),
                network_tx_instant_bps: Some(m.network_tx_instant_bps as f64),
                used_disk_space_bytes: Some(m.used_disk_space_bytes as f64),
                total_disk_space_bytes: Some(m.total_disk_space_bytes as f64),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        return Ok(results);
    }

    // If an interval is specified, proceed with aggregation.
    let duration = end_time - start_time;
    let interval_secs = interval_seconds.unwrap().max(1);

    let (metric_source, time_col, is_aggregated) = if duration <= Duration::hours(1) {
        ("performance_metrics", "time", false)
    } else if duration <= Duration::days(7) {
        ("performance_metrics_summary_1m", "time", true)
    } else if duration <= Duration::days(30) {
        ("performance_metrics_summary_1h", "time", true)
    } else {
        ("performance_metrics_summary_1d", "time", true)
    };
    debug!(?duration, ?interval_seconds, metric_source, "Choosing DuckDB data source for performance query");

    let sql = if !is_aggregated {
        // Query raw data and aggregate on the fly
        format!(
            r#"
            SELECT
                date_trunc('second', "time") + INTERVAL '{interval_secs} seconds' * (epoch("time") / {interval_secs}) AS time_bucket,
                vps_id,
                AVG(cpu_usage_percent),
                AVG(memory_usage_bytes),
                MAX(memory_total_bytes),
                AVG(swap_usage_bytes),
                AVG(disk_io_read_bps),
                AVG(disk_io_write_bps),
                AVG(network_rx_instant_bps),
                AVG(network_tx_instant_bps),
                AVG(used_disk_space_bytes),
                AVG(total_disk_space_bytes)
            FROM {metric_source}
            WHERE vps_id = ? AND "time" >= ? AND "time" <= ?
            GROUP BY time_bucket, vps_id
            ORDER BY time_bucket ASC
            "#
        )
    } else {
        // Query pre-aggregated data
        format!(
            r#"
            SELECT
                date_trunc('second', "{time_col}") + INTERVAL '{interval_secs} seconds' * (epoch("{time_col}") / {interval_secs}) AS time_bucket,
                vps_id,
                AVG(avg_cpu_usage_percent),
                AVG(avg_memory_usage_bytes),
                MAX(max_memory_total_bytes),
                AVG(avg_swap_usage_bytes),
                AVG(avg_disk_io_read_bps),
                AVG(avg_disk_io_write_bps),
                AVG(avg_network_rx_instant_bps),
                AVG(avg_network_tx_instant_bps),
                AVG(avg_used_disk_space_bytes),
                AVG(avg_total_disk_space_bytes)
            FROM {metric_source}
            WHERE vps_id = ? AND "{time_col}" >= ? AND "{time_col}" <= ?
            GROUP BY time_bucket, vps_id
            ORDER BY time_bucket ASC
            "#
        )
    };

    let mut stmt = conn.prepare(&sql)?;
    let results = stmt.query_map(params![vps_id, start_time, end_time], |row| {
        Ok(PerformanceMetricPoint {
            time: row.get(0)?,
            vps_id: row.get(1)?,
            cpu_usage_percent: row.get(2)?,
            memory_usage_bytes: row.get(3)?,
            memory_total_bytes: row.get(4).map(|v: i64| v as f64).ok(),
            swap_usage_bytes: row.get(5)?,
            disk_io_read_bps: row.get(6)?,
            disk_io_write_bps: row.get(7)?,
            network_rx_instant_bps: row.get(8)?,
            network_tx_instant_bps: row.get(9)?,
            used_disk_space_bytes: row.get(10)?,
            total_disk_space_bytes: row.get(11)?,
        })
    })?
    .collect::<Result<Vec<_>, _>>()?;

    Ok(results)
}


/// Retrieves the latest performance metric for a given VPS from DuckDB.
pub async fn get_latest_performance_metric_for_vps(
    pool: &DuckDbPool,
    vps_id: i32,
) -> Result<Option<performance_metric::Model>, Error> {
    let conn = pool.get()?;
    let sql = "
        SELECT 
            time, vps_id, cpu_usage_percent, memory_usage_bytes, memory_total_bytes, 
            swap_usage_bytes, swap_total_bytes, disk_io_read_bps, disk_io_write_bps, 
            network_rx_cumulative, network_tx_cumulative, network_rx_instant_bps, 
            network_tx_instant_bps, uptime_seconds, total_processes_count, 
            running_processes_count, tcp_established_connection_count, 
            total_disk_space_bytes, used_disk_space_bytes
        FROM performance_metrics 
        WHERE vps_id = ? ORDER BY time DESC LIMIT 1";

    let mut stmt = conn.prepare(sql)?;
    
    let result = stmt.query_row(params![vps_id], |row| {
        Ok(performance_metric::Model {
            time: row.get(0)?,
            vps_id: row.get(1)?,
            cpu_usage_percent: row.get(2)?,
            memory_usage_bytes: row.get(3)?,
            memory_total_bytes: row.get(4)?,
            swap_usage_bytes: row.get(5)?,
            swap_total_bytes: row.get(6)?,
            disk_io_read_bps: row.get(7)?,
            disk_io_write_bps: row.get(8)?,
            network_rx_cumulative: row.get(9)?,
            network_tx_cumulative: row.get(10)?,
            network_rx_instant_bps: row.get(11)?,
            network_tx_instant_bps: row.get(12)?,
            uptime_seconds: row.get(13)?,
            total_processes_count: row.get(14)?,
            running_processes_count: row.get(15)?,
            tcp_established_connection_count: row.get(16)?,
            total_disk_space_bytes: row.get(17)?,
            used_disk_space_bytes: row.get(18)?,
        })
    });

    match result {
        Ok(model) => Ok(Some(model)),
        Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}


/// This is a stub function to maintain API compatibility.
/// The actual data saving is handled by the `duckdb_service::writer`.
pub async fn save_performance_snapshot_batch(
    _pool: &DuckDbPool,
    _vps_id: i32,
    batch: &PerformanceSnapshotBatch,
) -> Result<Vec<performance_metric::Model>, Error> {
    if !batch.snapshots.is_empty() {
        debug!("`save_performance_snapshot_batch` called in DuckDB service. This is a stub. Data should be sent via the writer channel.");
    }
    Ok(Vec::new())
}

/// Retrieves the summary of total and used disk space from the latest performance metric from DuckDB.
pub async fn get_latest_disk_usage_summary(
    pool: &DuckDbPool,
    vps_id: i32,
) -> Result<Option<(i64, i64)>, Error> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT total_disk_space_bytes, used_disk_space_bytes FROM performance_metrics WHERE vps_id = ? ORDER BY time DESC LIMIT 1",
    )?;

    let result = stmt.query_row(params![vps_id], |row| {
        Ok((row.get(0)?, row.get(1)?))
    });

    match result {
        Ok(summary) => Ok(Some(summary)),
        Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}