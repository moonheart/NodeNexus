use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Represents a performance metric snapshot for a VPS.
/// Corresponds to the `performance_metrics` hypertable.
/// Note: `time` is the hypertable's time dimension.
#[derive(Debug, Clone, Serialize, Deserialize)] // Removed FromRow
pub struct PerformanceMetric {
    pub id: i32, // Primary key, added from migration
    pub time: DateTime<Utc>,
    pub vps_id: i32,
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: i64,
    pub memory_total_bytes: i64,
    pub swap_usage_bytes: i64, // Added from migration
    pub swap_total_bytes: i64, // Added from migration
    pub disk_io_read_bps: i64,
    pub disk_io_write_bps: i64,
    pub network_rx_bps: i64, // Cumulative RX bytes (default interface)
    pub network_tx_bps: i64, // Cumulative TX bytes (default interface)
    pub network_rx_instant_bps: i64, // Instantaneous RX BPS (default interface) - Added
    pub network_tx_instant_bps: i64, // Instantaneous TX BPS (default interface) - Added
    // Removed load_average fields
    pub uptime_seconds: i64,          // Added from migration
    pub total_processes_count: i32,   // Added from migration
    pub running_processes_count: i32, // Added from migration
    pub tcp_established_connection_count: i32, // Added from migration
                                      // Note: Detailed disk and network stats are in separate tables.
}

/// Represents a complete AlertRule for API responses, including associated channel IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertRule {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub vps_id: Option<i32>,
    pub metric_type: String,
    pub threshold: f64,
    pub comparison_operator: String,
    pub duration_seconds: i32,
    pub notification_channel_ids: Option<Vec<i32>>, // Manually populated
    pub is_active: bool,
    pub last_triggered_at: Option<DateTime<Utc>>,
    pub cooldown_seconds: i32, // Added
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Represents an aggregated performance metric, typically used for time-bucketed queries.
/// Fields are optional because not all aggregations will produce all values (e.g., raw data might not have averages).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")] // Ensure all fields are camelCase in JSON
pub struct AggregatedPerformanceMetric {
    pub time: Option<DateTime<Utc>>, // Represents the start of the time bucket, made Option
    pub vps_id: i32,                 // Will be serialized as vpsId
    pub avg_cpu_usage_percent: Option<f64>,
    pub avg_memory_usage_bytes: Option<f64>, // Using f64 for AVG
    pub max_memory_total_bytes: Option<i64>, // MAX might be more appropriate for total
    pub avg_network_rx_instant_bps: Option<f64>, // Average of instantaneous Rx BPS
    pub avg_network_tx_instant_bps: Option<f64>, // Average of instantaneous Tx BPS
    pub avg_disk_io_read_bps: Option<f64>,   // Added
    pub avg_disk_io_write_bps: Option<f64>,  // Added
                                             // Add other aggregated fields as needed
}

/// Represents a tag that can be associated with a VPS.
/// Corresponds to the `tags` table.
#[derive(Debug, Clone, Serialize, Deserialize)] // Removed FromRow
pub struct Tag {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub color: String,
    pub icon: Option<String>,
    pub url: Option<String>,
    pub is_visible: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// DTO for raw performance metric points, ensuring camelCase for API responses.
/// Used by the /latest-n endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawPerformanceMetricPointDto {
    pub time: DateTime<Utc>,
    pub vps_id: i32, // Will be serialized as vpsId
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: i64,
    pub memory_total_bytes: i64,
    pub swap_usage_bytes: i64,
    pub swap_total_bytes: i64,
    pub disk_io_read_bps: i64,
    pub disk_io_write_bps: i64,
    pub network_rx_instant_bps: i64,
    pub network_tx_instant_bps: i64,
    pub uptime_seconds: i64,
    pub total_processes_count: i32,
    pub running_processes_count: i32,
    pub tcp_established_connection_count: i32,
    // Note: network_rx_bps and network_tx_bps (cumulative) are omitted as this DTO
    // is for "instant" points, similar to what's used in charts.
}
