use crate::db::enums::BatchCommandStatus;
use serde::{Deserialize, Serialize}; // Added import

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub batch_command_id: uuid::Uuid,
    pub original_request_payload: serde_json::Value,
    pub status: BatchCommandStatus, // Changed type from String
    pub execution_alias: Option<String>,
    pub user_id: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}
