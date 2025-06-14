use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::db::services::BatchCommandManager; // To update task status
use crate::server::agent_state::ConnectedAgents; // To get agent connections (gRPC clients)
// AgentCommandServiceClient is not used directly here anymore as we use the existing stream sender
use crate::agent_service::{
    BatchAgentCommandRequest, // Renamed and moved
    BatchTerminateCommandRequest, // Added for termination
    MessageToAgent,
    message_to_agent,
    CommandType as GrpcCommandType, // This is from batch_command.proto, now part of agent_service
};
use crate::db::enums::ChildCommandStatus; // For updating task status
// Streaming and AgentToServerMessage are not directly used in dispatch_command_to_agent for sending

// Static atomic counter for generating unique server_message_ids for messages sent via CommandDispatcher
static NEXT_SERVER_MESSAGE_ID: AtomicU64 = AtomicU64::new(1);

// Placeholder for a more comprehensive error type for this service
#[derive(Debug, thiserror::Error)]
pub enum DispatcherError {
    #[error("Agent not found for VPS ID: {0}")]
    AgentNotFound(String),
    #[error("Agent connection error for VPS ID: {0} - {1}")]
    AgentConnectionError(String, String),
    #[error("gRPC call failed: {0}")]
    GrpcError(#[from] tonic::Status),
    #[error("Failed to send command to agent via mpsc channel: {0}")]
    MpscSendError(String),
    // ResponseError might be handled by a separate task listening to AgentToServerMessage
    // For now, focusing on dispatch.
    #[error("Database update error: {0}")]
    DbUpdateError(String), // From BatchCommandManager
    #[error("Invalid VPS ID format: {0}")]
    InvalidVpsId(String),
}

#[derive(Clone)]
pub struct CommandDispatcher {
    connected_agents: Arc<Mutex<ConnectedAgents>>,
    batch_command_manager: Arc<BatchCommandManager>,
    // db: Arc<DatabaseConnection>, // Alternatively, pass DB for direct updates if needed
}

impl CommandDispatcher {
    pub fn new(
        connected_agents: Arc<Mutex<ConnectedAgents>>,
        batch_command_manager: Arc<BatchCommandManager>,
    ) -> Self {
        Self {
            connected_agents,
            batch_command_manager,
        }
    }

    pub async fn dispatch_command_to_agent(
        &self,
        child_task_id: Uuid,
        vps_id: i32, // Renamed to avoid confusion
        command_content: &str,
        command_type: GrpcCommandType,
        working_directory: Option<String>,
    ) -> Result<(), DispatcherError> {
        let agent_sender = { // Scope to release the lock quickly
            let agents_guard = self.connected_agents.lock().await;
            agents_guard.find_by_vps_id(vps_id).map(|state| state.sender)
        };

        match agent_sender {
            Some(sender) => {
                // Update status to SentToAgent before actually sending
                self.batch_command_manager.update_child_task_status(
                    child_task_id,
                    ChildCommandStatus::SentToAgent,
                    None,
                    None,
                ).await.map_err(|e| DispatcherError::DbUpdateError(e.to_string()))?;

                let batch_command_req = BatchAgentCommandRequest {
                    command_id: child_task_id.to_string(),
                    r#type: command_type.into(), // Ensure GrpcCommandType is convertible to i32 if needed by proto
                    content: command_content.to_string(),
                    working_directory: working_directory.unwrap_or_default(), // Proto expects string, not Option<String>
                };
let message_to_agent = MessageToAgent {
    server_message_id: NEXT_SERVER_MESSAGE_ID.fetch_add(1, Ordering::Relaxed),
    payload: Some(message_to_agent::Payload::BatchAgentCommandRequest(batch_command_req)),
};


                if let Err(e) = sender.send(Ok(message_to_agent)).await {
                    // If sending fails, update status to AgentUnreachable
                    self.batch_command_manager.update_child_task_status(
                        child_task_id,
                        ChildCommandStatus::AgentUnreachable,
                        None,
                        Some(format!("Failed to send command to agent via mpsc: {}", e)),
                    ).await.map_err(|db_err| DispatcherError::DbUpdateError(db_err.to_string()))?;
                    return Err(DispatcherError::MpscSendError(e.to_string()));
                }
                
                println!("Successfully dispatched command for child_task_id {} to VPS ID {}", child_task_id, vps_id);
                // TODO: Spawn a task to handle the response stream (AgentToServerMessage)
                // This task would listen on a channel associated with this agent's communication stream
                // and process BatchCommandOutputStream and BatchCommandResult messages.
                // It would then call BatchCommandManager methods to update DB.
            }
            None => {
                self.batch_command_manager.update_child_task_status(
                    child_task_id,
                    ChildCommandStatus::AgentUnreachable,
                    None,
                    Some("Agent not connected or found.".to_string()),
                ).await.map_err(|e| DispatcherError::DbUpdateError(e.to_string()))?;
                return Err(DispatcherError::AgentNotFound(vps_id.to_string()));
            }
        }
        Ok(())
    }

    pub async fn terminate_command_on_agent(
        &self,
        child_task_id: Uuid,
        vps_id: i32,
    ) -> Result<(), DispatcherError> {
        let agent_sender = {
            let agents_guard = self.connected_agents.lock().await;
            agents_guard.find_by_vps_id(vps_id).map(|state| state.sender)
        };

        match agent_sender {
            Some(sender) => {
                let terminate_req = BatchTerminateCommandRequest {
                    command_id: child_task_id.to_string(),
                };
 
                let message_to_agent = MessageToAgent {
                    server_message_id: NEXT_SERVER_MESSAGE_ID.fetch_add(1, Ordering::Relaxed),
                    payload: Some(message_to_agent::Payload::BatchTerminateCommandRequest(terminate_req)),
                };

                if let Err(e) = sender.send(Ok(message_to_agent)).await {
                    // Log the error, but the task is already marked as Terminating in DB.
                    // Further DB update here might be redundant or could conflict if agent is truly unreachable.
                    // The primary responsibility here is to attempt sending the termination signal.
                    eprintln!(
                        "Failed to send terminate command for child_task_id {} to VPS ID {} via mpsc: {}",
                        child_task_id, vps_id, e
                    );
                    // We might not want to return an error that stops processing other terminations.
                    // The status in DB is already 'Terminating'.
                    // Consider if this should return an error or just log. For now, let's log and continue.
                    // return Err(DispatcherError::MpscSendError(e.to_string()));
                } else {
                    println!("Successfully dispatched terminate command for child_task_id {} to VPS ID {}", child_task_id, vps_id);
                }
            }
            None => {
                // Agent not found. The task is already marked as Terminating in DB.
                // Log this, but it's not necessarily an error for the dispatcher's attempt.
                println!(
                    "Agent not found for VPS ID {} when trying to send terminate for child_task_id {}. Task already marked Terminating.",
                    vps_id, child_task_id
                );
                // return Err(DispatcherError::AgentNotFound(vps_id_str.to_string()));
            }
        }
        Ok(())
    }
}