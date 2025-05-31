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
    pub ip_address: String,
    pub os_type: Option<String>,
    pub agent_secret: String, // Secret for agent authentication
    pub status: String,       // e.g., "online", "offline", "pending"
    pub metadata: Option<serde_json::Value>, // For storing additional info like vendor details
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Represents a performance metric snapshot for a VPS.
/// Corresponds to the `performance_metrics` hypertable.
/// Note: `time` is the hypertable's time dimension.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PerformanceMetric {
    pub time: DateTime<Utc>,
    pub vps_id: i32,
    pub cpu_usage: f64, // Assuming FLOAT maps to f64
    pub mem_usage: f64,
    pub disk_io_read: i64,
    pub disk_io_write: i64,
    pub net_rx: i64,
    pub net_tx: i64,
    // According to arch_design.md, more details are in PerformanceSnapshot proto.
    // For the DB model, we'll stick to the defined columns for now.
    // Consider adding fields like `disk_usage_percent`, `load_average` if they become direct columns.
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

/// Represents an alert rule defined by a user.
/// Corresponds to the `alert_rules` table.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AlertRule {
    pub id: i32,
    pub user_id: i32,
    pub vps_id: Option<i32>, // Can be global if null
    pub metric_type: String, // e.g., "cpu_usage", "mem_usage"
    pub threshold: f64,
    pub comparison_operator: String, // e.g., ">", "<", "="
    pub duration_seconds: i32,
    pub notification_channel: String, // e.g., "email", "slack"
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