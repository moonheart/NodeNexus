use std::sync::Arc;
use chrono::Utc; // Removed DateTime
use tokio::sync::{mpsc, Mutex};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};
use uuid::Uuid;
use sqlx::PgPool; // Added PgPool

use crate::agent_service::{
    AgentConfig, MessageToAgent, MessageToServer, ServerHandshakeAck, OsType as ProtoOsType, // Added OsType
    message_to_server::Payload as ServerPayload,
    message_to_agent::Payload as AgentPayload,
};
use crate::server::agent_state::{AgentState, ConnectedAgents};
// use crate::db::models::PerformanceMetric; // No longer directly used here
use crate::db::services; // Will be used later for db operations

pub async fn handle_connection(
    mut in_stream: tonic::Streaming<MessageToServer>,
    connected_agents_arc: Arc<Mutex<ConnectedAgents>>,
    pool: Arc<PgPool>, // Added PgPool
) -> Result<Response<ReceiverStream<Result<MessageToAgent, Status>>>, Status> {
    let (tx_to_agent, rx_from_server) = mpsc::channel(128);
    let connection_id = Uuid::new_v4().to_string();
    println!("[{}] New connection stream established", connection_id);

    let connected_agents_arc_clone = connected_agents_arc.clone();
    let pool_clone = pool.clone(); // Clone pool for the spawned task
    tokio::spawn(async move {
        let mut current_session_agent_id: Option<String> = None; // Server-generated session ID
        let mut server_message_id_counter: u64 = 1; // Counter for messages sent from server to agent

        while let Some(result) = in_stream.next().await {
            match result {
                Ok(msg_to_server) => {
                    // Authenticate each message
                    let vps_db_id_from_msg = msg_to_server.vps_db_id;
                    let agent_secret_from_msg = &msg_to_server.agent_secret;
                    let mut auth_successful_for_msg = false;
                    let mut error_message_for_ack = String::new();

                    match services::get_vps_by_id(&pool_clone, vps_db_id_from_msg).await {
                        Ok(Some(vps_record)) => {
                            if vps_record.agent_secret == *agent_secret_from_msg {
                                auth_successful_for_msg = true;
                            } else {
                                error_message_for_ack = "Authentication failed: Invalid secret.".to_string();
                                eprintln!("[{}] Auth failed for VPS ID {}: Invalid secret.", connection_id, vps_db_id_from_msg);
                            }
                        }
                        Ok(None) => {
                            error_message_for_ack = format!("Authentication failed: VPS ID {} not found.", vps_db_id_from_msg);
                            eprintln!("[{}] Auth failed: VPS ID {} not found.", connection_id, vps_db_id_from_msg);
                        }
                        Err(e) => {
                            error_message_for_ack = format!("Authentication failed: Database error ({})", e);
                            eprintln!("[{}] Auth failed for VPS ID {}: DB error: {}", connection_id, vps_db_id_from_msg, e);
                        }
                    }

                    // Handle handshake payload specifically
                    if let Some(ServerPayload::AgentHandshake(handshake)) = &msg_to_server.payload {
                        println!("[{}] Received AgentHandshake from VPS ID {}: {:?}", connection_id, vps_db_id_from_msg, handshake);
                        if auth_successful_for_msg {
                            let assigned_agent_id = Uuid::new_v4().to_string();
                            current_session_agent_id = Some(assigned_agent_id.clone());

                            let initial_config = AgentConfig {
                                metrics_collect_interval_seconds: 1, // TODO: Load from DB or config file per VPS/User
                                metrics_upload_batch_max_size: 50,
                                metrics_upload_interval_seconds: 1,
                                docker_info_collect_interval_seconds: 1,
                                docker_info_upload_interval_seconds: 1,
                                generic_metrics_upload_batch_max_size: 50,
                                generic_metrics_upload_interval_seconds: 1,
                                feature_flags: std::collections::HashMap::new(),
                                log_level: "INFO".to_string(),
                                heartbeat_interval_seconds: 30,
                            };
                            
                            // Convert OsType from i32 to String
                            let os_type_str = ProtoOsType::try_from(handshake.os_type)
                                .map(|os_enum| format!("{:?}", os_enum)) // Converts enum variant to string, e.g., "Linux"
                                .unwrap_or_else(|_| "Unknown".to_string());
                            
                            // Update VPS info in the database
                            // Pass the public_ip_addresses Vec as a slice
                            match services::update_vps_info_on_handshake(
                                &pool_clone,
                                vps_db_id_from_msg,
                                &os_type_str,
                                &handshake.os_name,
                                &handshake.arch,
                                &handshake.hostname,
                                &handshake.public_ip_addresses, // Pass as slice
                            ).await {
                                Ok(rows_affected) => {
                                    if rows_affected > 0 {
                                        println!("[{}] Successfully updated VPS info for VPS ID {}", connection_id, vps_db_id_from_msg);
                                    } else {
                                        eprintln!("[{}] VPS info update on handshake for VPS ID {} affected 0 rows (handler level).", connection_id, vps_db_id_from_msg);
                                    }
                                }
                                Err(e) => {
                                    eprintln!("[{}] Failed to update VPS info for VPS ID {}: {}", connection_id, vps_db_id_from_msg, e);
                                }
                            }
                            
                            let agent_state = AgentState {
                                agent_id: assigned_agent_id.clone(),
                                last_heartbeat_ms: Utc::now().timestamp_millis(),
                                config: initial_config.clone(),
                                vps_db_id: vps_db_id_from_msg,
                            };
                            
                            {
                                let mut agents_guard = connected_agents_arc_clone.lock().await;
                                agents_guard.agents.insert(assigned_agent_id.clone(), agent_state);
                                println!("[{}] Agent {} (VPS DB ID: {}) registered. Total agents: {}",
                                    connection_id, assigned_agent_id, vps_db_id_from_msg, agents_guard.agents.len());
                            }
                            
                            let ack = ServerHandshakeAck {
                                authentication_successful: true,
                                error_message: String::new(),
                                assigned_agent_id,
                                initial_config: Some(initial_config),
                                new_agent_secret: String::new(), // Not changing secret on handshake for now
                                server_time_unix_ms: Utc::now().timestamp_millis(),
                            };
                            if tx_to_agent.send(Ok(MessageToAgent {
                                server_message_id: server_message_id_counter,
                                payload: Some(AgentPayload::ServerHandshakeAck(ack)),
                            })).await.is_err() {
                                eprintln!("[{}] Failed to send successful ServerHandshakeAck to agent for VPS ID {}", connection_id, vps_db_id_from_msg);
                            }
                            server_message_id_counter += 1;
                        } else {
                            // Auth failed for handshake
                            let ack = ServerHandshakeAck {
                                authentication_successful: false,
                                error_message: error_message_for_ack,
                                assigned_agent_id: String::new(),
                                initial_config: None,
                                new_agent_secret: String::new(),
                                server_time_unix_ms: Utc::now().timestamp_millis(),
                            };
                            let _ = tx_to_agent.send(Ok(MessageToAgent {
                                server_message_id: server_message_id_counter,
                                payload: Some(AgentPayload::ServerHandshakeAck(ack)),
                            })).await;
                            eprintln!("[{}] Handshake authentication failed for VPS ID {}. Closing stream.", connection_id, vps_db_id_from_msg);
                            return; // Close stream on failed handshake
                        }
                    } else { // Not a handshake message
                        if !auth_successful_for_msg {
                            eprintln!("[{}] Authentication failed for non-handshake message from VPS ID {}. Ignoring message. ClientMsgID: {}",
                                connection_id, vps_db_id_from_msg, msg_to_server.client_message_id);
                            // Optionally, could close the stream here too if strict auth per message is desired
                            // For now, just ignore the unauthenticated non-handshake message.
                            continue;
                        }

                        // Auth successful for non-handshake message, process payload
                        if let Some(session_id) = &current_session_agent_id {
                            if let Some(payload) = msg_to_server.payload {
                                match payload {
                                    ServerPayload::Heartbeat(heartbeat) => {
                                        println!("[{}] Received Heartbeat from {} (VPS ID {}): client_msg_id={}, ts={}",
                                            connection_id, session_id, vps_db_id_from_msg, msg_to_server.client_message_id, heartbeat.timestamp_unix_ms);
                                        let mut agents_guard = connected_agents_arc_clone.lock().await;
                                        if let Some(state) = agents_guard.agents.get_mut(session_id) {
                                            state.last_heartbeat_ms = Utc::now().timestamp_millis();
                                        } else {
                                            eprintln!("[{}] Received Heartbeat from unknown/deregistered agent_id: {}. Ignoring.", connection_id, session_id);
                                        }
                                    }
                                    ServerPayload::PerformanceBatch(batch) => {
                                        println!("[{}] Received PerformanceBatch from {} (VPS ID {}). Snapshots: {}",
                                            connection_id, session_id, vps_db_id_from_msg, batch.snapshots.len());

                                        match services::save_performance_snapshot_batch(&pool_clone, vps_db_id_from_msg, &batch).await {
                                            Ok(_) => {
                                                println!("[{}] Successfully saved performance batch for agent {} (VPS DB ID {})", connection_id, session_id, vps_db_id_from_msg);
                                            }
                                            Err(e) => {
                                                eprintln!("[{}] Failed to save performance batch for agent {} (VPS DB ID {}): {}", connection_id, session_id, vps_db_id_from_msg, e);
                                            }
                                        }
                                    }
                                    ServerPayload::AgentHandshake(_) => {
                                        // This case should have been handled above. If reached, it's an anomaly.
                                        eprintln!("[{}] Received duplicate AgentHandshake from VPS ID {} after initial handshake. Ignoring.", connection_id, vps_db_id_from_msg);
                                    }
                                    _ => {
                                        println!("[{}] Received unhandled message type from {} (VPS ID {}): client_msg_id={}, payload_type: {:?}",
                                            connection_id, session_id, vps_db_id_from_msg, msg_to_server.client_message_id, payload);
                                    }
                                }
                            } else {
                                 println!("[{}] Received message with no payload from VPS ID {}: client_msg_id={}",
                                    connection_id, vps_db_id_from_msg, msg_to_server.client_message_id);
                            }
                        } else if msg_to_server.payload.is_some() { // Non-handshake message before handshake completed
                             eprintln!("[{}] Received non-handshake message from VPS ID {} before handshake was completed. Ignoring.", connection_id, vps_db_id_from_msg);
                        }
                    }
                }
                Err(status) => {
                    eprintln!("[{}] Error receiving message from agent (session: {:?}): {:?}",
                        connection_id, current_session_agent_id, status);
                    break;
                }
            }
        }

        // 清理
        if let Some(session_id_to_remove) = current_session_agent_id {
            println!("[{}] Stream ended for agent session {}", connection_id, session_id_to_remove);
            {
                let mut agents_guard = connected_agents_arc_clone.lock().await;
                if agents_guard.agents.remove(&session_id_to_remove).is_some() {
                    println!("[{}] Agent session {} deregistered. Total agents: {}", connection_id, session_id_to_remove, agents_guard.agents.len());
                }
            }
        } else {
            println!("[{}] Stream ended for a connection that did not complete handshake.", connection_id);
        }
        println!("[{}] Connection task finished.", connection_id);
    });

    Ok(Response::new(ReceiverStream::from(rx_from_server)))
}