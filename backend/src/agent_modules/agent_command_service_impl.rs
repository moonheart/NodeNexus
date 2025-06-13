use std::ffi::OsStr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::process::Command as TokioCommand;
use tokio::io::{AsyncBufReadExt, BufReader};
use std::process::Stdio;
// use uuid::Uuid; // Not strictly needed if id_provider is used for all client_message_ids
use chrono::Utc;
// use std::time::Duration; // For potential timeouts

use crate::agent_service::{
    MessageToServer,
    BatchAgentCommandRequest,
    BatchCommandOutputStream,
    BatchCommandResult,
    BatchTerminateCommandRequest,
    OutputType,
    CommandStatus,
};
use crate::agent_service::message_to_server::Payload as ServerPayload;
use crate::agent_modules::command_tracker::{RunningCommandsTracker, ChildProcessHandle};

pub async fn handle_batch_agent_command(
    request: BatchAgentCommandRequest,
    tx_to_server: mpsc::Sender<MessageToServer>,
    command_tracker: Arc<RunningCommandsTracker>,
    original_server_message_id: u64,
    agent_id: String,
    vps_db_id: i32,
    agent_secret: String,
    mut id_provider: impl FnMut() -> u64 + Send + Clone + 'static, // Added Clone bound
) {
    let child_command_id = request.command_id.clone();
    let _server_msg_id_context = original_server_message_id;


    println!("[Agent:{}][Command {}] Starting execution.", agent_id, child_command_id);

    let command_to_run = request.content;
    if command_to_run.is_empty() {
        eprintln!("[Agent:{}][Command {}] Command content is empty.", agent_id, child_command_id);
        let error_result = BatchCommandResult {
            command_id: child_command_id.clone(),
            status: CommandStatus::Failure.into(),
            exit_code: -1,
            error_message: "Command content was empty.".to_string(),
        };
        let client_msg_id = id_provider();
        if tx_to_server.send(MessageToServer {
            client_message_id: client_msg_id,
            payload: Some(ServerPayload::BatchCommandResult(error_result)),
            vps_db_id,
            agent_secret: agent_secret.clone(),
        }).await.is_err() {
            eprintln!("[Agent:{}][Command {}] Failed to send error result for empty content.", agent_id, child_command_id);
        }
        return;
    }

    let parts: Vec<&str> = command_to_run.split_whitespace().collect();
    if parts.is_empty() {
        eprintln!("[Agent:{}][Command {}] Command content resulted in no executable parts.", agent_id, child_command_id);
        let error_result = BatchCommandResult {
            command_id: child_command_id.clone(),
            status: CommandStatus::Failure.into(),
            exit_code: -1,
            error_message: "Command content was empty after splitting.".to_string(),
        };
        let client_msg_id = id_provider();
        if tx_to_server.send(MessageToServer {
            client_message_id: client_msg_id,
            payload: Some(ServerPayload::BatchCommandResult(error_result)),
            vps_db_id,
            agent_secret: agent_secret.clone(),
        }).await.is_err() {
             eprintln!("[Agent:{}][Command {}] Failed to send error result for empty parts.", agent_id, child_command_id);
        }
        return;
    }

    // Correctly initialize command with the program name and arguments
    let mut command = TokioCommand::new(OsStr::new(parts[0]));
    if parts.len() > 1 {
        command.args(&parts[1..]);
    }

    // This logic for working_directory appears correct for Option<String>
    if let Some(cwd_str) = Some(request.working_directory) {
        if !cwd_str.is_empty() {
            command.current_dir(cwd_str); // cwd_str is String here
        }
    }
    // TODO: Handle request.environment_variables if necessary

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    match command.spawn() {
        Ok(mut child_process_instance) => {
            let child_pid = child_process_instance.id();
            println!("[Agent:{}][Command {}] Spawned successfully. PID: {:?}", agent_id, child_command_id, child_pid);

            let stdout_opt = child_process_instance.stdout.take();
            let stderr_opt = child_process_instance.stderr.take();

            // Create the handle with the new structure
            let child_handle_for_tracker = ChildProcessHandle { child: Some(child_process_instance), pid: child_pid };

            if command_tracker.add_command(child_command_id.clone(), child_handle_for_tracker).is_err() {
                eprintln!("[Agent:{}][Command {}] Failed to add command to tracker (already exists?).", agent_id, child_command_id);
                let error_result = BatchCommandResult {
                    command_id: child_command_id.clone(),
                    status: CommandStatus::Failure.into(),
                    exit_code: -1,
                    error_message: "Failed to track command internally (duplicate ID?).".to_string(),
                };
                let client_msg_id = id_provider();
                if tx_to_server.send(MessageToServer {
                    client_message_id: client_msg_id,
                    payload: Some(ServerPayload::BatchCommandResult(error_result)),
                    vps_db_id,
                    agent_secret: agent_secret.clone(),
                }).await.is_err() {
                    eprintln!("[Agent:{}][Command {}] Failed to send error result for tracking failure.", agent_id, child_command_id);
                }
                return;
            }

            if stdout_opt.is_none() || stderr_opt.is_none() {
                eprintln!("[Agent:{}][Command {}] Failed to take stdout/stderr from child process.", agent_id, child_command_id);
                 let error_result = BatchCommandResult {
                    command_id: child_command_id.clone(),
                    status: CommandStatus::Failure.into(),
                    exit_code: -1,
                    error_message: "Failed to capture stdout/stderr from process.".to_string(),
                };
                let client_msg_id = id_provider();
                if tx_to_server.send(MessageToServer {
                    client_message_id: client_msg_id,
                    payload: Some(ServerPayload::BatchCommandResult(error_result)),
                    vps_db_id,
                    agent_secret: agent_secret.clone(),
                }).await.is_err() {
                    eprintln!("[Agent:{}][Command {}] Failed to send error for stdout/stderr capture failure.", agent_id, child_command_id);
                }
                command_tracker.remove_command(&child_command_id).await;
                return;
            }

            let stdout = stdout_opt.unwrap();
            let stderr = stderr_opt.unwrap();

            let stdout_tx = tx_to_server.clone();
            let stderr_tx = tx_to_server.clone();
            let stdout_child_id = child_command_id.clone();
            let stderr_child_id = child_command_id.clone();
            let agent_id_stdout = agent_id.clone();
            let agent_id_stderr = agent_id.clone();
            let vps_db_id_stdout = vps_db_id;
            let agent_secret_stdout = agent_secret.clone();
            let vps_db_id_stderr = vps_db_id;
            let agent_secret_stderr = agent_secret.clone();
            let mut id_provider_stdout = id_provider.clone();
            let mut id_provider_stderr = id_provider.clone();


            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout);
                let mut buffer = Vec::new();
                loop {
                    match reader.read_until(b'\n', &mut buffer).await {
                        Ok(0) => break,
                        Ok(_) => {
                            let output_stream_msg = BatchCommandOutputStream {
                                command_id: stdout_child_id.clone(),
                                stream_type: OutputType::Stdout.into(),
                                chunk: buffer.clone(),
                                timestamp: Utc::now().timestamp_millis(),
                            };
                            let client_msg_id = id_provider_stdout();
                            if stdout_tx.send(MessageToServer {
                                client_message_id: client_msg_id,
                                payload: Some(ServerPayload::BatchCommandOutputStream(output_stream_msg)),
                                vps_db_id: vps_db_id_stdout,
                                agent_secret: agent_secret_stdout.clone(),
                            }).await.is_err() {
                                eprintln!("[Agent:{}][Command {}] STDOUT: Failed to send to server.", agent_id_stdout, stdout_child_id);
                                break;
                            }
                            buffer.clear();
                        }
                        Err(e) => {
                            eprintln!("[Agent:{}][Command {}] STDOUT: Error reading: {}", agent_id_stdout, stdout_child_id, e);
                            break;
                        }
                    }
                }
            });

            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr);
                let mut buffer = Vec::new();
                loop {
                    match reader.read_until(b'\n', &mut buffer).await {
                        Ok(0) => break,
                        Ok(_) => {
                            let error_stream_msg = BatchCommandOutputStream {
                                command_id: stderr_child_id.clone(),
                                stream_type: OutputType::Stderr.into(),
                                chunk: buffer.clone(),
                                timestamp: Utc::now().timestamp_millis(),
                            };
                            let client_msg_id = id_provider_stderr();
                            if stderr_tx.send(MessageToServer {
                                client_message_id: client_msg_id,
                                payload: Some(ServerPayload::BatchCommandOutputStream(error_stream_msg)),
                                vps_db_id: vps_db_id_stderr,
                                agent_secret: agent_secret_stderr.clone(),
                            }).await.is_err() {
                                eprintln!("[Agent:{}][Command {}] STDERR: Failed to send to server.", agent_id_stderr, stderr_child_id);
                                break;
                            }
                            buffer.clear();
                        }
                        Err(e) => {
                            eprintln!("[Agent:{}][Command {}] STDERR: Error reading: {}", agent_id_stderr, stderr_child_id, e);
                            break;
                        }
                    }
                }
            });

            if let Some(tracked_handle_arc_for_wait) = command_tracker.get_command_handle(&child_command_id) {
                let final_status_result = {
                    // Take the child process from the handle to release the MutexGuard before .await
                    let mut child_to_wait_on = {
                        let mut guard = tracked_handle_arc_for_wait.lock().unwrap();
                        guard.child.take()
                    }; // MutexGuard is dropped here

                    if let Some(mut actual_child) = child_to_wait_on {
                        match actual_child.wait().await {
                            Ok(status) => {
                                println!("[Agent:{}][Command {}] Completed with status: {}", agent_id, child_command_id, status);
                                let final_status_enum = if status.success() { CommandStatus::Success } else { CommandStatus::Failure };
                                BatchCommandResult {
                                    command_id: child_command_id.clone(),
                                    status: final_status_enum.into(),
                                    exit_code: status.code().unwrap_or(-1),
                                    error_message: if status.success() { String::new() } else { format!("Exited with status {}", status) },
                                }
                            }
                            Err(e) => {
                                eprintln!("[Agent:{}][Command {}] Failed to wait for command: {}", agent_id, child_command_id, e);
                                BatchCommandResult {
                                    command_id: child_command_id.clone(),
                                    status: CommandStatus::Failure.into(),
                                    exit_code: -1,
                                    error_message: format!("Failed to wait for command: {}", e),
                                }
                            }
                        }
                    } else {
                        eprintln!("[Agent:{}][Command {}] Child process was already taken or None in tracker handle before waiting.", agent_id, child_command_id);
                        BatchCommandResult {
                            command_id: child_command_id.clone(),
                            status: CommandStatus::Failure.into(),
                            exit_code: -1,
                            error_message: "Child process handle was unexpectedly empty before waiting.".to_string(),
                        }
                    }
                };

                let client_msg_id = id_provider();
                if tx_to_server.send(MessageToServer {
                    client_message_id: client_msg_id,
                    payload: Some(ServerPayload::BatchCommandResult(final_status_result)),
                    vps_db_id,
                    agent_secret: agent_secret.clone(),
                }).await.is_err() {
                     eprintln!("[Agent:{}][Command {}] Failed to send final result.", agent_id, child_command_id);
                }
            } else {
                 eprintln!("[Agent:{}][Command {}] Could not re-fetch command handle for waiting. This should not happen.", agent_id, child_command_id);
            }

            command_tracker.remove_command(&child_command_id).await;
            println!("[Agent:{}][Command {}] Execution finished and cleaned up from tracker.", agent_id, child_command_id);

        }
        Err(e) => {
            eprintln!("[Agent:{}][Command {}] Failed to spawn command: {}", agent_id, child_command_id, e);
            let error_result = BatchCommandResult {
                command_id: child_command_id.clone(),
                status: CommandStatus::Failure.into(),
                exit_code: -1,
                error_message: format!("Failed to spawn command: {}", e),
            };
            let client_msg_id = id_provider();
            if tx_to_server.send(MessageToServer {
                client_message_id: client_msg_id,
                payload: Some(ServerPayload::BatchCommandResult(error_result)),
                vps_db_id,
                agent_secret: agent_secret.clone(),
            }).await.is_err() {
                eprintln!("[Agent:{}][Command {}] Failed to send error result for spawn failure.", agent_id, child_command_id);
            }
        }
    }
}


pub async fn handle_batch_terminate_command(
    request: BatchTerminateCommandRequest,
    tx_to_server: mpsc::Sender<MessageToServer>,
    command_tracker: Arc<RunningCommandsTracker>,
    original_server_message_id: u64,
    agent_id: String,
    vps_db_id: i32,
    agent_secret: String,
    mut id_provider: impl FnMut() -> u64 + Send + 'static,
) {
    let command_id_to_terminate = request.command_id;
    let _server_msg_id_context = original_server_message_id;

    println!("[Agent:{}][Terminate {}] Attempting termination.", agent_id, command_id_to_terminate);

    let termination_result_payload = match command_tracker.kill_command(&command_id_to_terminate).await {
        Ok(msg) => {
            println!("[Agent:{}][Terminate {}] Kill attempt info: {}", agent_id, command_id_to_terminate, msg);
            BatchCommandResult {
                command_id: command_id_to_terminate.clone(),
                status: CommandStatus::Terminated.into(),
                exit_code: -1,
                error_message: format!("Termination requested by server. Kill signal sent: {}", msg),
            }
        }
        Err(e) => {
            eprintln!("[Agent:{}][Terminate {}] Failed to initiate kill: {}", agent_id, command_id_to_terminate, e);
            BatchCommandResult {
                command_id: command_id_to_terminate.clone(),
                status: CommandStatus::Failure.into(),
                exit_code: -1,
                error_message: format!("Failed to terminate command: {}", e),
            }
        }
    };

    let client_msg_id = id_provider();
    if tx_to_server.send(MessageToServer {
        client_message_id: client_msg_id,
        payload: Some(ServerPayload::BatchCommandResult(termination_result_payload)),
        vps_db_id,
        agent_secret: agent_secret.clone(),
    }).await.is_err() {
        eprintln!("[Agent:{}][Terminate {}] Failed to send termination result.", agent_id, command_id_to_terminate);
    }
}