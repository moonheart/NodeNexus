use std::ffi::OsStr;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command as TokioCommand;
use tokio::sync::{mpsc, oneshot};
use chrono::Utc;
use tracing::{info, error, warn, debug};

use crate::agent_service::{
    message_to_server::Payload as ServerPayload, BatchAgentCommandRequest,
    BatchCommandOutputStream, BatchCommandResult, BatchTerminateCommandRequest, CommandStatus,
    MessageToServer, OutputType,
};
use crate::agent_modules::command_tracker::RunningCommandsTracker;

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
        ).await;
    });
}

/// This function encapsulates the entire lifecycle of a single command.
async fn manage_command_lifecycle(
    request: BatchAgentCommandRequest,
    tx_to_server: mpsc::Sender<MessageToServer>,
    command_tracker: Arc<RunningCommandsTracker>,
    agent_id: String,
    vps_db_id: i32,
    agent_secret: String,
    id_provider: impl Fn() -> u64 + Send + Sync + Clone + 'static,
    mut term_rx: oneshot::Receiver<()>, // Termination signal receiver
) {
    let child_command_id = request.command_id.clone();
    let command_to_run = request.content;

    // --- Command Pre-flight Checks ---
    if command_to_run.is_empty() {
        send_error_result("Command content was empty.", &child_command_id, &tx_to_server, vps_db_id, &agent_secret, &id_provider).await;
        command_tracker.remove_command(&child_command_id);
        return;
    }
    let parts: Vec<&str> = command_to_run.split_whitespace().collect();
    let program = parts[0];
    let args = &parts[1..];

    // --- Command Spawning ---
    let mut command = TokioCommand::new(OsStr::new(program));
    command.args(args);
    if !request.working_directory.is_empty() {
        command.current_dir(request.working_directory);
    }
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    let mut child_process = match command.spawn() {
        Ok(child) => child,
        Err(e) => {
            let error_msg = format!("Failed to spawn command: {}", e);
            send_error_result(&error_msg, &child_command_id, &tx_to_server, vps_db_id, &agent_secret, &id_provider).await;
            command_tracker.remove_command(&child_command_id);
            return;
        }
    };

    info!(pid = ?child_process.id(), "Spawned command process successfully.");

    let stdout = child_process.stdout.take().expect("Failed to take stdout");
    let stderr = child_process.stderr.take().expect("Failed to take stderr");

    // --- Concurrent I/O and Termination Handling ---
    let final_status_result = tokio::select! {
        // Case 1: The command is terminated by an external signal
        _ = &mut term_rx => {
            warn!("Termination signal received.");
            match child_process.kill().await {
                Ok(_) => {
                    info!("Kill signal sent successfully.");
                    BatchCommandResult {
                        command_id: child_command_id.clone(),
                        status: CommandStatus::Terminated.into(),
                        exit_code: -1, // Convention for terminated process
                        error_message: "Command terminated by user request.".to_string(),
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to send kill signal: {}", e);
                    error!(error = %error_msg);
                    BatchCommandResult {
                        command_id: child_command_id.clone(),
                        status: CommandStatus::Failure.into(),
                        exit_code: -1,
                        error_message: error_msg,
                    }
                }
            }
        }

        // Case 2: The command runs to completion
        result = async {
            let stdout_task = stream_output(stdout, OutputType::Stdout, child_command_id.clone(), tx_to_server.clone(), vps_db_id, agent_secret.clone(), id_provider.clone());
            let stderr_task = stream_output(stderr, OutputType::Stderr, child_command_id.clone(), tx_to_server.clone(), vps_db_id, agent_secret.clone(), id_provider.clone());

            // Wait for both I/O streams to finish, and then for the process to exit.
            tokio::join!(stdout_task, stderr_task);
            child_process.wait().await
        } => {
            match result {
                Ok(status) => {
                    info!(?status, "Command completed.");
                    let final_status_enum = if status.success() { CommandStatus::Success } else { CommandStatus::Failure };
                    BatchCommandResult {
                        command_id: child_command_id.clone(),
                        status: final_status_enum.into(),
                        exit_code: status.code().unwrap_or(-1),
                        error_message: if status.success() { String::new() } else { format!("Exited with status {}", status) },
                    }
                }
                Err(e) => {
                    let error_msg = format!("Failed to wait for command: {}", e);
                    error!(error = %error_msg);
                    BatchCommandResult {
                        command_id: child_command_id.clone(),
                        status: CommandStatus::Failure.into(),
                        exit_code: -1,
                        error_message: error_msg,
                    }
                }
            }
        }
    };

    // --- Final Result Reporting and Cleanup ---
    let client_msg_id = id_provider();
    if tx_to_server.send(MessageToServer {
        client_message_id: client_msg_id,
        payload: Some(ServerPayload::BatchCommandResult(final_status_result)),
        vps_db_id,
        agent_secret,
    }).await.is_err() {
        error!("Failed to send final result.");
    }

    // The command is finished, so remove it from the tracker.
    command_tracker.remove_command(&child_command_id);
    info!("Lifecycle management finished.");
}

/// Helper to stream output from stdout or stderr.
async fn stream_output(
    stream: impl tokio::io::AsyncRead + Unpin,
    stream_type: OutputType,
    command_id: String,
    tx: mpsc::Sender<MessageToServer>,
    vps_db_id: i32,
    agent_secret: String,
    id_provider: impl Fn() -> u64 + Send + Sync + Clone,
) {
    let mut reader = BufReader::new(stream);
    let mut buffer = Vec::new();
    while let Ok(bytes_read) = reader.read_until(b'\n', &mut buffer).await {
        if bytes_read == 0 {
            break;
        }
        let output_msg = BatchCommandOutputStream {
            command_id: command_id.clone(),
            stream_type: stream_type.into(),
            chunk: buffer.clone(),
            timestamp: Utc::now().timestamp_millis(),
        };
        let client_msg_id = id_provider();
        if tx.send(MessageToServer {
            client_message_id: client_msg_id,
            payload: Some(ServerPayload::BatchCommandOutputStream(output_msg)),
            vps_db_id,
            agent_secret: agent_secret.clone(),
        }).await.is_err() {
            error!("Output stream: Failed to send to server.");
            break;
        }
        buffer.clear();
    }
}

/// Helper to send a generic error result.
async fn send_error_result(
    error_message: &str,
    command_id: &str,
    tx: &mpsc::Sender<MessageToServer>,
    vps_db_id: i32,
    agent_secret: &str,
    id_provider: &(impl Fn() -> u64 + Send + Sync + Clone),
) {
    error!("Sending error result for command.");
    let error_result = BatchCommandResult {
        command_id: command_id.to_string(),
        status: CommandStatus::Failure.into(),
        exit_code: -1,
        error_message: error_message.to_string(),
    };
    let client_msg_id = id_provider();
    if tx.send(MessageToServer {
        client_message_id: client_msg_id,
        payload: Some(ServerPayload::BatchCommandResult(error_result)),
        vps_db_id,
        agent_secret: agent_secret.to_string(),
    }).await.is_err() {
        error!("Failed to send error result.");
    }
}

/// This is the handler for termination requests. It's now much simpler.
pub async fn handle_batch_terminate_command(
    request: BatchTerminateCommandRequest,
    tx_to_server: mpsc::Sender<MessageToServer>,
    command_tracker: Arc<RunningCommandsTracker>,
    _original_server_message_id: u64,
    agent_id: String,
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
            error_message: format!("Termination signal sent, but command was already completed or terminated: {}", e),
        };
        let client_msg_id = id_provider();
        if tx_to_server.send(MessageToServer {
            client_message_id: client_msg_id,
            payload: Some(ServerPayload::BatchCommandResult(result_payload)),
            vps_db_id,
            agent_secret,
        }).await.is_err() {
            error!("Failed to send 'already terminated' result.");
        }
    }
}