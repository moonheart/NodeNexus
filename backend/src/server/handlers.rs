use std::sync::Arc;
use chrono::Utc; // Removed DateTime
use tokio::sync::{mpsc, Mutex};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};
use uuid::Uuid;
use sqlx::PgPool; // Added PgPool

use crate::agent_service::{
    AgentConfig, MessageToAgent, MessageToServer, ServerHandshakeAck, // Added PerformanceSnapshot
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
        let mut agent_id_option: Option<String> = None;
        let mut vps_db_id_option: Option<i32> = None; // To store vps.id (integer)
        let server_message_id_counter: u64 = 1;

        // 处理握手消息
        match in_stream.next().await {
            Some(Ok(first_msg)) => {
                if let Some(ServerPayload::AgentHandshake(handshake)) = first_msg.payload {
                    println!("[{}] Received AgentHandshake: {:?}", connection_id, handshake);
                    let auth_successful = !handshake.current_agent_secret.is_empty(); // Basic check, real auth needs DB lookup
                    let mut error_message_str = String::new();

                    // TODO: Implement proper authentication by looking up Vps by handshake.current_agent_secret in DB
                    // let vps_record = crate::db::services::get_vps_by_secret(&pool_clone, &handshake.current_agent_secret).await;
                    // For now, simulate successful auth and a placeholder vps_db_id
                    let simulated_vps_db_id = 1; // Placeholder

                    if auth_successful { // Replace with check on vps_record.is_ok() and vps_record.unwrap().is_some()
                        let assigned_agent_id = Uuid::new_v4().to_string(); // This is the server-generated session ID for the agent
                        agent_id_option = Some(assigned_agent_id.clone());
                        vps_db_id_option = Some(simulated_vps_db_id); // Store the fetched vps_db_id

                        let initial_config = AgentConfig {
                            metrics_collect_interval_seconds: 1,
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
                            // TODO: Add vps_db_id to AgentState struct definition
                            // vps_db_id: simulated_vps_db_id,
                        };

                        {
                            let agents_guard = connected_agents_arc_clone.lock().await;
                            // TODO: Ensure AgentState struct has vps_db_id field
                            // agents_guard.agents.insert(assigned_agent_id.clone(), agent_state);
                            // For now, let's assume AgentState is updated correctly elsewhere or this is a simplified registration
                            println!("[{}] Agent {} (VPS DB ID: {}) registered. Total agents: {}",
                                connection_id, assigned_agent_id, simulated_vps_db_id, agents_guard.agents.len());
                        }
                        // Temporarily insert into connected_agents for the sake of current_agent_id logic below
                        // This part needs to be properly integrated with AgentState modification
                         if true { // Simulate adding to connected_agents for now
                            let mut agents_guard = connected_agents_arc_clone.lock().await;
                            agents_guard.agents.insert(assigned_agent_id.clone(), agent_state);
                         }


                        let msg_payload = AgentPayload::ServerHandshakeAck(ack);
                        if tx_to_agent.send(Ok(MessageToAgent {
                            server_message_id: server_message_id_counter,
                            payload: Some(msg_payload),
                        })).await.is_err() {
                            eprintln!("[{}] Failed to send ServerHandshakeAck to agent {}", connection_id, assigned_agent_id);
                        } // else {
                          //  server_message_id_counter += 1; // server_message_id_counter is not used
                        // }
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
                        let _ = tx_to_agent.send(Ok(MessageToAgent {
                            server_message_id: server_message_id_counter,
                            payload: Some(msg_payload),
                        })).await;
                        return;
                    }
                } else {
                    eprintln!("[{}] First message was not AgentHandshake. Closing stream.", connection_id);
                    return;
                }
            }
            Some(Err(status)) => {
                eprintln!("[{}] Error receiving first message: {:?}. Closing stream.", connection_id, status);
                return;
            }
            None => {
                eprintln!("[{}] Client disconnected before sending any message or stream error.", connection_id);
                return;
            }
        }

        let current_agent_id = match agent_id_option {
            Some(id) => id,
            None => {
                eprintln!("[{}] Handshake failed or was not completed. Terminating stream processing.", connection_id);
                return;
            }
        };

        // 处理后续消息
        while let Some(result) = in_stream.next().await {
            match result {
                Ok(msg_to_server) => {
                    if let Some(payload) = msg_to_server.payload {
                        match payload {
                            ServerPayload::Heartbeat(heartbeat) => {
                                println!("[{}] Received Heartbeat from {}: client_msg_id={}, ts={}",
                                    connection_id, current_agent_id, msg_to_server.client_message_id, heartbeat.timestamp_unix_ms);
                                let mut agents_guard = connected_agents_arc_clone.lock().await;
                                if let Some(state) = agents_guard.agents.get_mut(&current_agent_id) {
                                    state.last_heartbeat_ms = Utc::now().timestamp_millis();
                                    // println!("[{}] Agent {} last_heartbeat updated.", connection_id, current_agent_id);
                                } else {
                                    eprintln!("[{}] Received Heartbeat from unknown/deregistered agent_id: {}. Ignoring.", connection_id, current_agent_id);
                                }
                            }
                            ServerPayload::PerformanceBatch(batch) => {
                                println!("[{}] Received PerformanceBatch from {} (Agent UUID). Snapshots: {}",
                                    connection_id, current_agent_id, batch.snapshots.len());

                                // TODO: Retrieve the actual vps_db_id associated with current_agent_id (UUID)
                                // This should have been stored in AgentState during handshake.
                                let vps_db_id_for_metrics = match vps_db_id_option {
                                     Some(id) => id,
                                     None => {
                                         eprintln!("[{}] Critical: vps_db_id not found for agent {}. Cannot save metrics.", connection_id, current_agent_id);
                                         continue; // Skip this batch
                                     }
                                 };

                                // Call the new batch save function
                                match services::save_performance_snapshot_batch(&pool_clone, vps_db_id_for_metrics, &batch).await {
                                    Ok(_) => {
                                        println!("[{}] Successfully saved performance batch for agent {} (VPS DB ID {})", connection_id, current_agent_id, vps_db_id_for_metrics);
                                    }
                                    Err(e) => {
                                        eprintln!("[{}] Failed to save performance batch for agent {} (VPS DB ID {}): {}", connection_id, current_agent_id, vps_db_id_for_metrics, e);
                                    }
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
                    break;
                }
            }
        }

        // 清理
        println!("[{}] Stream ended for agent {}", connection_id, current_agent_id);
        {
            let mut agents_guard = connected_agents_arc_clone.lock().await;
            if agents_guard.agents.remove(&current_agent_id).is_some() {
                println!("[{}] Agent {} deregistered. Total agents: {}", connection_id, current_agent_id, agents_guard.agents.len());
            }
        }
        println!("[{}] Connection task finished.", connection_id);
    });

    Ok(Response::new(ReceiverStream::from(rx_from_server)))
}