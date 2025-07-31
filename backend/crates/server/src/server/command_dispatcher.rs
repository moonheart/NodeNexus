use futures_util::SinkExt;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use tokio::sync::Mutex;
use tracing::{error, info, warn};
use uuid::Uuid; // Import the SinkExt trait

use crate::db;
use crate::db::duckdb_service::DuckDbPool;
use crate::server::agent_state::ConnectedAgents; // To get agent connections (gRPC clients)
// AgentCommandServiceClient is not used directly here anymore as we use the existing stream sender
use nodenexus_common::agent_service::{
    BatchAgentCommandRequest,       // Renamed and moved
    BatchTerminateCommandRequest,   // Added for termination
    CommandType as GrpcCommandType, // This is from batch_command.proto, now part of agent_service
    MessageToAgent,
    message_to_agent,
};
use crate::db::enums::ChildCommandStatus; // For updating task status
use crate::server::result_broadcaster::ResultBroadcaster;
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
    DbUpdateError(String), // From batch_command_service
    #[error("Invalid VPS ID format: {0}")]
    InvalidVpsId(String),
}

#[derive(Clone)]
pub struct CommandDispatcher {
    connected_agents: Arc<Mutex<ConnectedAgents>>,
    duckdb_pool: DuckDbPool,
    result_broadcaster: Arc<ResultBroadcaster>,
}

impl CommandDispatcher {
    pub fn new(
        connected_agents: Arc<Mutex<ConnectedAgents>>,
        duckdb_pool: DuckDbPool,
        result_broadcaster: Arc<ResultBroadcaster>,
    ) -> Self {
        Self {
            connected_agents,
            duckdb_pool,
            result_broadcaster,
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
        let agent_sender = {
            // Scope to release the lock quickly
            let agents_guard = self.connected_agents.lock().await;
            agents_guard
                .find_by_vps_id(vps_id)
                .map(|state| state.sender)
        };

        match agent_sender {
            Some(mut sender) => {
                // Make sender mutable
                // Update status to SentToAgent before actually sending
                db::duckdb_service::batch_command_service::update_child_task_status(
                    self.duckdb_pool.clone(),
                    self.result_broadcaster.clone(),
                    child_task_id,
                    ChildCommandStatus::SentToAgent,
                    None,
                    None,
                )
                .await
                .map_err(|e| DispatcherError::DbUpdateError(e.to_string()))?;

                let batch_command_req = BatchAgentCommandRequest {
                    command_id: child_task_id.to_string(),
                    r#type: command_type.into(), // Ensure GrpcCommandType is convertible to i32 if needed by proto
                    content: command_content.to_string(),
                    working_directory: working_directory.unwrap_or_default(), // Proto expects string, not Option<String>
                };
                let message_to_agent = MessageToAgent {
                    server_message_id: NEXT_SERVER_MESSAGE_ID.fetch_add(1, Ordering::Relaxed),
                    payload: Some(message_to_agent::Payload::BatchAgentCommandRequest(
                        batch_command_req,
                    )),
                };

                if let Err(e) = sender.send(message_to_agent).await {
                    // If sending fails, update status to AgentUnreachable
                    db::duckdb_service::batch_command_service::update_child_task_status(
                        self.duckdb_pool.clone(),
                        self.result_broadcaster.clone(),
                        child_task_id,
                        ChildCommandStatus::AgentUnreachable,
                        Some(format!("Failed to send command to agent via mpsc: {e}")),
                        None,
                    )
                    .await
                    .map_err(|db_err| DispatcherError::DbUpdateError(db_err.to_string()))?;
                    return Err(DispatcherError::MpscSendError(e.to_string()));
                }

                info!("Successfully dispatched command to agent.");
                // TODO: Spawn a task to handle the response stream (AgentToServerMessage)
                // This task would listen on a channel associated with this agent's communication stream
                // and process BatchCommandOutputStream and BatchCommandResult messages.
                // It would then call BatchCommandManager methods to update DB.
            }
            None => {
                db::duckdb_service::batch_command_service::update_child_task_status(
                    self.duckdb_pool.clone(),
                    self.result_broadcaster.clone(),
                    child_task_id,
                    ChildCommandStatus::AgentUnreachable,
                    Some("Agent not connected or found.".to_string()),
                    None,
                )
                .await
                .map_err(|e| DispatcherError::DbUpdateError(e.to_string()))?;
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
            agents_guard
                .find_by_vps_id(vps_id)
                .map(|state| state.sender)
        };

        match agent_sender {
            Some(mut sender) => {
                // Make sender mutable
                let terminate_req = BatchTerminateCommandRequest {
                    command_id: child_task_id.to_string(),
                };

                let message_to_agent = MessageToAgent {
                    server_message_id: NEXT_SERVER_MESSAGE_ID.fetch_add(1, Ordering::Relaxed),
                    payload: Some(message_to_agent::Payload::BatchTerminateCommandRequest(
                        terminate_req,
                    )),
                };

                if let Err(e) = sender.send(message_to_agent).await {
                    error!(error = %e, "Failed to send terminate command to agent via mpsc. Marking as terminated.");
                    // If sending fails, the agent is unreachable. We should finalize the termination.
                    db::duckdb_service::batch_command_service::update_child_task_status(
                        self.duckdb_pool.clone(),
                        self.result_broadcaster.clone(),
                        child_task_id,
                        ChildCommandStatus::Terminated,
                        Some(format!("Agent unreachable during termination: {e}")),
                        None,
                    )
                    .await
                    .map_err(|db_err| DispatcherError::DbUpdateError(db_err.to_string()))?;
                } else {
                    info!("Successfully dispatched terminate command to agent.");
                }
            }
            None => {
                warn!(
                    "Agent not found when trying to send terminate. Marking as terminated."
                );
                // Agent not found, so we can consider the termination complete for this task.
                db::duckdb_service::batch_command_service::update_child_task_status(
                    self.duckdb_pool.clone(),
                    self.result_broadcaster.clone(),
                    child_task_id,
                    ChildCommandStatus::Terminated,
                    Some("Agent not connected, task terminated.".to_string()),
                    None,
                )
                .await
                .map_err(|e| DispatcherError::DbUpdateError(e.to_string()))?;
            }
        }
        Ok(())
    }
}
