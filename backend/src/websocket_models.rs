use serde::Serialize;
use chrono::{DateTime, Utc};

/// Represents a tag as it will be sent to the frontend via WebSocket.
use serde::Deserialize;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Tag {
    pub id: i32,
    pub name: String,
    pub color: String,
    pub icon: Option<String>,
    pub url: Option<String>,
    pub is_visible: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerBasicInfo {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub ip_address: Option<String>,
    pub status: String,
    #[serde(rename = "group")]
    pub group: Option<String>,
    pub tags: Option<Vec<Tag>>, // Changed from Option<String>
    // Config status fields
    pub config_status: String,
    pub last_config_update_at: Option<DateTime<Utc>>,
    pub last_config_error: Option<String>,
    // Traffic monitoring fields
    pub traffic_limit_bytes: Option<i64>,
    pub traffic_billing_rule: Option<String>,
    pub traffic_current_cycle_rx_bytes: Option<i64>,
    pub traffic_current_cycle_tx_bytes: Option<i64>,
    pub traffic_last_reset_at: Option<DateTime<Utc>>,
    pub traffic_reset_config_type: Option<String>,
    pub traffic_reset_config_value: Option<String>,
    pub next_traffic_reset_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerMetricsSnapshot {
    pub time: DateTime<Utc>, // Added time field
    pub cpu_usage_percent: f32,
    pub memory_usage_bytes: u64,
    pub memory_total_bytes: u64,
    pub network_rx_instant_bps: Option<u64>,
    pub network_tx_instant_bps: Option<u64>,
    pub uptime_seconds: Option<u64>,
    // Add other metrics fields as necessary
    // Ensure these fields align with frontend/src/types/index.ts LatestPerformanceMetric
    // For example, disk usage might be relevant if it's part of the full server info
    pub disk_used_bytes: Option<u64>,
    pub disk_total_bytes: Option<u64>,
    // Added for disk I/O rates
    pub disk_io_read_bps: Option<u64>,
    pub disk_io_write_bps: Option<u64>,
    // Additional fields to align with frontend's LatestPerformanceMetric
    pub swap_usage_bytes: Option<u64>,
    pub swap_total_bytes: Option<u64>,
    pub network_rx_bps: Option<u64>, // Cumulative RX bytes
    pub network_tx_bps: Option<u64>, // Cumulative TX bytes
    pub total_processes_count: Option<u32>,
    pub running_processes_count: Option<u32>,
    pub tcp_established_connection_count: Option<u32>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerWithDetails {
    #[serde(flatten)]
    pub basic_info: ServerBasicInfo,
    pub latest_metrics: Option<ServerMetricsSnapshot>,
    pub os_type: Option<String>,
    pub created_at: DateTime<Utc>, // Assuming this comes from the database Vps model
    pub metadata: Option<serde_json::Value>, // Added to include VPS metadata

    // Renewal Info Fields
    pub renewal_cycle: Option<String>,
    pub renewal_cycle_custom_days: Option<i32>,
    pub renewal_price: Option<f64>,
    pub renewal_currency: Option<String>,
    pub next_renewal_date: Option<DateTime<Utc>>,
    pub last_renewal_date: Option<DateTime<Utc>>,
    pub service_start_date: Option<DateTime<Utc>>,
    pub payment_method: Option<String>,
    pub auto_renew_enabled: Option<bool>,
    pub renewal_notes: Option<String>,
    pub reminder_active: Option<bool>,
    // pub last_reminder_generated_at: Option<DateTime<Utc>>, // Decided to omit from websocket model for now, primarily backend concern
}

use crate::http_server::models::service_monitor_models::ServiceMonitorResultDetails;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FullServerListPush {
    pub servers: Vec<ServerWithDetails>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "type", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum WsMessage {
    FullServerList(FullServerListPush),
    ServiceMonitorResult(ServiceMonitorResultDetails),
}