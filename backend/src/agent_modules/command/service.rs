use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, warn};

use crate::agent_modules::command::execution::manage_command_lifecycle;
use crate::agent_modules::command::tracker::RunningCommandsTracker;
use crate::agent_service::{
    BatchAgentCommandRequest, BatchCommandResult, BatchTerminateCommandRequest, CommandStatus,
    MessageToServer, message_to_server::Payload as ServerPayload,
};

/// This is the main function for handling a new command execution request.
/// It spawns a dedicated "management task" for each command to handle its entire lifecycle.
pub async fn handle_batch_agent_command(
    request: BatchAgentCommandRequest,
    tx_to_server: mpsc::Sender<MessageToServer>,
    command_tracker: Arc<RunningCommandsTracker>,
    _original_server_message_id: u64,
    agent_id: String,
    vps_db_id: i32,
    agent_secret: String,
    id_provider: impl Fn() -> u64 + Send + Sync + Clone + 'static,
) {
    info!("Received command request.");

    // Create a one-shot channel for termination signaling.
    let (term_tx, term_rx) = oneshot::channel();

    // Add the termination sender to the tracker.
    command_tracker.add_command(request.command_id.clone(), term_tx);

    // Spawn the dedicated management task.
    tokio::spawn(async move {
        // The command management logic is now fully encapsulated here.
        manage_command_lifecycle(
            request,
            tx_to_server,
            command_tracker,
            agent_id,
            vps_db_id,
            agent_secret,
            id_provider,
            term_rx, // Pass the receiver to the lifecycle manager
        )
        .await;
    });
}

/// This is the handler for termination requests. It's now much simpler.
pub async fn handle_batch_terminate_command(
    request: BatchTerminateCommandRequest,
    tx_to_server: mpsc::Sender<MessageToServer>,
    command_tracker: Arc<RunningCommandsTracker>,
    _original_server_message_id: u64,
    _agent_id: String,
    vps_db_id: i32,
    agent_secret: String,
    id_provider: impl Fn() -> u64 + Send + Sync + Clone + 'static,
) {
    let command_id_to_terminate = request.command_id;
    info!("Received termination request.");

    // Simply signal the command's managing task to terminate.
    // The managing task is responsible for the actual killing and result reporting.
    if let Err(e) = command_tracker.signal_termination(&command_id_to_terminate) {
        // This case happens if the command already completed or was terminated.
        // We can send a message back to the server to confirm we tried, but the command was already gone.
        warn!(error = %e, "Termination signal failed, command likely already finished.");
        let result_payload = BatchCommandResult {
            command_id: command_id_to_terminate.clone(),
            status: CommandStatus::Terminated.into(), // We can consider it "Terminated" as the end state is correct.
            exit_code: -1,
            error_message: format!(
                "Termination signal sent, but command was already completed or terminated: {e}"
            ),
        };
        let client_msg_id = id_provider();
        if tx_to_server
            .send(MessageToServer {
                client_message_id: client_msg_id,
                payload: Some(ServerPayload::BatchCommandResult(result_payload)),
                vps_db_id,
                agent_secret,
            })
            .await
            .is_err()
        {
            error!("Failed to send 'already terminated' result.");
        }
    }
}
