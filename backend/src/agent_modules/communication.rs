use crate::agent_modules::config::{self, AgentCliConfig};
use crate::agent_modules::utils::collect_public_ip_addresses;
use crate::agent_service::agent_communication_service_client::AgentCommunicationServiceClient;
use crate::agent_service::message_to_server::Payload;
use crate::agent_service::{
    AgentConfig, AgentHandshake, Heartbeat, MessageToAgent, MessageToServer, OsType,
};
use std::error::Error;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use sysinfo::System;
use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use uuid::Uuid;

pub async fn heartbeat_loop(
    tx_to_server: mpsc::Sender<MessageToServer>,
    shared_agent_config: Arc<RwLock<AgentConfig>>,
    agent_id: String,
    mut id_provider: impl FnMut() -> u64 + Send + 'static,
    vps_db_id: i32,
    agent_secret: String,
) {
    loop {
        let interval_duration = {
            let config = shared_agent_config.read().unwrap();
            let seconds = config.heartbeat_interval_seconds;
            if seconds > 0 { seconds } else { 30 }
        };

        println!("[Agent:{}] Heartbeat task tick. Next heartbeat in {}s", agent_id, interval_duration);
        tokio::time::sleep(Duration::from_secs(interval_duration as u64)).await;

        let heartbeat_payload = Heartbeat {
            timestamp_unix_ms: chrono::Utc::now().timestamp_millis(),
        };
        let msg_id = id_provider();
        if let Err(e) = tx_to_server.send(MessageToServer {
            client_message_id: msg_id,
            payload: Some(Payload::Heartbeat(heartbeat_payload)),
            vps_db_id,
            agent_secret: agent_secret.clone(),
        }).await {
            eprintln!("[Agent:{}] Failed to send heartbeat: {}. Exiting heartbeat task.", agent_id, e);
            break;
        }
    }
}

pub async fn server_message_handler_loop(
    mut in_stream: tonic::Streaming<MessageToAgent>,
    tx_to_server: mpsc::Sender<MessageToServer>,
    agent_id: String,
    mut id_provider: impl FnMut() -> u64 + Send + 'static,
    vps_db_id: i32,
    agent_secret: String,
    shared_agent_config: Arc<RwLock<AgentConfig>>,
    config_path: String,
) {
    println!("[Agent:{}] Listening for messages from server...", agent_id);
    while let Some(message_result) = in_stream.next().await {
        match message_result {
            Ok(message_to_agent) => {
                if let Some(payload) = message_to_agent.payload {
                    match payload {
                        crate::agent_service::message_to_agent::Payload::UpdateConfigRequest(update_req) => {
                            println!("[Agent:{}] Received new AgentConfig from server.", agent_id);
                            let mut success = false;
                            let mut error_message = String::new();

                            if let Some(new_config) = update_req.new_config {
                                match config::save_agent_config(&new_config, &config_path) {
                                    Ok(_) => {
                                        // Now update the in-memory shared config
                                        let mut config_w = shared_agent_config.write().unwrap();
                                        *config_w = new_config;
                                        success = true;
                                        println!("[Agent:{}] Successfully updated and saved new config.", agent_id);
                                    }
                                    Err(e) => {
                                        error_message = format!("Failed to save config file: {}", e);
                                        eprintln!("[Agent:{}] {}", agent_id, error_message);
                                    }
                                }
                            } else {
                                error_message = "Received UpdateConfigRequest with no config payload.".to_string();
                                eprintln!("[Agent:{}] {}", agent_id, error_message);
                            }

                            let response = crate::agent_service::UpdateConfigResponse {
                                config_version_id: update_req.config_version_id,
                                success,
                                error_message,
                            };

                            let msg_id = id_provider();
                            if let Err(e) = tx_to_server.send(MessageToServer {
                                client_message_id: msg_id,
                                payload: Some(Payload::UpdateConfigResponse(response)),
                                vps_db_id,
                                agent_secret: agent_secret.clone(),
                            }).await {
                                eprintln!("[Agent:{}] Failed to send config update response: {}", agent_id, e);
                            }
                        }
                        crate::agent_service::message_to_agent::Payload::CommandRequest(cmd_req) => {
                            println!("[Agent:{}] Received CommandRequest: {:?}", agent_id, cmd_req);
                            // TODO: Implement command execution logic
                        }
                        _ => {
                            // Potentially log unhandled payload types if verbose logging is enabled
                            // println!("[Agent:{}] Received unhandled payload type from server.", agent_id);
                        }
                    }
                }
            }
            Err(status) => {
                eprintln!("[Agent:{}] Error receiving message from server: {}. Stream broken.", agent_id, status);
                break; // Exit loop on stream error
            }
        }
    }
    println!("[Agent:{}] Server message stream ended.", agent_id);
}

pub struct ConnectionHandler {
    // client: AgentCommunicationServiceClient<tonic::transport::Channel>, // Client is used once for stream, then not needed
    in_stream: tonic::Streaming<MessageToAgent>,
    tx_to_server: mpsc::Sender<MessageToServer>,
    // rx_for_stream_keepalive is removed as ReceiverStream holds the receiver
    pub assigned_agent_id: String,
    pub initial_agent_config: AgentConfig,
    client_message_id_counter: Arc<AtomicU64>, // Changed Mutex<u64> to AtomicU64
}

impl ConnectionHandler {
    pub async fn connect_and_handshake(
        agent_cli_config: &AgentCliConfig,
        initial_message_id_counter_val: u64,
    ) -> Result<Self, Box<dyn Error>> {
        println!("[Agent] Attempting to connect to server: {}", agent_cli_config.server_address);
        
        // Connect client
        let mut client = AgentCommunicationServiceClient::connect(agent_cli_config.server_address.clone()).await
            .map_err(|e| {
                eprintln!("[Agent] Failed to connect to gRPC endpoint: {}", e);
                e
            })?;
        println!("[Agent] Successfully connected to gRPC endpoint.");

        // Establish stream
        let (tx_to_server, rx_for_stream) = mpsc::channel(128); // Increased buffer size
        let stream_response = client.establish_communication_stream(ReceiverStream::new(rx_for_stream)).await
            .map_err(|e| {
                eprintln!("[Agent] Failed to establish communication stream: {}", e);
                e
            })?;
        let mut in_stream = stream_response.into_inner();
        println!("[Agent] Communication stream established.");

        // Prepare handshake
        let os_type_proto = if cfg!(target_os = "linux") { OsType::Linux }
                          else if cfg!(target_os = "macos") { OsType::Macos }
                          else if cfg!(target_os = "windows") { OsType::Windows }
                          else { OsType::default() }; // Should ideally be OsType::Unknown or similar
        
        let public_ip_addresses = collect_public_ip_addresses();

        // Create a System instance to gather information.
        // Most System::* functions for static info are associated functions and don't need an instance.
        // However, to get CPU details (cpus()) and total_memory/total_swap, an instance is needed.
        let mut sys = System::new(); // Create with RefreshKind::nothing() initially
        // Refresh specific parts needed for static info.
        // CPU list for brand, vendor, frequency.
        sys.refresh_cpu_list(sysinfo::CpuRefreshKind::everything());
        // Memory for total_memory and total_swap.
        sys.refresh_memory_specifics(sysinfo::MemoryRefreshKind::everything());

        // Get static info for the first CPU core, if available
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
            public_ip_addresses,
            kernel_version: System::kernel_version().unwrap_or_else(|| "N/A".to_string()),
            os_version_detail: System::os_version().unwrap_or_else(|| "N/A".to_string()),
            long_os_version: System::long_os_version().unwrap_or_else(|| "N/A".to_string()),
            distribution_id: System::distribution_id(),
            physical_core_count: System::physical_core_count().map(|c| c as u32),
            total_memory_bytes: Some(sys.total_memory()),
            total_swap_bytes: Some(sys.total_swap()),
            cpu_static_info: cpu_static_info_opt, // Use the new optional single CPU info from the first core
        };
        
        let client_message_id_counter = Arc::new(AtomicU64::new(initial_message_id_counter_val)); // Use AtomicU64::new
        // Use fetch_add for the first ID
        let handshake_msg_id = client_message_id_counter.fetch_add(1, Ordering::SeqCst); // Use fetch_add

        // Send handshake
        tx_to_server.send(MessageToServer {
            client_message_id: handshake_msg_id,
            payload: Some(Payload::AgentHandshake(handshake_payload)),
            vps_db_id: agent_cli_config.vps_id,
            agent_secret: agent_cli_config.agent_secret.clone(),
        }).await.map_err(|e| {
            eprintln!("[Agent] Failed to send handshake message: {}", e);
            Box::new(e) as Box<dyn Error>
        })?;
        
        // Await handshake response
        match in_stream.next().await {
            Some(Ok(response_msg)) => {
                if let Some(crate::agent_service::message_to_agent::Payload::ServerHandshakeAck(ack)) = response_msg.payload {
                    if ack.authentication_successful {
                        println!("[Agent:{}] Authenticated successfully. Server assigned Agent ID.", ack.assigned_agent_id);
                        Ok(Self {
                            // client, // Not stored as it's not used beyond stream setup
                            in_stream,
                            tx_to_server,
                            // rx_for_stream_keepalive removed
                            assigned_agent_id: ack.assigned_agent_id,
                            initial_agent_config: ack.initial_config.unwrap_or_default(),
                            client_message_id_counter,
                        })
                    } else {
                        let err_msg = format!("Authentication failed: {}. This is a critical error. Agent will not retry automatically for auth failures.", ack.error_message);
                        eprintln!("[Agent] {}", err_msg);
                        Err(Box::new(std::io::Error::new(std::io::ErrorKind::PermissionDenied, ack.error_message)) as Box<dyn Error>)
                    }
                } else {
                    eprintln!("[Agent] Unexpected first message from server (not HandshakeAck).");
                    Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Unexpected first message from server")) as Box<dyn Error>)
                }
            }
            Some(Err(status)) => {
                eprintln!("[Agent] Error receiving handshake response: {}", status);
                Err(Box::new(status) as Box<dyn Error>)
            }
            None => {
                eprintln!("[Agent] Server closed stream during handshake.");
                Err(Box::new(std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "Server closed stream during handshake")) as Box<dyn Error>)
            }
        }
    }

    // Splits the handler into components needed for spawning tasks.
    pub fn split_for_tasks(self) -> (
        tonic::Streaming<MessageToAgent>,    // in_stream for server_message_handler_loop
        mpsc::Sender<MessageToServer>,        // tx_to_server for metrics and heartbeat loops
        Arc<AtomicU64>,                       // client_message_id_counter for id_provider (Changed type)
        String,                               // assigned_agent_id
        AgentConfig,                          // initial_agent_config
    ) {
        (
            self.in_stream,
            self.tx_to_server,
            self.client_message_id_counter,
            self.assigned_agent_id,
            self.initial_agent_config,
        )
    }

    // Provides a closure for generating message IDs using atomic operations.
    // This closure can be cloned and passed to tasks.
    pub fn get_id_provider_closure(counter: Arc<AtomicU64>) -> impl FnMut() -> u64 + Send + 'static { // Changed parameter type
        move || {
            // Use atomic fetch_add for non-blocking ID generation
            counter.fetch_add(1, Ordering::SeqCst) // Use fetch_add, removed block_on and async block
        }
    }
}