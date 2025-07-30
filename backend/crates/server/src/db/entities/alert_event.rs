use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub id: i32,
    pub rule_id: i32,
    pub vps_id: i32,
    pub trigger_time: chrono::DateTime<chrono::Utc>,
    pub resolve_time: Option<chrono::DateTime<chrono::Utc>>,
    pub details: Option<String>,
}
