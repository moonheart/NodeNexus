use crate::agent_modules::config::{self, AgentCliConfig};
use crate::agent_modules::utils::collect_public_ip_addresses;
use crate::agent_service::agent_communication_service_client::AgentCommunicationServiceClient;
use crate::agent_service::message_to_server::Payload as ServerPayload; // Renamed for clarity
use crate::agent_service::message_to_agent::Payload as AgentPayload; // Renamed for clarity
use crate::agent_service::{
    AgentConfig, AgentHandshake, Heartbeat, MessageToAgent, MessageToServer, OsType, // Enums used by batch messages
};
use crate::agent_modules::command_tracker::RunningCommandsTracker; // Added
use std::error::Error;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use sysinfo::System;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use uuid::Uuid;
 // For timestamps
use tracing::{info, error, warn, debug};

// Import the refactored helper functions
use crate::agent_modules::agent_command_service_impl::{handle_batch_agent_command, handle_batch_terminate_command};

pub async fn heartbeat_loop(
    tx_to_server: mpsc::Sender<MessageToServer>,
    shared_agent_config: Arc<RwLock<AgentConfig>>,
    agent_id: String,
    id_provider: impl Fn() -> u64 + Send + Sync + 'static,
    vps_db_id: i32,
    agent_secret: String,
) {
    loop {
        let interval_duration = {
            let config = shared_agent_config.read().unwrap();
            let seconds = config.heartbeat_interval_seconds;
            if seconds > 0 { seconds } else { 30 }
        };

        debug!(interval_seconds = interval_duration, "Heartbeat task tick.");
        tokio::time::sleep(Duration::from_secs(interval_duration as u64)).await;

        let heartbeat_payload = Heartbeat {
            timestamp_unix_ms: chrono::Utc::now().timestamp_millis(),
        };
        let msg_id = id_provider();
        if let Err(e) = tx_to_server.send(MessageToServer {
            client_message_id: msg_id,
            payload: Some(ServerPayload::Heartbeat(heartbeat_payload)),
            vps_db_id,
            agent_secret: agent_secret.clone(),
        }).await {
            error!(error = %e, "Failed to send heartbeat. Exiting heartbeat task.");
            break;
        }
    }
}

pub async fn server_message_handler_loop(
    mut in_stream: tonic::Streaming<MessageToAgent>,
    tx_to_server: mpsc::Sender<MessageToServer>,
    agent_id: String,
    id_provider: impl Fn() -> u64 + Send + Sync + Clone + 'static, // Added Clone for spawning tasks
    vps_db_id: i32,
    agent_secret: String,
    shared_agent_config: Arc<RwLock<AgentConfig>>,
    config_path: String,
    command_tracker: Arc<RunningCommandsTracker>,
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
                            let id_provider_clone = id_provider.clone(); // Clone id_provider for the new task

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
                            let id_provider_clone = id_provider.clone(); // Clone id_provider for the new task

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

pub struct ConnectionHandler {
    in_stream: tonic::Streaming<MessageToAgent>,
    tx_to_server: mpsc::Sender<MessageToServer>,
    pub assigned_agent_id: String,
    pub initial_agent_config: AgentConfig,
    client_message_id_counter: Arc<AtomicU64>,
}

impl ConnectionHandler {
    pub async fn connect_and_handshake(
        agent_cli_config: &AgentCliConfig,
        initial_message_id_counter_val: u64,
    ) -> Result<Self, Box<dyn Error>> {
        info!("Attempting to connect to server");
        
        let mut client = AgentCommunicationServiceClient::connect(agent_cli_config.server_address.clone()).await
            .map_err(|e| {
                error!(error = %e, "Failed to connect to gRPC endpoint.");
                e
            })?;
        info!("Successfully connected to gRPC endpoint.");

        let (tx_to_server, rx_for_stream) = mpsc::channel(128);
        let stream_response = client.establish_communication_stream(ReceiverStream::new(rx_for_stream)).await
            .map_err(|e| {
                error!(error = %e, "Failed to establish communication stream.");
                e
            })?;
        let mut in_stream = stream_response.into_inner();
        info!("Communication stream established.");

        let os_type_proto = if cfg!(target_os = "linux") { OsType::Linux }
                          else if cfg!(target_os = "macos") { OsType::Macos }
                          else if cfg!(target_os = "windows") { OsType::Windows }
                          else { OsType::default() }; 
        
        let (public_ips, country_opt) = collect_public_ip_addresses().await;

        let mut sys = System::new(); 
        sys.refresh_cpu_list(sysinfo::CpuRefreshKind::everything());
        sys.refresh_memory_specifics(sysinfo::MemoryRefreshKind::everything());

        let cpu_static_info_opt: Option<crate::agent_service::CpuStaticInfo> = sys.cpus().first().map(|cpu| {
            crate::agent_service::CpuStaticInfo {
                name: cpu.name().to_string(),
                frequency: cpu.frequency(),
                vendor_id: cpu.vendor_id().to_string(),
                brand: cpu.brand().to_string(),
            }
        });

        let handshake_payload = AgentHandshake {
            agent_id_hint: Uuid::new_v4().to_string(),
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
            os_type: i32::from(os_type_proto),
            os_name: System::name().unwrap_or_else(|| "N/A".to_string()),
            arch: System::cpu_arch(),
            hostname: System::host_name().unwrap_or_else(|| "N/A".to_string()),
            public_ip_addresses: public_ips,
            kernel_version: System::kernel_version().unwrap_or_else(|| "N/A".to_string()),
            os_version_detail: System::os_version().unwrap_or_else(|| "N/A".to_string()),
            long_os_version: System::long_os_version().unwrap_or_else(|| "N/A".to_string()),
            distribution_id: System::distribution_id(),
            physical_core_count: System::physical_core_count().map(|c| c as u32),
            total_memory_bytes: Some(sys.total_memory()),
            total_swap_bytes: Some(sys.total_swap()),
            cpu_static_info: cpu_static_info_opt,
            country_code: country_opt,
        };
        
        let client_message_id_counter = Arc::new(AtomicU64::new(initial_message_id_counter_val));
        let handshake_msg_id = client_message_id_counter.fetch_add(1, Ordering::SeqCst);

        tx_to_server.send(MessageToServer {
            client_message_id: handshake_msg_id,
            payload: Some(ServerPayload::AgentHandshake(handshake_payload)),
            vps_db_id: agent_cli_config.vps_id,
            agent_secret: agent_cli_config.agent_secret.clone(),
        }).await.map_err(|e| {
            error!(error = %e, "Failed to send handshake message.");
            Box::new(e) as Box<dyn Error>
        })?;
        
        match in_stream.next().await {
            Some(Ok(response_msg)) => {
                if let Some(AgentPayload::ServerHandshakeAck(ack)) = response_msg.payload {
                    if ack.authentication_successful {
                        info!(agent_id = %ack.assigned_agent_id, "Authenticated successfully. Server assigned Agent ID.");
                        Ok(Self {
                            in_stream,
                            tx_to_server,
                            assigned_agent_id: ack.assigned_agent_id,
                            initial_agent_config: ack.initial_config.unwrap_or_default(),
                            client_message_id_counter,
                        })
                    } else {
                        let err_msg = format!("Authentication failed: {}. This is a critical error. Agent will not retry automatically for auth failures.", ack.error_message);
                        error!(error_message = %err_msg, "Handshake authentication failed.");
                        Err(Box::new(std::io::Error::new(std::io::ErrorKind::PermissionDenied, ack.error_message)) as Box<dyn Error>)
                    }
                } else {
                    error!("Unexpected first message from server (not HandshakeAck).");
                    Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Unexpected first message from server")) as Box<dyn Error>)
                }
            }
            Some(Err(status)) => {
                error!(?status, "Error receiving handshake response.");
                Err(Box::new(status) as Box<dyn Error>)
            }
            None => {
                error!("Server closed stream during handshake.");
                Err(Box::new(std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "Server closed stream during handshake")) as Box<dyn Error>)
            }
        }
    }

    pub fn split_for_tasks(self) -> (
        tonic::Streaming<MessageToAgent>,
        mpsc::Sender<MessageToServer>,
        Arc<AtomicU64>, 
        String,          
        AgentConfig,     
    ) {
        (
            self.in_stream,
            self.tx_to_server,
            self.client_message_id_counter,
            self.assigned_agent_id,
            self.initial_agent_config,
        )
    }

    // This function now needs to ensure the returned closure is Clone
    pub fn get_id_provider_closure(counter: Arc<AtomicU64>) -> impl Fn() -> u64 + Send + Sync + Clone + 'static {
        move || {
            counter.fetch_add(1, Ordering::SeqCst)
        }
    }
}