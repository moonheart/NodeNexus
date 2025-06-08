use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

/// Represents a user in the system.
/// Corresponds to the `users` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i32,
    pub username: String,
    pub password_hash: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Represents a Virtual Private Server (VPS) monitored by the system.
/// Corresponds to the `vps` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Vps {
    pub id: i32,
    pub user_id: i32, // Foreign key to User
    pub name: String,
    pub ip_address: Option<String>, // Made optional
    pub os_type: Option<String>,
    pub agent_secret: String, // Secret for agent authentication
    pub status: String,       // e.g., "online", "offline", "pending"
    pub metadata: Option<serde_json::Value>, // For storing additional info like vendor details
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(rename = "group")]
    pub group: Option<String>,
    // New fields for config management
    pub agent_config_override: Option<serde_json::Value>,
    pub config_status: String,
    pub last_config_update_at: Option<DateTime<Utc>>,
    pub last_config_error: Option<String>,
}

/// Represents a performance metric snapshot for a VPS.
/// Corresponds to the `performance_metrics` hypertable.
/// Note: `time` is the hypertable's time dimension.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PerformanceMetric {
    pub id: i32, // Primary key, added from migration
    pub time: DateTime<Utc>,
    pub vps_id: i32,
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: i64,
    pub memory_total_bytes: i64,
    pub swap_usage_bytes: i64,    // Added from migration
    pub swap_total_bytes: i64,    // Added from migration
    pub disk_io_read_bps: i64,
    pub disk_io_write_bps: i64,
    pub network_rx_bps: i64,      // Cumulative RX bytes (default interface)
    pub network_tx_bps: i64,      // Cumulative TX bytes (default interface)
    pub network_rx_instant_bps: i64, // Instantaneous RX BPS (default interface) - Added
    pub network_tx_instant_bps: i64, // Instantaneous TX BPS (default interface) - Added
    // Removed load_average fields
    pub uptime_seconds: i64,      // Added from migration
    pub total_processes_count: i32, // Added from migration
    pub running_processes_count: i32, // Added from migration
    pub tcp_established_connection_count: i32, // Added from migration
    // Note: Detailed disk and network stats are in separate tables.
}

/// Represents a Docker container on a VPS.
/// Corresponds to the `docker_containers` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DockerContainer {
    pub id: i32, // Primary key for this table
    pub vps_id: i32,
    pub container_id_on_host: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub created_at_on_host: Option<DateTime<Utc>>, // Or NaiveDateTime if no TZ info from agent
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // `labels`, `mounts` from proto could be stored in a jsonb column if needed
}

/// Represents a performance metric snapshot for a Docker container.
/// Corresponds to the `docker_metrics` hypertable.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DockerMetric {
    pub time: DateTime<Utc>,
    pub container_db_id: i32, // Foreign key to docker_containers.id
    pub cpu_usage: f64,
    pub mem_usage: f64, // Assuming this will store bytes as i64 or f64 depending on precision needs
}

/// Represents a defined task.
/// Corresponds to the `tasks` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Task {
    pub id: i32,
    pub user_id: i32,
    pub vps_id_target: Option<i32>, // Can be null if task is not VPS specific
    pub name: String,
    #[serde(rename = "type")] // To allow "type" as field name
    pub task_type: String, // e.g., "shell", "ansible"
    pub schedule_cron: Option<String>,
    pub command_payload: Option<serde_json::Value>,
    pub ansible_playbook_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub next_run_at: Option<DateTime<Utc>>,
}

/// Represents an execution record of a task.
/// Corresponds to the `task_runs` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TaskRun {
    pub id: i32,
    pub task_id: i32,
    pub status: String, // e.g., "pending", "running", "success", "failed"
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub output: Option<String>,
}

/// Represents an alert rule defined by a user, directly mapping to `alert_rules` table columns.
/// Used for direct database row mapping.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AlertRuleFromDb {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub vps_id: Option<i32>,
    pub metric_type: String,
    pub threshold: f64,
    pub comparison_operator: String,
    pub duration_seconds: i32,
    pub is_active: bool,
    pub last_triggered_at: Option<DateTime<Utc>>,
    pub cooldown_seconds: i32, // Added
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

/// Represents a triggered alert event.
/// Corresponds to the `alert_events` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AlertEvent {
    pub id: i32,
    pub rule_id: i32,
    pub vps_id: i32,
    pub trigger_time: DateTime<Utc>,
    pub resolve_time: Option<DateTime<Utc>>,
    pub details: Option<String>, // Could be JSON for structured details
}

/// Represents monthly traffic data for a VPS.
/// Corresponds to the `vps_monthly_traffic` table.
/// Note: `month` is part of the primary key.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VpsMonthlyTraffic {
    pub vps_id: i32,
    pub month: chrono::NaiveDate, // DATE type
    pub total_rx: i64,
    pub total_tx: i64,
}

/// Represents detailed disk usage for a specific performance metric snapshot.
/// Corresponds to the `performance_disk_usages` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PerformanceDiskUsage {
    pub id: i32,
    pub performance_metric_id: i32,
    pub mount_point: String,
    pub used_bytes: i64,
    pub total_bytes: i64,
    pub fstype: Option<String>,
    pub usage_percent: f64,
}

/// Represents detailed network interface statistics for a specific performance metric snapshot.
/// Corresponds to the `performance_network_interface_stats` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PerformanceNetworkInterfaceStat {
    pub id: i32,
    pub performance_metric_id: i32,
    pub interface_name: String,
    pub rx_bytes_per_sec: i64,
    pub tx_bytes_per_sec: i64,
    pub rx_packets_per_sec: i64,
    pub tx_packets_per_sec: i64,
    pub rx_errors_total_cumulative: i64,
    pub tx_errors_total_cumulative: i64,
}

/// Represents an aggregated performance metric, typically used for time-bucketed queries.
/// Fields are optional because not all aggregations will produce all values (e.g., raw data might not have averages).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AggregatedPerformanceMetric {
    pub time: Option<DateTime<Utc>>, // Represents the start of the time bucket, made Option
    pub vps_id: i32,
    pub avg_cpu_usage_percent: Option<f64>,
    pub avg_memory_usage_bytes: Option<f64>, // Using f64 for AVG
    pub max_memory_total_bytes: Option<i64>, // MAX might be more appropriate for total
    pub avg_network_rx_instant_bps: Option<f64>, // Average of instantaneous Rx BPS
    pub avg_network_tx_instant_bps: Option<f64>, // Average of instantaneous Tx BPS
    // Add other aggregated fields as needed
}

/// Represents a global setting in the `settings` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Setting {
    pub key: String,
    pub value: serde_json::Value,
    pub updated_at: DateTime<Utc>,
}

/// Represents a tag that can be associated with a VPS.
/// Corresponds to the `tags` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
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

/// Represents the association between a VPS and a Tag.
/// Corresponds to the `vps_tags` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VpsTag {
    pub vps_id: i32,
    pub tag_id: i32,
}

/// Represents a configured notification channel.
/// Corresponds to the `notification_channels` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct NotificationChannel {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub channel_type: String, // e.g., "telegram", "webhook"
    pub config: Vec<u8>,      // Encrypted JSON blob
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Represents the association between an AlertRule and a NotificationChannel.
/// Corresponds to the `alert_rule_channels` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AlertRuleChannel {
    pub alert_rule_id: i32,
    pub channel_id: i32,
}