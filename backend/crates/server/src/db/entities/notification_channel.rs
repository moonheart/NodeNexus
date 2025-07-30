use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub channel_type: String, // e.g., "telegram", "webhook"
    pub config: Vec<u8>,      // Encrypted JSON blob
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
