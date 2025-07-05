use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServiceMonitorDetails {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub monitor_type: String,
    pub target: String,
    pub frequency_seconds: i32,
    pub timeout_seconds: i32,
    pub is_active: bool,
    pub monitor_config: Value,
    pub created_at: String,
    pub updated_at: String,
    pub agent_ids: Vec<i32>,
    pub tag_ids: Vec<i32>,
    pub assignment_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_check: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_message: Option<String>,
}

// Model for representing monitor assignments in API requests
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MonitorAssignments {
    pub agent_ids: Option<Vec<i32>>,
    pub tag_ids: Option<Vec<i32>>,
    pub assignment_type: Option<String>,
}

// Model for creating a new service monitor
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CreateMonitor {
    pub name: String,
    pub monitor_type: String,
    pub target: String,
    pub frequency_seconds: Option<i32>,
    pub timeout_seconds: Option<i32>,
    pub is_active: Option<bool>,
    pub monitor_config: Option<serde_json::Value>,
    pub assignments: MonitorAssignments,
}

// Model for updating an existing service monitor
#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMonitor {
    pub name: Option<String>,
    pub monitor_type: Option<String>,
    pub target: Option<String>,
    pub frequency_seconds: Option<i32>,
    pub timeout_seconds: Option<i32>,
    pub is_active: Option<bool>,
    pub monitor_config: Option<serde_json::Value>,
    pub assignments: Option<MonitorAssignments>,
}
#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServiceMonitorResultDetails {
    pub time: String,
    pub monitor_id: i32,
    pub agent_id: i32,
    pub agent_name: String,
    pub monitor_name: String,
    pub is_up: bool,
    pub latency_ms: Option<i32>,
    pub details: Option<Value>,
}
