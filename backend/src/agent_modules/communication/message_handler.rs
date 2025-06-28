use crate::agent_modules::{
    agent_command_service_impl::{handle_batch_agent_command, handle_batch_terminate_command},
    command_tracker::RunningCommandsTracker,
    config,
    updater,
};
use crate::agent_service::{
    message_to_agent::Payload as AgentPayload,
    message_to_server::Payload as ServerPayload,
    AgentConfig, MessageToAgent, MessageToServer,
};
use std::pin::Pin;
use futures_util::Stream;
use futures_util::StreamExt;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use tonic::Status;
use tracing::{error, info, warn};

pub async fn server_message_handler_loop(
    mut in_stream: Pin<Box<dyn Stream<Item = Result<MessageToAgent, Status>> + Send + Unpin>>,
    tx_to_server: mpsc::Sender<MessageToServer>,
    agent_id: String,
    id_provider: impl Fn() -> u64 + Send + Sync + Clone + 'static,
    vps_db_id: i32,
    agent_secret: String,
    shared_agent_config: Arc<RwLock<AgentConfig>>,
    config_path: String,
    command_tracker: Arc<RunningCommandsTracker>,
    update_lock: Arc<tokio::sync::Mutex<()>>,
) {
    info!("Listening for messages from server...");
    
    while let Some(message_result) = in_stream.next().await {
        match message_result {
            Ok(message_to_agent) => {
                let server_msg_id_clone = message_to_agent.server_message_id.clone();
                let server_msg_id = message_to_agent.server_message_id;

                if let Some(payload) = message_to_agent.payload {
                    match payload {
                        AgentPayload::UpdateConfigRequest(update_req) => {
                            info!(config_version_id = %update_req.config_version_id, "Received new AgentConfig from server.");
                            let mut success = false;
                            let mut error_message = String::new();

                            if let Some(new_config) = update_req.new_config {
                                match config::save_agent_config(&new_config, &config_path) {
                                    Ok(_) => {
                                        let mut config_w = shared_agent_config.write().unwrap();
                                        *config_w = new_config;
                                        success = true;
                                        info!("Successfully updated and saved new config.");
                                    }
                                    Err(e) => {
                                        error_message = format!("Failed to save config file: {}", e);
                                        error!(error = %error_message);
                                    }
                                }
                            } else {
                                error_message = "Received UpdateConfigRequest with no config payload.".to_string();
                                error!(error = %error_message);
                            }

                            let response = crate::agent_service::UpdateConfigResponse {
                                config_version_id: update_req.config_version_id,
                                success,
                                error_message,
                            };

                            let msg_id = id_provider();
                            if let Err(e) = tx_to_server.send(MessageToServer {
                                client_message_id: msg_id,
                                payload: Some(ServerPayload::UpdateConfigResponse(response)),
                                vps_db_id,
                                agent_secret: agent_secret.clone(),
                            }).await {
                                error!(error = %e, "Failed to send config update response.");
                            }
                        }
                        AgentPayload::CommandRequest(cmd_req) => {
                            warn!(request = ?cmd_req, "Received general CommandRequest. This is not currently handled for batch processing.");
                             let error_result = crate::agent_service::CommandResponse {
                                request_id: cmd_req.request_id.clone(),
                                success: false,
                                error_message: "General CommandRequest not implemented in batch context".to_string(),
                                result_payload: None,
                            };
                            let client_msg_id = id_provider();
                            if tx_to_server.send(MessageToServer {
                                client_message_id: client_msg_id,
                                payload: Some(ServerPayload::CommandResponse(error_result)),
                                vps_db_id,
                                agent_secret: agent_secret.clone(),
                            }).await.is_err() {
                                error!("Failed to send error response for unhandled CommandRequest");
                            }
                        }
                        AgentPayload::BatchAgentCommandRequest(batch_cmd_req) => {
                            info!(command_id = %batch_cmd_req.command_id, "Received BatchAgentCommandRequest.");
                            let tx_clone = tx_to_server.clone();
                            let tracker_clone = command_tracker.clone();
                            let agent_id_clone = agent_id.clone();
                            let vps_db_id_clone = vps_db_id;
                            let agent_secret_clone = agent_secret.clone();
                            let id_provider_clone = id_provider.clone();

                            tokio::spawn(async move {
                                handle_batch_agent_command(
                                    batch_cmd_req,
                                    tx_clone,
                                    tracker_clone,
                                    server_msg_id_clone,
                                    agent_id_clone,
                                    vps_db_id_clone,
                                    agent_secret_clone,
                                    id_provider_clone,
                                ).await;
                            });
                        }
                        AgentPayload::BatchTerminateCommandRequest(batch_term_req) => {
                            info!(command_id = %batch_term_req.command_id, "Received BatchTerminateCommandRequest.");
                            let tx_clone = tx_to_server.clone();
                            let tracker_clone = command_tracker.clone();
                            let agent_id_clone = agent_id.clone();
                            let vps_db_id_clone = vps_db_id;
                            let agent_secret_clone = agent_secret.clone();
                            let id_provider_clone = id_provider.clone();

                            tokio::spawn(async move {
                                handle_batch_terminate_command(
                                    batch_term_req,
                                    tx_clone,
                                    tracker_clone,
                                    server_msg_id_clone,
                                    agent_id_clone,
                                    vps_db_id_clone,
                                    agent_secret_clone,
                                    id_provider_clone,
                                ).await;
                            });
                        }
                        AgentPayload::TriggerUpdateCheck(_cmd) => {
                            info!("Received TriggerUpdateCheck command from server. Spawning update task.");
                            let lock_clone = update_lock.clone();
                            tokio::spawn(async move {
                                updater::handle_update_check(lock_clone).await;
                            });
                        }
                        _ => {
                             warn!(?payload, "Received unhandled payload type from server.");
                        }
                    }
                } else {
                    warn!(server_msg_id = server_msg_id, "Received message from server with no payload.");
                }
            }
            Err(status) => {
                error!(?status, "Error receiving message from server. Stream broken.");
                break;
            }
        }
    }
    info!("Server message stream ended.");
}