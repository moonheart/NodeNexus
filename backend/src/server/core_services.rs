use chrono::Utc;
use futures_util::{Sink, SinkExt, Stream};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::agent_service::{
    message_to_agent::Payload as AgentPayload, message_to_server::Payload as ServerPayload,
    AgentConfig, CommandStatus as GrpcCommandStatus, MessageToAgent, MessageToServer,
    OutputType as GrpcOutputType, ServerHandshakeAck,
};
use crate::db::enums::ChildCommandStatus;
use crate::db::services;
use crate::server::agent_state::{AgentSender, AgentState, ConnectedAgents};
use crate::web::models::websocket_models::WsMessage;

// 1. Define the generic AgentStream trait
pub trait AgentStream:
    Stream<Item = Result<MessageToServer, tonic::Status>>
    + Sink<MessageToAgent, Error = tonic::Status>
    + Send
{
}

// 2. Create the new function that takes the generic stream
pub async fn process_agent_stream<S>(
    mut agent_stream: S,
    agent_sender: AgentSender,
    connected_agents_arc: Arc<Mutex<ConnectedAgents>>,
    pool: Arc<DatabaseConnection>,
    ws_data_broadcaster_tx: broadcast::Sender<WsMessage>,
    update_trigger_tx: mpsc::Sender<()>,
    batch_command_manager: Arc<crate::db::services::BatchCommandManager>,
) where
    S: AgentStream,
{
    tokio::pin!(agent_stream);
    let mut agent_sender = Some(agent_sender);
    let mut vps_db_id: Option<i32> = None;
    let mut server_message_id_counter: u64 = 1;
    let mut handshake_completed = false;

    while let Some(result) = agent_stream.next().await {
        match result {
            Ok(msg_to_server) => {
                let vps_db_id_from_msg = msg_to_server.vps_db_id;
                let agent_secret_from_msg = &msg_to_server.agent_secret;
                let mut auth_successful_for_msg = false;
                let mut error_message_for_ack = String::new();

                // Authenticate every message
                match services::get_vps_by_id(&pool, vps_db_id_from_msg).await {
                    Ok(Some(vps_record)) => {
                        if vps_record.agent_secret == *agent_secret_from_msg {
                            auth_successful_for_msg = true;
                            vps_db_id = Some(vps_db_id_from_msg); // Set vps_db_id on first successful auth
                        } else {
                            error_message_for_ack =
                                "Authentication failed: Invalid secret.".to_string();
                            warn!(vps_id = vps_db_id_from_msg, "Authentication failed: Invalid secret.");
                        }
                    }
                    Ok(None) => {
                        error_message_for_ack = format!(
                            "Authentication failed: VPS ID {vps_db_id_from_msg} not found."
                        );
                        warn!(vps_id = vps_db_id_from_msg, "Authentication failed: VPS ID not found.");
                    }
                    Err(e) => {
                        error_message_for_ack =
                            format!("Authentication failed: Database error ({e})");
                        error!(vps_id = vps_db_id_from_msg, error = %e, "Authentication failed: Database error.");
                    }
                }

                if !auth_successful_for_msg {
                    // If it's a handshake attempt, send a NACK and close.
                    if let Some(ServerPayload::AgentHandshake(_)) = &msg_to_server.payload {
                        let ack = ServerHandshakeAck {
                            authentication_successful: false,
                            error_message: error_message_for_ack,
                            initial_config: None,
                            new_agent_secret: String::new(),
                            server_time_unix_ms: Utc::now().timestamp_millis(),
                        };
                        let _ = agent_stream.send(MessageToAgent {
                            server_message_id: server_message_id_counter,
                            payload: Some(AgentPayload::ServerHandshakeAck(ack)),
                        }).await;
                        error!("Handshake authentication failed. Closing stream.");
                        return; // Close connection
                    } else {
                        // For any other message, just ignore it.
                        warn!(vps_id = vps_db_id_from_msg, "Ignoring unauthenticated message.");
                        continue;
                    }
                }

                // --- At this point, the message is authenticated ---

                if let Some(ServerPayload::AgentHandshake(handshake)) = &msg_to_server.payload {
                    info!(vps_id = vps_db_id_from_msg, "Received AgentHandshake.");
                    handshake_completed = true;

                    let tasks = match services::service_monitor_service::get_tasks_for_agent(
                        &pool,
                        vps_db_id_from_msg,
                    )
                    .await
                    {
                        Ok(tasks) => tasks,
                        Err(e) => {
                            error!(error = %e, "Error fetching service monitor tasks for agent.");
                            vec![]
                        }
                    };

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
                        service_monitor_tasks: tasks,
                    };

                    if let Err(e) = services::update_vps_info_on_handshake(
                        &pool,
                        vps_db_id_from_msg,
                        handshake,
                    )
                    .await {
                        error!(error = %e, "Failed to update VPS info on handshake.");
                    } else {
                        if update_trigger_tx.send(()).await.is_err() {
                            error!("Failed to send update trigger after handshake.");
                        }
                    }

                    let agent_state = AgentState {
                        last_seen_ms: Utc::now().timestamp_millis(),
                        config: initial_config.clone(),
                        vps_db_id: vps_db_id_from_msg,
                        sender: agent_sender
                            .take()
                            .expect("AgentSender should be available for the first handshake"),
                    };

                    // Insert the new state, which returns the old state if it existed.
                    let old_state = {
                        let mut agents_guard = connected_agents_arc.lock().await;
                        agents_guard.agents.insert(vps_db_id_from_msg, agent_state)
                    };

                    // If there was an old state, gracefully close its connection.
                    if let Some(mut old_state) = old_state {
                        info!(vps_id = vps_db_id_from_msg, "Replaced stale agent session.");
                        tokio::spawn(async move {
                            if let Err(e) = old_state.sender.close().await {
                                warn!(vps_id = old_state.vps_db_id, error = %e, "Error closing stale agent sender.");
                            }
                        });
                    } else {
                        info!(vps_id = vps_db_id_from_msg, "New agent session registered.");
                    }


                    let ack = ServerHandshakeAck {
                        authentication_successful: true,
                        error_message: String::new(),
                        initial_config: Some(initial_config),
                        new_agent_secret: String::new(),
                        server_time_unix_ms: Utc::now().timestamp_millis(),
                    };
                    if agent_stream.send(MessageToAgent {
                        server_message_id: server_message_id_counter,
                        payload: Some(AgentPayload::ServerHandshakeAck(ack)),
                    }).await.is_err() {
                        error!("Failed to send successful ServerHandshakeAck to agent.");
                    }
                    server_message_id_counter += 1;

                } else if handshake_completed {
                    // Any subsequent message from an authenticated agent updates its liveness timestamp.
                    {
                        let mut agents_guard = connected_agents_arc.lock().await;
                        if let Some(state) = agents_guard.agents.get_mut(&vps_db_id_from_msg) {
                            state.last_seen_ms = Utc::now().timestamp_millis();
                        }
                    }

                    if let Some(payload) = msg_to_server.payload {
                        match payload {
                            ServerPayload::PerformanceBatch(batch) => {
                                if let Err(e) = services::save_performance_snapshot_batch(
                                    &pool,
                                    vps_db_id_from_msg,
                                    &batch,
                                )
                                .await {
                                    error!(error = %e, "Failed to save performance batch.");
                                } else {
                                    if update_trigger_tx.send(()).await.is_err() {
                                        error!("Failed to send update trigger after metrics batch.");
                                    }
                                }
                            }
                            ServerPayload::UpdateConfigResponse(response) => {
                                let status = if response.success { "synced" } else { "failed" };
                                let error_msg = if response.success { None } else { Some(response.error_message.as_str()) };
                                if let Err(e) = services::update_vps_config_status(
                                    &pool,
                                    vps_db_id_from_msg,
                                    status,
                                    error_msg,
                                )
                                .await {
                                    error!(error = %e, "Failed to update config status.");
                                } else {
                                    if update_trigger_tx.send(()).await.is_err() {
                                        error!("Failed to send update trigger after config update.");
                                    }
                                }
                            }
                            ServerPayload::BatchCommandOutputStream(output_stream) => {
                                if let Ok(child_task_id) = Uuid::parse_str(&output_stream.command_id) {
                                    let stream_type = GrpcOutputType::try_from(output_stream.stream_type).unwrap_or(GrpcOutputType::Unspecified);
                                    if let Err(e) = batch_command_manager.record_child_task_output(
                                        child_task_id,
                                        stream_type,
                                        output_stream.chunk.into_bytes(),
                                        Some(output_stream.timestamp),
                                    ).await {
                                        error!(child_task_id = %child_task_id, error = ?e, "Error recording child task output.");
                                    }
                                }
                            }
                            ServerPayload::BatchCommandResult(command_result) => {
                                if let Ok(child_task_id) = Uuid::parse_str(&command_result.command_id) {
                                    let new_status = match GrpcCommandStatus::try_from(command_result.status) {
                                        Ok(GrpcCommandStatus::Success) => ChildCommandStatus::CompletedSuccessfully,
                                        Ok(GrpcCommandStatus::Failure) => ChildCommandStatus::CompletedWithFailure,
                                        Ok(GrpcCommandStatus::Terminated) => ChildCommandStatus::Terminated,
                                        _ => ChildCommandStatus::AgentError,
                                    };
                                    let error_message = if command_result.error_message.is_empty() { None } else { Some(command_result.error_message) };
                                    if let Err(e) = batch_command_manager.update_child_task_status(
                                        child_task_id,
                                        new_status,
                                        Some(command_result.exit_code),
                                        error_message,
                                    ).await {
                                        error!(child_task_id = %child_task_id, error = ?e, "Error updating child task status.");
                                    }
                                }
                            }
                            ServerPayload::ServiceMonitorResult(result) => {
                                if let Err(e) = services::service_monitor_service::record_monitor_result(
                                    &pool,
                                    vps_db_id_from_msg,
                                    &result,
                                )
                                .await
                                    {
                                        error!(monitor_id = result.monitor_id, error = %e, "Failed to record monitor result.");
                                    } else {
                                        let details = services::service_monitor_service::get_monitor_results_by_id(&pool, result.monitor_id, None, None, Some(1)).await;
                                        if let Ok(mut details_vec) = details {
                                            if let Some(detail) = details_vec.pop() {
                                                if ws_data_broadcaster_tx.receiver_count() > 0 {
                                                    let update = crate::web::models::websocket_models::ServiceMonitorUpdate {
                                                        result_details: detail,
                                                        vps_id: vps_db_id_from_msg,
                                                    };
                                                    let message =
                                                        WsMessage::ServiceMonitorResult(update);
                                                    if let Err(e) =
                                                        ws_data_broadcaster_tx.send(message)
                                                    {
                                                        error!(error = %e, "Failed to broadcast service monitor result.");
                                                    }
                                                }
                                            }
                                        }
                                    }
                            }
                            _ => {
                                warn!(client_msg_id = msg_to_server.client_message_id, "Received unhandled message type.");
                            }
                        }
                    }
                } else {
                     warn!(vps_id = vps_db_id_from_msg, "Ignoring message: Handshake not yet completed.");
                }
            }
            Err(status) => {
                error!(?status, "Error receiving message from agent. Stream broken.");
                break;
            }
        }
    }

    // When the stream ends (client disconnects), we don't need to do anything here.
    // If the agent reconnects, the new handshake will replace the state in ConnectedAgents.
    // If the agent does not reconnect, the agent_liveness_check_task will eventually
    // clean up the state and set the VPS status to 'offline'.
    if let Some(id) = vps_db_id {
        info!(vps_id = id, "Agent stream disconnected. Cleanup will be handled by liveness check or next reconnect.");
    } else {
        info!("Unauthenticated agent stream disconnected.");
    }
    info!("Connection task finished.");
}
