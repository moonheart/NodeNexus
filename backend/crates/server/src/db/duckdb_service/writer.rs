use crate::db::entities::performance_metric;
use duckdb::{params, Connection};
use std::{sync::mpsc, time::Duration};
use tracing::{error, info};

const BATCH_SIZE: usize = 100;
const FLUSH_INTERVAL_SECONDS: u64 = 10;

/// 后台任务，在一个专用的 OS 线程中运行。
/// 它从队列中读取指标并将其批量写入数据库。
pub(super) fn metrics_writer_task(
    pool: super::DuckDbPool,
    rx: mpsc::Receiver<performance_metric::Model>,
) {
    info!("DuckDB metrics writer thread started.");

    // 在这个线程中创建唯一的数据库连接。
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            error!("Writer thread failed to get DuckDB connection from pool: {}", e);
            return;
        }
    };

    let mut buffer = Vec::with_capacity(BATCH_SIZE);
    let flush_interval = Duration::from_secs(FLUSH_INTERVAL_SECONDS);

    // Loop to receive messages with a timeout.
    loop {
        match rx.recv_timeout(flush_interval) {
            Ok(metric) => {
                buffer.push(metric);
                if buffer.len() >= BATCH_SIZE {
                    if let Err(e) = flush_metrics_to_db(&mut conn, &mut buffer) {
                        error!("Failed to flush metrics to DuckDB on batch size: {}", e);
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Timeout occurred, flush the buffer if it's not empty.
                if !buffer.is_empty() {
                    if let Err(e) = flush_metrics_to_db(&mut conn, &mut buffer) {
                        error!("Failed to flush metrics to DuckDB on interval: {}", e);
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // Channel has been closed.
                info!("Metrics channel closed. Flushing remaining metrics and shutting down writer thread.");
                if !buffer.is_empty() {
                    if let Err(e) = flush_metrics_to_db(&mut conn, &mut buffer) {
                        error!("Failed to flush remaining metrics to DuckDB: {}", e);
                    }
                }
                break;
            }
        }
    }
    info!("DuckDB metrics writer thread finished.");
}

/// 将缓冲区中的指标刷新到数据库 (同步版本)
fn flush_metrics_to_db(
    conn: &mut Connection, // 接收可变引用以创建事务
    buffer: &mut Vec<performance_metric::Model>,
) -> duckdb::Result<()> {
    if buffer.is_empty() {
        return Ok(());
    }

    info!("Flushing {} metrics to DuckDB.", buffer.len());

    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO performance_metrics (
                time, vps_id, cpu_usage_percent, memory_usage_bytes, memory_total_bytes,
                disk_io_read_bps, disk_io_write_bps, network_rx_cumulative, network_tx_cumulative,
                swap_usage_bytes, swap_total_bytes, uptime_seconds, total_processes_count,
                running_processes_count, tcp_established_connection_count, network_rx_instant_bps,
                network_tx_instant_bps, total_disk_space_bytes, used_disk_space_bytes
            ) VALUES (
                ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?
            )",
        )?;

        for metric in buffer.drain(..) { // 使用 drain 清空 buffer
            stmt.execute(params![
                metric.time,
                metric.vps_id,
                metric.cpu_usage_percent,
                { metric.memory_usage_bytes },
                { metric.memory_total_bytes },
                { metric.disk_io_read_bps },
                { metric.disk_io_write_bps },
                { metric.network_rx_cumulative },
                { metric.network_tx_cumulative },
                { metric.swap_usage_bytes },
                { metric.swap_total_bytes },
                { metric.uptime_seconds },
                { metric.total_processes_count },
                { metric.running_processes_count },
                { metric.tcp_established_connection_count },
                { metric.network_rx_instant_bps },
                { metric.network_tx_instant_bps },
                { metric.total_disk_space_bytes },
                { metric.used_disk_space_bytes },
            ])?;
        }
    }
    tx.commit()?;

    Ok(())
}