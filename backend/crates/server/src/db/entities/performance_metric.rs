use chrono::TimeZone;
use serde::{Deserialize, Serialize};
use nodenexus_common::agent_service::PerformanceSnapshot;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Model {
    pub time: chrono::DateTime<chrono::Utc>,
    pub vps_id: i32,
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: i64,
    pub memory_total_bytes: i64,
    pub swap_usage_bytes: i64,
    pub swap_total_bytes: i64,
    pub disk_io_read_bps: i64,
    pub disk_io_write_bps: i64,
    pub total_disk_space_bytes: i64,
    pub used_disk_space_bytes: i64,
    pub network_rx_cumulative: i64,
    pub network_tx_cumulative: i64,
    pub network_rx_instant_bps: i64,
    pub network_tx_instant_bps: i64,
    pub uptime_seconds: i64,
    pub total_processes_count: i32,
    pub running_processes_count: i32,
    pub tcp_established_connection_count: i32,
}

impl Model {
    pub fn from_snapshot(vps_id: i32, snapshot: &PerformanceSnapshot) -> Self {
        Self {
            time: chrono::Utc.timestamp_millis_opt(snapshot.timestamp_unix_ms).unwrap(),
            vps_id,
            cpu_usage_percent: snapshot.cpu_overall_usage_percent as f64,
            memory_usage_bytes: snapshot.memory_usage_bytes as i64,
            memory_total_bytes: snapshot.memory_total_bytes as i64,
            swap_usage_bytes: snapshot.swap_usage_bytes as i64,
            swap_total_bytes: snapshot.swap_total_bytes as i64,
            disk_io_read_bps: snapshot.disk_total_io_read_bytes_per_sec as i64,
            disk_io_write_bps: snapshot.disk_total_io_write_bytes_per_sec as i64,
            total_disk_space_bytes: snapshot.total_disk_space_bytes as i64,
            used_disk_space_bytes: snapshot.used_disk_space_bytes as i64,
            network_rx_cumulative: snapshot.network_rx_bytes_cumulative as i64,
            network_tx_cumulative: snapshot.network_tx_bytes_cumulative as i64,
            network_rx_instant_bps: snapshot.network_rx_bytes_per_sec as i64,
            network_tx_instant_bps: snapshot.network_tx_bytes_per_sec as i64,
            uptime_seconds: snapshot.uptime_seconds as i64,
            total_processes_count: snapshot.total_processes_count as i32,
            running_processes_count: snapshot.running_processes_count as i32,
            tcp_established_connection_count: snapshot.tcp_established_connection_count as i32,
        }
    }
}
