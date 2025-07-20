use serde_json::json; // For creating JSON payloads easily
use tokio::sync::broadcast;
use tracing::{debug, error, info};
use uuid::Uuid;

// Placeholder for the actual WebSocket message types for batch commands.
// For now, we'll assume String messages (JSON serialized).
pub type BatchCommandUpdateMsg = String;

#[derive(Debug, Clone)]
pub struct ResultBroadcaster {
    batch_updates_tx: broadcast::Sender<BatchCommandUpdateMsg>,
}

impl ResultBroadcaster {
    pub fn new(batch_updates_tx: broadcast::Sender<BatchCommandUpdateMsg>) -> Self {
        Self { batch_updates_tx }
    }
    pub fn subscribe(&self) -> broadcast::Receiver<BatchCommandUpdateMsg> {
        self.batch_updates_tx.subscribe()
    }

    // Placeholder: In a real implementation, these would serialize specific event DTOs.
    fn send_message(&self, message_type: &str, payload: serde_json::Value) {
        let message_content = json!({
            "type": message_type,
            "payload": payload,
        });
        match serde_json::to_string(&message_content) {
            Ok(json_string) => {
                let receiver_count = self.batch_updates_tx.receiver_count();
                if receiver_count > 0 {
                    if let Err(e) = self.batch_updates_tx.send(json_string.clone()) {
                        error!(
                            message_type = message_type,
                            error = %e,
                            "Failed to broadcast batch command update to {} receivers.",
                            receiver_count
                        );
                    } else {
                        debug!(
                            message_type = message_type,
                            "Successfully broadcasted batch command update to {} receivers.",
                            receiver_count
                        );
                    }
                } else {
                    debug!(
                        message_type = message_type,
                        "No active receivers, skipping broadcast."
                    );
                }
            }
            Err(e) => {
                error!(
                    message_type = message_type,
                    error = %e,
                    "Failed to serialize batch command update message."
                );
            }
        }
    }

    pub async fn broadcast_child_task_update(
        &self,
        batch_command_id: Uuid,
        child_task_id: Uuid,
        vps_id: i32,
        status: String,
        exit_code: Option<i32>,
    ) {
        info!(
            child_task_id = %child_task_id,
            status = %status,
            exit_code = ?exit_code,
            "Broadcasting child task update."
        );
        let payload = json!({
            "batch_command_id": batch_command_id.to_string(),
            "child_command_id": child_task_id.to_string(),
            "vps_id": vps_id,
            "status": status,
            "exit_code": exit_code,
        });
        self.send_message("CHILD_TASK_UPDATE", payload);
    }

    pub async fn broadcast_batch_task_update(
        &self,
        batch_command_id: Uuid,
        overall_status: String,
        completed_at: Option<String>, // Assuming DateTimeUtc to string
    ) {
        info!(
            batch_command_id = %batch_command_id,
            status = %overall_status,
            completed_at = ?completed_at,
            "Broadcasting batch task update."
        );
        let payload = json!({
            "batch_command_id": batch_command_id.to_string(),
            "overall_status": overall_status,
            "completed_at": completed_at,
        });
        self.send_message("BATCH_TASK_UPDATE", payload);
    }

    pub async fn broadcast_new_log_output(
        &self,
        batch_command_id: Uuid,
        child_task_id: Uuid,
        vps_id: i32,
        log_line: String,
        stream_type: String, // "stdout" or "stderr"
        timestamp: String,   // Assuming DateTimeUtc to string
    ) {
        debug!(
            child_task_id = %child_task_id,
            stream = %stream_type,
            line_length = log_line.len(),
            "Broadcasting new log output."
        );
        let payload = json!({
            "batch_command_id": batch_command_id.to_string(),
            "child_command_id": child_task_id.to_string(),
            "vps_id": vps_id,
            "log_line": log_line,
            "stream_type": stream_type,
            "timestamp": timestamp,
        });
        self.send_message("NEW_LOG_OUTPUT", payload);
    }
}
