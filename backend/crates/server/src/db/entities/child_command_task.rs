use crate::db::enums::ChildCommandStatus;
use serde::{Deserialize, Serialize}; // Added import

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub child_command_id: uuid::Uuid,
    pub batch_command_id: uuid::Uuid,
    pub vps_id: i32,
    pub status: ChildCommandStatus, // Changed type from String
    pub exit_code: Option<i32>,
    pub error_message: Option<String>,
    pub stdout_log_path: Option<String>,
    pub stderr_log_path: Option<String>,
    pub last_output_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub agent_started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub agent_completed_at: Option<chrono::DateTime<chrono::Utc>>,
}
