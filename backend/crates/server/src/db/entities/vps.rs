use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Model {
    pub id: i32,
    pub user_id: i32, // Foreign key to User
    pub name: String,
    pub ip_address: Option<String>,
    pub os_type: Option<String>,
    pub agent_secret: String,
    pub agent_version: Option<String>,
    pub status: String,
    pub metadata: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub group: Option<String>,
    pub agent_config_override: Option<serde_json::Value>,
    pub config_status: String,
    pub last_config_update_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_config_error: Option<String>,
    pub traffic_limit_bytes: Option<i64>,
    pub traffic_billing_rule: Option<String>,
    pub traffic_current_cycle_rx_bytes: Option<i64>,
    pub traffic_current_cycle_tx_bytes: Option<i64>,
    pub last_processed_cumulative_rx: Option<i64>,
    pub last_processed_cumulative_tx: Option<i64>,
    pub traffic_last_reset_at: Option<chrono::DateTime<chrono::Utc>>,
    pub traffic_reset_config_type: Option<String>,
    pub traffic_reset_config_value: Option<String>,
    pub next_traffic_reset_at: Option<chrono::DateTime<chrono::Utc>>,
}
