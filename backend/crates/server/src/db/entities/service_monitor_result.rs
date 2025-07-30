use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub time: chrono::DateTime<chrono::Utc>,
    pub monitor_id: i32,
    pub agent_id: i32,
    pub is_up: bool,
    pub latency_ms: Option<i32>,
    pub details: Option<serde_json::Value>,
}
