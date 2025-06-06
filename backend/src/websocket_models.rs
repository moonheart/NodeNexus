use serde::Serialize;
use chrono::{DateTime, Utc};

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerBasicInfo {
    pub id: i32,
    pub name: String,
    pub ip_address: Option<String>,
    pub status: String,
    #[serde(rename = "group")]
    pub group: Option<String>,
    pub tags: Option<String>,
    // Add other fields as necessary to match VpsListItemResponse or db model
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
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerWithDetails {
    #[serde(flatten)]
    pub basic_info: ServerBasicInfo,
    pub latest_metrics: Option<ServerMetricsSnapshot>,
    pub os_type: Option<String>,
    pub created_at: DateTime<Utc>, // Assuming this comes from the database Vps model
    // Add other metadata fields if needed, e.g., from Vps->metadata JSON blob
    // pub metadata: Option<serde_json::Value>, // Example if you have a generic metadata field
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct FullServerListPush {
    pub servers: Vec<ServerWithDetails>,
}