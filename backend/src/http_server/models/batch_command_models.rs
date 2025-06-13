use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateBatchCommandRequest {
    pub command_content: Option<String>,
    pub script_id: Option<String>,
    pub working_directory: Option<String>,
    pub target_vps_ids: Vec<String>, // Assuming vps_id is String, adjust if it's Uuid or i32
    pub execution_alias: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchCommandAcceptedResponse {
    pub batch_command_id: Uuid,
    pub status: String, // e.g., "PENDING" or "ACCEPTED"
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChildCommandTaskDetail {
    pub child_command_id: Uuid,
    pub vps_id: String,
    // pub vps_name: Option<String>, // Consider adding this for better UI display
    pub status: String,
    pub exit_code: Option<i32>,
    pub error_message: Option<String>,
    // pub stdout_summary: Option<Vec<String>>, // For brief output in list view
    // pub stderr_summary: Option<Vec<String>>, // For brief output in list view
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub agent_started_at: Option<DateTime<Utc>>,
    pub agent_completed_at: Option<DateTime<Utc>>,
    pub last_output_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchCommandTaskDetailResponse {
    pub batch_command_id: Uuid,
    pub overall_status: String,
    pub execution_alias: Option<String>,
    pub user_id: String,
    pub original_request_payload: serde_json::Value, // Store the original request for audit/retry
    pub tasks: Vec<ChildCommandTaskDetail>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

// You might also want a simpler list item response for a GET /api/batch_commands (list all) endpoint
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BatchCommandTaskListItem {
    pub batch_command_id: Uuid,
    pub overall_status: String,
    pub execution_alias: Option<String>,
    pub user_id: String,
    pub target_vps_count: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub pending_or_executing_count: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}