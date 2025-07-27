use super::{vps_traffic_service, DuckDbPool};
use duckdb::Connection;
use std::{sync::Arc, time::Duration};
use tokio::time;
use tracing::{error, info, instrument};

pub struct DuckDBTaskManager {
    db_path: String,
    pool: DuckDbPool,
}

impl DuckDBTaskManager {
    pub fn new(db_path: &str, pool: DuckDbPool) -> Self {
        Self {
            db_path: db_path.to_string(),
            pool,
        }
    }

    pub async fn run_periodic_tasks(self: Arc<Self>, interval_duration: Duration) {
        info!(
            "Starting DuckDB periodic tasks with interval: {:?}",
            interval_duration
        );
        let mut interval = time::interval(interval_duration);

        loop {
            interval.tick().await;
            
            let self_clone = self.clone();
            tokio::spawn(async move {
                info!("Running scheduled DuckDB aggregation and retention tasks...");
                // Use spawn_blocking for the potentially long-running, blocking DB operations.
                if let Err(e) = tokio::task::spawn_blocking(move || {
                    self_clone.perform_aggregation_and_retention()
                })
                .await
                {
                    error!("Error running DuckDB aggregation/retention task block: {:?}", e);
                }
            });

            let self_clone_for_traffic = self.clone();
            tokio::spawn(async move {
                info!("Running scheduled DuckDB traffic reset task...");
                if let Err(e) = self_clone_for_traffic.perform_traffic_resets().await {
                    error!("Error running DuckDB traffic reset task: {:?}", e);
                }
            });
        }
    }

    #[instrument(skip(self), fields(db_path = %self.db_path))]
    async fn perform_traffic_resets(&self) -> Result<(), super::Error> {
        info!("Checking for VPS due for traffic reset...");
        let vps_ids = vps_traffic_service::get_vps_due_for_traffic_reset(self.pool.clone()).await?;
        
        if vps_ids.is_empty() {
            info!("No VPS due for traffic reset.");
            return Ok(());
        }

        info!("Found {} VPS to reset traffic for.", vps_ids.len());

        for vps_id in vps_ids {
            let pool = self.pool.clone();
            tokio::spawn(async move {
                info!("Processing traffic reset for VPS ID: {}", vps_id);
                if let Err(e) = vps_traffic_service::process_vps_traffic_reset(pool.clone(), vps_id).await {
                    error!("Failed to process traffic reset for VPS ID {}: {:?}", vps_id, e);
                }
            });
        }

        Ok(())
    }

    #[instrument(skip(self), fields(db_path = %self.db_path))]
    fn perform_aggregation_and_retention(&self) -> Result<(), duckdb::Error> {
        info!("Connecting to DuckDB for maintenance tasks...");
        let conn = Connection::open(&self.db_path)?;
        info!("Connection successful. Starting transaction for aggregation...");

        conn.execute_batch("BEGIN TRANSACTION;")?;

        let result = (|| {
            // --- Aggregation Logic ---
            self.aggregate_to_1m(&conn)?;
            self.aggregate_to_1h(&conn)?;
            self.aggregate_to_1d(&conn)?;
            info!("Data aggregation completed.");

            // --- Retention (Cleanup) Logic ---
            self.apply_retention_policies(&conn)?;
            info!("Data retention policy applied.");

            Ok(())
        })();

        if result.is_ok() {
            info!("Committing transaction.");
            conn.execute_batch("COMMIT;")?;
        } else {
            error!("An error occurred during tasks. Rolling back transaction.");
            conn.execute_batch("ROLLBACK;")?;
        }

        result
    }

    fn get_last_aggregated_timestamp(&self, conn: &Connection, table_name: &str) -> Result<Option<String>, duckdb::Error> {
        let mut stmt = conn.prepare(&format!(
            "SELECT strftime(MAX(time), '%Y-%m-%dT%H:%M:%SZ') FROM {table_name}"
        ))?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let timestamp: Option<String> = row.get(0)?;
            return Ok(timestamp);
        }
        Ok(None)
    }

    fn aggregate_to_1m(&self, conn: &Connection) -> Result<(), duckdb::Error> {
        info!("Aggregating data to 1-minute summary table...");
        let last_ts = self.get_last_aggregated_timestamp(conn, "performance_metrics_summary_1m")?
            .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
        let sql = self.generate_aggregation_sql("performance_metrics", "minute");
        conn.execute(&sql, [last_ts])?;
        Ok(())
    }

    fn aggregate_to_1h(&self, conn: &Connection) -> Result<(), duckdb::Error> {
        info!("Aggregating data to 1-hour summary table...");
        let last_ts = self.get_last_aggregated_timestamp(conn, "performance_metrics_summary_1h")?
            .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
        let sql = self.generate_aggregation_sql("performance_metrics_summary_1m", "hour");
        conn.execute(&sql, [last_ts])?;
        Ok(())
    }

    fn aggregate_to_1d(&self, conn: &Connection) -> Result<(), duckdb::Error> {
        info!("Aggregating data to 1-day summary table...");
        let last_ts = self.get_last_aggregated_timestamp(conn, "performance_metrics_summary_1d")?
            .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
        let sql = self.generate_aggregation_sql("performance_metrics_summary_1h", "day");
        conn.execute(&sql, [last_ts])?;
        Ok(())
    }

    fn generate_aggregation_sql(&self, source_table: &str, time_bucket: &str) -> String {
        let target_table = format!(
            "performance_metrics_summary_1{}",
            &time_bucket[..1]
        );

        let (select_fields, from_table) = if source_table == "performance_metrics" {
            (
                r#"
                AVG(cpu_usage_percent), MAX(cpu_usage_percent), MIN(cpu_usage_percent),
                AVG(memory_usage_bytes), MAX(memory_usage_bytes), MIN(memory_usage_bytes), MAX(memory_total_bytes),
                AVG(swap_usage_bytes), MAX(swap_usage_bytes), MIN(swap_usage_bytes), MAX(swap_total_bytes),
                AVG(disk_io_read_bps), MAX(disk_io_read_bps), MIN(disk_io_read_bps),
                AVG(disk_io_write_bps), MAX(disk_io_write_bps), MIN(disk_io_write_bps),
                AVG(total_disk_space_bytes), MAX(total_disk_space_bytes), MIN(total_disk_space_bytes),
                AVG(used_disk_space_bytes), MAX(used_disk_space_bytes), MIN(used_disk_space_bytes),
                AVG(network_rx_instant_bps), MAX(network_rx_instant_bps), MIN(network_rx_instant_bps),
                AVG(network_tx_instant_bps), MAX(network_tx_instant_bps), MIN(network_tx_instant_bps),
                arg_max(network_rx_cumulative, time),
                arg_max(network_tx_cumulative, time),
                MAX(uptime_seconds),
                AVG(total_processes_count), MAX(total_processes_count),
                AVG(running_processes_count), MAX(running_processes_count),
                AVG(tcp_established_connection_count), MAX(tcp_established_connection_count)
                "#.to_string(),
                source_table.to_string(),
            )
        } else {
            (
                r#"
                AVG(avg_cpu_usage_percent), MAX(max_cpu_usage_percent), MIN(min_cpu_usage_percent),
                AVG(avg_memory_usage_bytes), MAX(max_memory_usage_bytes), MIN(min_memory_usage_bytes), MAX(max_memory_total_bytes),
                AVG(avg_swap_usage_bytes), MAX(max_swap_usage_bytes), MIN(min_swap_usage_bytes), MAX(max_swap_total_bytes),
                AVG(avg_disk_io_read_bps), MAX(max_disk_io_read_bps), MIN(min_disk_io_read_bps),
                AVG(avg_disk_io_write_bps), MAX(max_disk_io_write_bps), MIN(min_disk_io_write_bps),
                AVG(avg_total_disk_space_bytes), MAX(max_total_disk_space_bytes), MIN(min_total_disk_space_bytes),
                AVG(avg_used_disk_space_bytes), MAX(max_used_disk_space_bytes), MIN(min_used_disk_space_bytes),
                AVG(avg_network_rx_instant_bps), MAX(max_network_rx_instant_bps), MIN(min_network_rx_instant_bps),
                AVG(avg_network_tx_instant_bps), MAX(max_network_tx_instant_bps), MIN(min_network_tx_instant_bps),
                arg_max(last_network_rx_cumulative, time),
                arg_max(last_network_tx_cumulative, time),
                MAX(max_uptime_seconds),
                AVG(avg_total_processes_count), MAX(max_total_processes_count),
                AVG(avg_running_processes_count), MAX(max_running_processes_count),
                AVG(avg_tcp_established_connection_count), MAX(max_tcp_established_connection_count)
                "#.to_string(),
                source_table.to_string(),
            )
        };

        let update_set_clause = [
            "avg_cpu_usage_percent = excluded.avg_cpu_usage_percent",
            "max_cpu_usage_percent = excluded.max_cpu_usage_percent",
            "min_cpu_usage_percent = excluded.min_cpu_usage_percent",
            "avg_memory_usage_bytes = excluded.avg_memory_usage_bytes",
            "max_memory_usage_bytes = excluded.max_memory_usage_bytes",
            "min_memory_usage_bytes = excluded.min_memory_usage_bytes",
            "max_memory_total_bytes = excluded.max_memory_total_bytes",
            "avg_swap_usage_bytes = excluded.avg_swap_usage_bytes",
            "max_swap_usage_bytes = excluded.max_swap_usage_bytes",
            "min_swap_usage_bytes = excluded.min_swap_usage_bytes",
            "max_swap_total_bytes = excluded.max_swap_total_bytes",
            "avg_disk_io_read_bps = excluded.avg_disk_io_read_bps",
            "max_disk_io_read_bps = excluded.max_disk_io_read_bps",
            "min_disk_io_read_bps = excluded.min_disk_io_read_bps",
            "avg_disk_io_write_bps = excluded.avg_disk_io_write_bps",
            "max_disk_io_write_bps = excluded.max_disk_io_write_bps",
            "min_disk_io_write_bps = excluded.min_disk_io_write_bps",
            "avg_total_disk_space_bytes = excluded.avg_total_disk_space_bytes",
            "max_total_disk_space_bytes = excluded.max_total_disk_space_bytes",
            "min_total_disk_space_bytes = excluded.min_total_disk_space_bytes",
            "avg_used_disk_space_bytes = excluded.avg_used_disk_space_bytes",
            "max_used_disk_space_bytes = excluded.max_used_disk_space_bytes",
            "min_used_disk_space_bytes = excluded.min_used_disk_space_bytes",
            "avg_network_rx_instant_bps = excluded.avg_network_rx_instant_bps",
            "max_network_rx_instant_bps = excluded.max_network_rx_instant_bps",
            "min_network_rx_instant_bps = excluded.min_network_rx_instant_bps",
            "avg_network_tx_instant_bps = excluded.avg_network_tx_instant_bps",
            "max_network_tx_instant_bps = excluded.max_network_tx_instant_bps",
            "min_network_tx_instant_bps = excluded.min_network_tx_instant_bps",
            "last_network_rx_cumulative = excluded.last_network_rx_cumulative",
            "last_network_tx_cumulative = excluded.last_network_tx_cumulative",
            "max_uptime_seconds = excluded.max_uptime_seconds",
            "avg_total_processes_count = excluded.avg_total_processes_count",
            "max_total_processes_count = excluded.max_total_processes_count",
            "avg_running_processes_count = excluded.avg_running_processes_count",
            "max_running_processes_count = excluded.max_running_processes_count",
            "avg_tcp_established_connection_count = excluded.avg_tcp_established_connection_count",
            "max_tcp_established_connection_count = excluded.max_tcp_established_connection_count",
        ].join(",\n                ");

        format!(
            r#"
            INSERT INTO {target_table}
            SELECT
                vps_id,
                date_trunc('{time_bucket}', time) AS time,
                {select_fields}
            FROM {from_table}
            WHERE time > ?
            GROUP BY vps_id, date_trunc('{time_bucket}', time)
            ON CONFLICT (vps_id, time) DO UPDATE SET
                {update_set_clause};
        "#,
        )
    }

    fn apply_retention_policies(&self, conn: &Connection) -> Result<(), duckdb::Error> {
        info!("Applying retention policies...");
        // Delete raw metrics older than 24 hours
        conn.execute("DELETE FROM performance_metrics WHERE time < now() - INTERVAL '24 hours'", [])?;
        // Delete 1m metrics older than 7 days
        conn.execute("DELETE FROM performance_metrics_summary_1m WHERE time < now() - INTERVAL '7 days'", [])?;
        // Delete 1h metrics older than 30 days
        conn.execute("DELETE FROM performance_metrics_summary_1h WHERE time < now() - INTERVAL '30 days'", [])?;
        // Delete 1d metrics older than 365 days
        conn.execute("DELETE FROM performance_metrics_summary_1d WHERE time < now() - INTERVAL '365 days'", [])?;
        Ok(())
    }
}