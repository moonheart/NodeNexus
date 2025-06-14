use tokio::sync::broadcast;
use uuid::Uuid;
use serde_json::json; // For creating JSON payloads easily

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
                if let Err(e) = self.batch_updates_tx.send(json_string.clone()) {
                    eprintln!("Failed to broadcast batch command update ({}): {}. Message: {}", message_type, e, json_string);
                } else {
                    // Successfully sent, no need to log here as it can be noisy.
                    // Or, use a lower log level like debug! or trace!
                }
            }
            Err(e) => {
                eprintln!("Failed to serialize batch command update message ({}): {}", message_type, e);
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
        println!(
            "[ResultBroadcaster] Broadcasting child task update: child_id={}, status={}, exit_code={:?}",
            child_task_id, status, exit_code
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
        println!(
            "[ResultBroadcaster] Broadcasting batch task update: batch_id={}, status={}, completed_at={:?}",
            batch_command_id, overall_status, completed_at
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
        timestamp: String, // Assuming DateTimeUtc to string
    ) {
        println!(
            "[ResultBroadcaster] Broadcasting new log output: child_id={}, stream={}, line_length={}",
            child_task_id, stream_type, log_line.len()
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