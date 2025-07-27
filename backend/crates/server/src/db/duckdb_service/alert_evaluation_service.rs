use chrono::{DateTime, Utc};
use duckdb::params;
use tokio::task;

use crate::db::{
    duckdb_service::{vps_service, DuckDbPool},
    entities::{performance_metric, vps},
};

#[derive(Debug, thiserror::Error)]
pub enum AlertEvaluationDbError {
    #[error("Database pool error: {0}")]
    PoolError(#[from] r2d2::Error),
    #[error("Database error: {0}")]
    DbErr(#[from] duckdb::Error),
    #[error("Join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("VPS service error: {0}")]
    VpsServiceError(#[from] crate::web::error::AppError),
}

pub async fn get_performance_metrics(
    pool: DuckDbPool,
    vps_id: i32,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
) -> Result<Vec<performance_metric::Model>, AlertEvaluationDbError> {
    task::spawn_blocking(move || {
        let conn = pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT * FROM performance_metrics WHERE vps_id = ? AND time >= ? AND time <= ? ORDER BY time ASC",
        )?;
        let metrics_iter = stmt.query_map(params![vps_id, start_time, end_time], |row| {
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
                total_disk_space_bytes: row.get(9)?,
                used_disk_space_bytes: row.get(10)?,
                network_rx_cumulative: row.get(11)?,
                network_tx_cumulative: row.get(12)?,
                network_rx_instant_bps: row.get(13)?,
                network_tx_instant_bps: row.get(14)?,
                uptime_seconds: row.get(15)?,
                total_processes_count: row.get(16)?,
                running_processes_count: row.get(17)?,
                tcp_established_connection_count: row.get(18)?,
            })
        })?;

        let metrics = metrics_iter.collect::<Result<Vec<_>, _>>()?;
        Ok(metrics)
    })
    .await?
}

pub async fn get_all_vps_for_user(
    pool: DuckDbPool,
    user_id: i32,
) -> Result<Vec<vps::Model>, AlertEvaluationDbError> {
    let vps_list = vps_service::get_vps_by_user_id(pool, user_id).await?;
    Ok(vps_list)
}