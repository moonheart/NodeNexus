use crate::agent_modules::config::AgentCliConfig;
use crate::agent_modules::utils::collect_public_ip_addresses;
use crate::agent_service::agent_communication_service_client::AgentCommunicationServiceClient;
use crate::agent_service::message_to_server::Payload;
use crate::agent_service::{
    AgentConfig, AgentHandshake, Heartbeat, MessageToAgent, MessageToServer, OsType,
};
use std::error::Error;
use std::sync::atomic::{AtomicU64, Ordering}; // Added AtomicU64 and Ordering
use std::sync::Arc;
use std::time::Duration;
use sysinfo::System;
use tokio::sync::mpsc; // Removed Mutex import as it's no longer needed here
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use uuid::Uuid;

// This function is now part of ConnectionHandler or its id_provider
// pub async fn get_next_client_message_id(counter: &Arc<Mutex<u64>>) -> u64 {
//     let mut num = counter.lock().await;
//     let id = *num;
//     *num += 1;
//     id
// }

pub async fn heartbeat_loop(
    tx_to_server: mpsc::Sender<MessageToServer>,
    agent_config: AgentConfig,
    agent_id: String,
    mut id_provider: impl FnMut() -> u64 + Send + 'static, // Closure to provide message IDs
    vps_db_id: i32,
    agent_secret: String,
) {
    let mut interval_duration = agent_config.heartbeat_interval_seconds;
    if interval_duration == 0 { interval_duration = 30; }
    let mut interval = tokio::time::interval(Duration::from_secs(interval_duration as u64));
    println!("[Agent:{}] Heartbeat task started. Interval: {}s", agent_id, interval_duration);

    loop {
        interval.tick().await;
        let heartbeat_payload = Heartbeat {
            timestamp_unix_ms: chrono::Utc::now().timestamp_millis(),
        };
        let msg_id = id_provider(); // Use the closure
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
    agent_id: String,
    // TODO: Consider how to pass AgentConfig updates if needed by other parts of this loop
    // For now, it only prints received config.
) {
    println!("[Agent:{}] Listening for messages from server...", agent_id);
    while let Some(message_result) = in_stream.next().await {
        match message_result {
            Ok(message_to_agent) => {
                if let Some(payload) = message_to_agent.payload {
                    match payload {
                        crate::agent_service::message_to_agent::Payload::AgentConfig(new_config) => {
                            println!("[Agent:{}] Received new AgentConfig from server: {:?}", agent_id, new_config);
                            // TODO: Implement dynamic config update logic (e.g., via a shared state or channel)
                        }
                        crate::agent_service::message_to_agent::Payload::CommandRequest(cmd_req) => {
                            println!("[Agent:{}] Received CommandRequest: {:?}", agent_id, cmd_req);
                            // TODO: Implement command execution logic
                        }
                        _ => {
                            // Potentially log unhandled payload types if verbose logging is enabled
                            // println!("[Agent:{}] Received unhandled payload type from server: {:?}", agent_id, payload);
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

        let handshake_payload = AgentHandshake {
            agent_id_hint: Uuid::new_v4().to_string(),
            agent_version: env!("CARGO_PKG_VERSION").to_string(),
            os_type: i32::from(os_type_proto),
            os_name: System::name().unwrap_or_else(|| "Unknown".to_string()),
            arch: std::env::consts::ARCH.to_string(),
            hostname: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
            public_ip_addresses,
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