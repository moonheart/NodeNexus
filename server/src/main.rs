use std::collections::HashMap;
use std::sync::Arc;
// use std::time::Duration; // For potential timeouts, not used in current plan directly

use chrono::Utc;
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::StreamExt; // Required for in_stream.next()
use tonic::{transport::Server, Request, Response, Status, Streaming};
use uuid::Uuid;

// Generated protobuf code
pub mod agent_service {
    tonic::include_proto!("agent_service"); // Matches the package name in server.proto
}

use agent_service::{
    agent_communication_service_server::{
        AgentCommunicationService, AgentCommunicationServiceServer,
    },
    // Assuming prost generates map<string, string> as HashMap<String, String>
    // If it generates a specific struct like FeatureFlagMap, adjust accordingly.
    // For AgentConfig.feature_flags, if it's map<string, string>, HashMap is fine.
    message_to_server::Payload as ServerPayload,
    message_to_agent::Payload as AgentPayload,
    AgentConfig, MessageToAgent, MessageToServer, ServerHandshakeAck,
    // OsType, // Not directly used for default config values in this step
};

#[derive(Debug, Clone)]
struct AgentState {
    agent_id: String,
    last_heartbeat_ms: i64,
    config: AgentConfig, // Store the sent config
}

#[derive(Default, Debug)]
struct ConnectedAgents {
    // Key: agent_id (String, UUID)
    agents: HashMap<String, AgentState>,
}

#[derive(Debug)]
pub struct MyAgentCommService {
    connected_agents: Arc<Mutex<ConnectedAgents>>,
}

impl MyAgentCommService {
    pub fn new() -> Self {
        Self {
            connected_agents: Arc::new(Mutex::new(ConnectedAgents::default())),
        }
    }
}

#[tonic::async_trait]
impl AgentCommunicationService for MyAgentCommService {
    type EstablishCommunicationStreamStream =
        ReceiverStream<Result<MessageToAgent, Status>>;

    async fn establish_communication_stream(
        &self,
        request: Request<Streaming<MessageToServer>>,
    ) -> Result<Response<Self::EstablishCommunicationStreamStream>, Status> {
        let mut in_stream = request.into_inner();
        let (tx_to_agent, rx_from_server) = mpsc::channel(128); // Buffer for messages to agent

        let connected_agents_arc = self.connected_agents.clone();
        let connection_id = Uuid::new_v4().to_string(); // For logging this specific stream
        println!("[{}] New connection stream established", connection_id);

        tokio::spawn(async move {
            let mut agent_id_option: Option<String> = None;
            let mut server_message_id_counter: u64 = 1; // Simple counter for server_message_id

            // 1. Expect AgentHandshake as the first message
            match in_stream.next().await {
                Some(Ok(first_msg)) => {
                    if let Some(ServerPayload::AgentHandshake(handshake)) = first_msg.payload {
                        println!("[{}] Received AgentHandshake: {:?}", connection_id, handshake);

                        // Simplified authentication
                        let auth_successful = !handshake.current_agent_secret.is_empty();
                        let mut error_message_str = String::new();

                        if auth_successful {
                            let assigned_agent_id = Uuid::new_v4().to_string();
                            agent_id_option = Some(assigned_agent_id.clone());

                            let initial_config = AgentConfig {
                                metrics_collect_interval_seconds: 1,
                                metrics_upload_batch_max_size: 50,
                                metrics_upload_interval_seconds: 1,
                                docker_info_collect_interval_seconds: 1,
                                docker_info_upload_interval_seconds: 1,
                                generic_metrics_upload_batch_max_size: 50,
                                generic_metrics_upload_interval_seconds: 1,
                                feature_flags: HashMap::new(),
                                log_level: "INFO".to_string(),
                                heartbeat_interval_seconds: 30,
                            };

                            let ack = ServerHandshakeAck {
                                authentication_successful: true,
                                error_message: String::new(),
                                assigned_agent_id: assigned_agent_id.clone(),
                                initial_config: Some(initial_config.clone()),
                                new_agent_secret: String::new(),
                                server_time_unix_ms: Utc::now().timestamp_millis(),
                            };

                            let agent_state = AgentState {
                                agent_id: assigned_agent_id.clone(),
                                last_heartbeat_ms: Utc::now().timestamp_millis(),
                                config: initial_config,
                            };
                            
                            { 
                                let mut agents_guard = connected_agents_arc.lock().await;
                                agents_guard.agents.insert(assigned_agent_id.clone(), agent_state);
                                println!("[{}] Agent {} registered. Total agents: {}", connection_id, assigned_agent_id, agents_guard.agents.len());
                            }

                            let msg_payload = AgentPayload::ServerHandshakeAck(ack);
                            if tx_to_agent.send(Ok(MessageToAgent {
                                server_message_id: server_message_id_counter,
                                payload: Some(msg_payload),
                            })).await.is_err() {
                                eprintln!("[{}] Failed to send ServerHandshakeAck to agent {}", connection_id, assigned_agent_id);
                                // Cleanup is handled at the end of the task
                            } else {
                                server_message_id_counter += 1;
                            }
                        } else {
                            error_message_str = "Authentication failed: Invalid or missing secret.".to_string();
                            eprintln!("[{}] Authentication failed for agent hint: {}", connection_id, handshake.agent_id_hint);
                            let ack = ServerHandshakeAck {
                                authentication_successful: false,
                                error_message: error_message_str.clone(),
                                assigned_agent_id: String::new(),
                                initial_config: None,
                                new_agent_secret: String::new(),
                                server_time_unix_ms: Utc::now().timestamp_millis(),
                            };
                            let msg_payload = AgentPayload::ServerHandshakeAck(ack);
                            // Attempt to send error ack
                            let _ = tx_to_agent.send(Ok(MessageToAgent {
                                server_message_id: server_message_id_counter,
                                payload: Some(msg_payload),
                            })).await;
                            // Close connection by not proceeding further
                            println!("[{}] Handshake failed due to auth. Terminating stream.", connection_id);
                            return; // Exit task
                        }
                    } else {
                        eprintln!("[{}] First message was not AgentHandshake. Closing stream.", connection_id);
                        // Optionally send a generic error if MessageToAgent supports it
                        return; // Exit task
                    }
                }
                Some(Err(status)) => {
                     eprintln!("[{}] Error receiving first message: {:?}. Closing stream.", connection_id, status);
                     // Attempt to send error to client if a generic error message type is available.
                     // For now, just close.
                     return; // Exit task
                }
                None => {
                    eprintln!("[{}] Client disconnected before sending any message or stream error.", connection_id);
                    return; // Exit task
                }
            }

            let current_agent_id = match agent_id_option {
                Some(id) => id,
                None => {
                    // This case should ideally be covered by returns above if handshake fails
                    eprintln!("[{}] Handshake failed or was not completed. Terminating stream processing.", connection_id);
                    return; // Exit task
                }
            };

            // 2. Process subsequent messages (Heartbeat, etc.)
            while let Some(result) = in_stream.next().await {
                match result {
                    Ok(msg_to_server) => {
                        if let Some(payload) = msg_to_server.payload {
                            match payload {
                                ServerPayload::Heartbeat(heartbeat) => {
                                    println!("[{}] Received Heartbeat from {}: client_msg_id={}, ts={}", 
                                        connection_id, current_agent_id, msg_to_server.client_message_id, heartbeat.timestamp_unix_ms);
                                    let mut agents_guard = connected_agents_arc.lock().await;
                                    if let Some(state) = agents_guard.agents.get_mut(&current_agent_id) {
                                        state.last_heartbeat_ms = Utc::now().timestamp_millis(); // Update with server's current time
                                        println!("[{}] Agent {} last_heartbeat updated.", connection_id, current_agent_id);

                                        // Optionally, send a heartbeat back if required by protocol
                                        // let response_heartbeat = Heartbeat { timestamp_unix_ms: Utc::now().timestamp_millis() };
                                        // let msg_payload = AgentPayload::HeartbeatRequest(response_heartbeat); // Assuming HeartbeatRequest is the correct wrapper for server->agent heartbeat
                                        // if tx_to_agent.send(Ok(MessageToAgent {
                                        //     server_message_id: server_message_id_counter,
                                        //     payload: Some(msg_payload),
                                        // })).await.is_err() {
                                        //    eprintln!("[{}] Failed to send Heartbeat response to agent {}", connection_id, current_agent_id);
                                        // } else {
                                        //    server_message_id_counter += 1;
                                        // }

                                    } else {
                                        eprintln!("[{}] Received Heartbeat from unknown/deregistered agent_id: {}. Ignoring.", connection_id, current_agent_id);
                                    }
                                }
                                _ => {
                                    println!("[{}] Received unhandled message type from {}: client_msg_id={}, payload_type: {:?}", 
                                        connection_id, current_agent_id, msg_to_server.client_message_id, payload);
                                }
                            }
                        } else {
                             println!("[{}] Received message with no payload from {}: client_msg_id={}", 
                                connection_id, current_agent_id, msg_to_server.client_message_id);
                        }
                    }
                    Err(status) => {
                        eprintln!("[{}] Error receiving message from agent {}: {:?}", connection_id, current_agent_id, status);
                        // Attempt to send an error message if a general error type is defined
                        // For now, break and cleanup.
                        break; 
                    }
                }
            }

            // Cleanup: Stream ended (client closed, error, or completed normally)
            println!("[{}] Stream ended for agent {}", connection_id, current_agent_id);
            { 
                let mut agents_guard = connected_agents_arc.lock().await;
                if agents_guard.agents.remove(&current_agent_id).is_some() {
                    println!("[{}] Agent {} deregistered. Total agents: {}", connection_id, current_agent_id, agents_guard.agents.len());
                }
            }
            println!("[{}] Connection task finished.", connection_id);
        });

        Ok(Response::new(ReceiverStream::new(rx_from_server)))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup tracing/logging if desired
    // For example:
    // if std::env::var("RUST_LOG").is_err() {
    //     std::env::set_var("RUST_LOG", "info,server=debug"); // Adjust log levels
    // }
    // tracing_subscriber::fmt::init();

    let addr = "[::1]:50051".parse()?;
    let agent_comm_service = MyAgentCommService::new();

    println!("AgentCommunicationService server listening on {}", addr);
    // tracing::info!("AgentCommunicationService server listening on {}", addr);


    Server::builder()
        .add_service(AgentCommunicationServiceServer::new(agent_comm_service))
        .serve(addr)
        .await?;

    Ok(())
}
