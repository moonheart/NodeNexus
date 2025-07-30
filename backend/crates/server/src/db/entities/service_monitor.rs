use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub monitor_type: String,
    pub target: String,
    pub frequency_seconds: i32,
    pub timeout_seconds: i32,
    pub is_active: bool,
    pub assignment_type: String,
    pub monitor_config: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
