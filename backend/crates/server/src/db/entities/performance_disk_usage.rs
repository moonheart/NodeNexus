use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Model {
    pub id: i32,
    pub performance_metric_id: i32,
    pub mount_point: String,
    pub used_bytes: i64,
    pub total_bytes: i64,
    pub fstype: Option<String>,
    pub usage_percent: f64,
}
