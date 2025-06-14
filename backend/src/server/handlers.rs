use std::sync::Arc;
use chrono::Utc; // Added TimeZone
use tokio::sync::{mpsc, Mutex};
use crate::server::agent_state::LiveServerDataCache; // Added for cache
 // Added for cache update
 // To map from
use tokio_stream::StreamExt;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};
use uuid::Uuid;
use sea_orm::DatabaseConnection; // Replaced PgPool
use tracing::{info, error, warn, debug};

use crate::agent_service::{
    AgentConfig, MessageToAgent, MessageToServer, ServerHandshakeAck, // Added OsType
    message_to_server::Payload as ServerPayload,
    message_to_agent::Payload as AgentPayload,
    CommandStatus as GrpcCommandStatus, OutputType as GrpcOutputType, // For batch command results
};
use crate::db::enums::ChildCommandStatus; // For converting GrpcCommandStatus
use crate::server::agent_state::{AgentState, ConnectedAgents};
// use crate::db::models::PerformanceMetric; // No longer directly used here
use crate::db::services; // Will be used later for db operations
 // For fetching VPS details if not in cache
 
use crate::websocket_models::{FullServerListPush, WsMessage};
use tokio::sync::broadcast;

pub async fn handle_connection(
    mut in_stream: tonic::Streaming<MessageToServer>,
    connected_agents_arc: Arc<Mutex<ConnectedAgents>>,
    pool: Arc<DatabaseConnection>, // Changed PgPool to DatabaseConnection
    live_server_data_cache: LiveServerDataCache,
    ws_data_broadcaster_tx: broadcast::Sender<WsMessage>,
    update_trigger_tx: mpsc::Sender<()>,
    batch_command_manager: Arc<crate::db::services::BatchCommandManager>, // Added BatchCommandManager
) -> Result<Response<ReceiverStream<Result<MessageToAgent, Status>>>, Status> {
    let (tx_to_agent, rx_from_server) = mpsc::channel(128);
    info!("New connection stream established");

    let connected_agents_arc_clone = connected_agents_arc.clone();
    let pool_clone = pool.clone();
    let _cache_clone = live_server_data_cache.clone(); // Renamed, Not used directly for broadcast anymore
    let _broadcaster_clone = ws_data_broadcaster_tx.clone(); // Renamed, Not used directly for broadcast anymore
    let trigger_clone = update_trigger_tx.clone();
    let batch_command_manager_clone = batch_command_manager.clone(); // Clone BatchCommandManager

    tokio::spawn(async move {
        let mut current_session_agent_id: Option<String> = None; // Server-generated session ID
        let mut vps_db_id: Option<i32> = None; // Store the VPS ID for cleanup
        let mut server_message_id_counter: u64 = 1; // Counter for messages sent from server to agent

        while let Some(result) = in_stream.next().await {
            match result {
                Ok(msg_to_server) => {
                    // Authenticate each message
                    let vps_db_id_from_msg = msg_to_server.vps_db_id;
                    vps_db_id = Some(vps_db_id_from_msg); // Store for later use
                    let agent_secret_from_msg = &msg_to_server.agent_secret;
                    let mut auth_successful_for_msg = false;
                    let mut error_message_for_ack = String::new();

                    match services::get_vps_by_id(&*pool_clone, vps_db_id_from_msg).await { // Dereference Arc
                        Ok(Some(vps_record)) => {
                            if vps_record.agent_secret == *agent_secret_from_msg {
                                auth_successful_for_msg = true;
                            } else {
                                error_message_for_ack = "Authentication failed: Invalid secret.".to_string();
                                warn!("Authentication failed: Invalid secret.");
                            }
                        }
                        Ok(None) => {
                            error_message_for_ack = format!("Authentication failed: VPS ID {} not found.", vps_db_id_from_msg);
                            warn!("Authentication failed: VPS ID not found.");
                        }
                        Err(e) => {
                            error_message_for_ack = format!("Authentication failed: Database error ({})", e);
                            error!(error = %e, "Authentication failed: Database error.");
                        }
                    }

                    // Handle handshake payload specifically
                    if let Some(ServerPayload::AgentHandshake(handshake)) = &msg_to_server.payload {
                        info!(handshake = ?handshake, "Received AgentHandshake.");
                        if auth_successful_for_msg {
                            let assigned_agent_id = Uuid::new_v4().to_string();
                            current_session_agent_id = Some(assigned_agent_id.clone());

                            // Fetch service monitoring tasks for this agent
                            let tasks = match services::service_monitor_service::get_tasks_for_agent(&*pool_clone, vps_db_id_from_msg).await {
                                Ok(tasks) => {
                                    info!(count = tasks.len(), "Found service monitor tasks for agent.");
                                    tasks
                                }
                                Err(e) => {
                                    error!(error = %e, "Error fetching service monitor tasks for agent. Defaulting to empty list.");
                                    vec![]
                                }
                            };

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
                                service_monitor_tasks: tasks,
                            };
                            
                            // Convert OsType from i32 to String
                            // This is now handled within update_vps_info_on_handshake
                            // let os_type_str = ProtoOsType::try_from(handshake.os_type)
                            //     .map(|os_enum| format!("{:?}", os_enum))
                            //     .unwrap_or_else(|_| "Unknown".to_string());
                            
                            // Update VPS info in the database
                            // Pass the entire handshake object
                            match services::update_vps_info_on_handshake(
                                &*pool_clone, // Dereference Arc
                                vps_db_id_from_msg,
                                handshake, // Pass the handshake object itself
                            ).await {
                                Ok(rows_affected) => {
                                    if rows_affected > 0 {
                                        info!("Successfully updated VPS info. Triggering broadcast.");
                                        // Trigger broadcast after successful handshake update
                                        if trigger_clone.send(()).await.is_err() {
                                            error!("Failed to send update trigger after handshake.");
                                        }
                                    } else {
                                        warn!("VPS info update on handshake affected 0 rows (handler level).");
                                    }
                                }
                                Err(e) => {
                                    error!(error = %e, "Failed to update VPS info.");
                                }
                            }
                            
                            let agent_state = AgentState {
                                agent_id: assigned_agent_id.clone(),
                                last_heartbeat_ms: Utc::now().timestamp_millis(),
                                config: initial_config.clone(),
                                vps_db_id: vps_db_id_from_msg,
                                sender: tx_to_agent.clone(),
                            };
                            
                            {
                                let mut agents_guard = connected_agents_arc_clone.lock().await;
                                agents_guard.agents.insert(assigned_agent_id.clone(), agent_state);
                                info!(total_agents = agents_guard.agents.len(), "Agent registered.");
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
                                error!("Failed to send successful ServerHandshakeAck to agent.");
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
                            error!("Handshake authentication failed. Closing stream.");
                            return; // Close stream on failed handshake
                        }
                    } else { // Not a handshake message
                        if !auth_successful_for_msg {
                            warn!(client_msg_id = msg_to_server.client_message_id, "Authentication failed for non-handshake message. Ignoring.");
                            // Optionally, could close the stream here too if strict auth per message is desired
                            // For now, just ignore the unauthenticated non-handshake message.
                            continue;
                        }

                        // Auth successful for non-handshake message, process payload
                        if let Some(session_id) = &current_session_agent_id {
                            if let Some(payload) = msg_to_server.payload {
                                match payload {
                                    ServerPayload::Heartbeat(heartbeat) => {
                                        debug!(client_msg_id = msg_to_server.client_message_id, ts = heartbeat.timestamp_unix_ms, "Received Heartbeat.");
                                        let mut agents_guard = connected_agents_arc_clone.lock().await;
                                        if let Some(state) = agents_guard.agents.get_mut(session_id) {
                                            state.last_heartbeat_ms = Utc::now().timestamp_millis();
                                        } else {
                                            warn!("Received Heartbeat from unknown/deregistered agent. Ignoring.");
                                        }
                                    }
                                    ServerPayload::PerformanceBatch(batch) => {
                                        // debug!(snapshot_count = batch.snapshots.len(), "Received PerformanceBatch.");
                                        match services::save_performance_snapshot_batch(&*pool_clone, vps_db_id_from_msg, &batch).await { // Dereference Arc
                                            Ok(_) => {
                                                // After saving metrics, trigger a full broadcast.
                                                // This replaces the silent cache update with a full, consistent state refresh.
                                                if trigger_clone.send(()).await.is_err() {
                                                    error!("Failed to send update trigger after metrics batch.");
                                                }
                                            }
                                            Err(e) => {
                                                error!(error = %e, "Failed to save performance batch.");
                                            }
                                        }
                                    }
                                    ServerPayload::AgentHandshake(_) => {
                                        // This case should have been handled above. If reached, it's an anomaly.
                                        warn!("Received duplicate AgentHandshake after initial handshake. Ignoring.");
                                    }
                                    ServerPayload::UpdateConfigResponse(response) => {
                                       info!(success = response.success, version_id = response.config_version_id, "Received UpdateConfigResponse.");

                                       let status = if response.success { "synced" } else { "failed" };
                                       let error_msg = if response.success { None } else { Some(response.error_message.as_str()) };

                                       match services::update_vps_config_status(&*pool_clone, vps_db_id_from_msg, status, error_msg).await { // Dereference Arc
                                           Ok(_) => {
                                               info!("Successfully updated config status. Triggering broadcast.");
                                               if trigger_clone.send(()).await.is_err() {
                                                   error!("Failed to send update trigger after config update.");
                                               }
                                           }
                                           Err(e) => error!(error = %e, "Failed to update config status."),
                                       }
                                   }
                                   ServerPayload::BatchCommandOutputStream(output_stream) => {
                                       debug!(command_id = %output_stream.command_id, "Received BatchCommandOutputStream.");
                                       match Uuid::parse_str(&output_stream.command_id) {
                                           Ok(child_task_id) => {
                                               let stream_type = GrpcOutputType::try_from(output_stream.stream_type)
                                                   .unwrap_or(GrpcOutputType::Unspecified); // Assuming strip_enum_prefix might be true
                                               if let Err(e) = batch_command_manager_clone.record_child_task_output(
                                                   child_task_id,
                                                   stream_type,
                                                   output_stream.chunk,
                                                   Some(output_stream.timestamp),
                                               ).await {
                                                   error!(child_task_id = %child_task_id, error = ?e, "Error recording child task output.");
                                               }
                                           }
                                           Err(e) => {
                                               error!(command_id = %output_stream.command_id, error = %e, "Failed to parse command_id from BatchCommandOutputStream.");
                                           }
                                       }
                                   }
                                   ServerPayload::BatchCommandResult(command_result) => {
                                       info!(command_id = %command_result.command_id, status = ?command_result.status, exit_code = command_result.exit_code, "Received BatchCommandResult.");
                                       match Uuid::parse_str(&command_result.command_id) {
                                           Ok(child_task_id) => {
                                               let new_status = match GrpcCommandStatus::try_from(command_result.status) {
                                                   Ok(GrpcCommandStatus::Success) => ChildCommandStatus::CompletedSuccessfully,
                                                   Ok(GrpcCommandStatus::Failure) => ChildCommandStatus::CompletedWithFailure,
                                                   Ok(GrpcCommandStatus::Terminated) => ChildCommandStatus::Terminated,
                                                   _ => ChildCommandStatus::AgentError, // Default or map other statuses appropriately
                                               };
                                               let error_message = if command_result.error_message.is_empty() { None } else { Some(command_result.error_message) };

                                               if let Err(e) = batch_command_manager_clone.update_child_task_status(
                                                   child_task_id,
                                                   new_status,
                                                   Some(command_result.exit_code),
                                                   error_message,
                                               ).await {
                                                   error!(child_task_id = %child_task_id, error = ?e, "Error updating child task status.");
                                               }
                                           }
                                           Err(e) => {
                                               error!(command_id = %command_result.command_id, error = %e, "Failed to parse command_id from BatchCommandResult.");
                                           }
                                       }
                                   }
                                   ServerPayload::ServiceMonitorResult(result) => {
                                       if let Err(e) = services::service_monitor_service::record_monitor_result(&*pool_clone, vps_db_id_from_msg, &result).await {
                                           error!(monitor_id = result.monitor_id, error = %e, "Failed to record monitor result.");
                                       } else {
                                           // After successfully recording, fetch the detailed result and broadcast it.
                                           let details = services::service_monitor_service::get_monitor_results_by_id(&*pool_clone, result.monitor_id, None, None, Some(1)).await;
                                           if let Ok(mut details_vec) = details {
                                               if let Some(detail) = details_vec.pop() {
                                                   let message = WsMessage::ServiceMonitorResult(detail);
                                                   if let Err(e) = ws_data_broadcaster_tx.send(message) {
                                                       error!(error = %e, "Failed to broadcast service monitor result.");
                                                   }
                                               }
                                           }
                                       }
                                   }
                                   _ => {
                                       warn!(client_msg_id = msg_to_server.client_message_id, payload_type = ?payload, "Received unhandled message type.");
                                   }
                               }
                          } else {
                                 debug!(client_msg_id = msg_to_server.client_message_id, "Received message with no payload.");
                            }
                        } else if msg_to_server.payload.is_some() { // Non-handshake message before handshake completed
                             warn!("Received non-handshake message before handshake was completed. Ignoring.");
                        }
                    }
                }
                Err(status) => {
                    error!(?status, "Error receiving message from agent. Stream broken.");
                    break;
                }
            }
        }

        // Cleanup logic for when the stream ends (client disconnects)
        if let Some(session_id_to_remove) = current_session_agent_id {
            info!("Stream ended for agent session.");
            
            // Remove agent from the connected list
            {
                let mut agents_guard = connected_agents_arc_clone.lock().await;
                if agents_guard.agents.remove(&session_id_to_remove).is_some() {
                    info!(total_agents = agents_guard.agents.len(), "Agent session deregistered.");
                }
            }

            // Update status to "offline" and broadcast the change
            if let Some(id) = vps_db_id {
                info!("Setting status to 'offline'.");
                match services::update_vps_status(&*pool_clone, id, "offline").await { // Dereference Arc
                    Ok(rows_affected) if rows_affected > 0 => {
                        info!("Successfully set status to 'offline'. Triggering broadcast.");
                        // Trigger a final broadcast to update all clients
                        if trigger_clone.send(()).await.is_err() {
                            error!("Failed to send update trigger on disconnect.");
                        }
                    }
                    Ok(_) => {
                        warn!("Attempted to set status to 'offline', but no rows were affected.");
                    }
                    Err(e) => {
                        error!(error = %e, "Failed to set status to 'offline'.");
                    }
                }
            }
        } else {
            info!("Stream ended for a connection that did not complete handshake.");
        }
        info!("Connection task finished.");
    });

    Ok(Response::new(ReceiverStream::from(rx_from_server)))
}