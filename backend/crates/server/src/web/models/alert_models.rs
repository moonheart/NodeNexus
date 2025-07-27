use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAlertRuleRequest {
    pub name: String,
    pub vps_id: Option<i32>,
    pub metric_type: String,
    pub threshold: f64,
    pub comparison_operator: String,
    pub duration_seconds: i32,
    pub notification_channel_ids: Option<Vec<i32>>,
    pub cooldown_seconds: Option<i32>, // Added
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAlertRuleRequest {
    pub name: Option<String>,
    pub vps_id: Option<i32>, // Option<Option<i32>> to allow setting vps_id to null
    pub metric_type: Option<String>,
    pub threshold: Option<f64>,
    pub comparison_operator: Option<String>,
    pub duration_seconds: Option<i32>,
    pub notification_channel_ids: Option<Vec<i32>>,
    pub cooldown_seconds: Option<i32>, // Added
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAlertRuleStatusRequest {
    pub is_active: bool,
}

// The response for get/create/update will typically be the db::models::AlertRule struct,
// with notification_channel_ids populated by the service layer.
