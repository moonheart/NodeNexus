use chrono::Utc;
use futures_util::{Sink, Stream};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio_stream::StreamExt;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::agent_service::{
    AgentConfig, CommandStatus as GrpcCommandStatus, MessageToAgent, MessageToServer,
    OutputType as GrpcOutputType, ServerHandshakeAck, message_to_agent::Payload as AgentPayload,
    message_to_server::Payload as ServerPayload,
};
use crate::db::enums::ChildCommandStatus;
use crate::db::services;
use crate::server::agent_state::{AgentSender, AgentState, ConnectedAgents};
use crate::web::models::websocket_models::WsMessage;
use tokio::sync::broadcast;

// 1. Define the generic AgentStream trait
pub trait AgentStream:
    Stream<Item = Result<MessageToServer, tonic::Status>>
    + Sink<MessageToAgent, Error = tonic::Status>
    + Send
{
}

// 2. Create the new function that takes the generic stream
pub async fn process_agent_stream<S>(
    agent_stream: S,
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
    let mut current_session_agent_id: Option<String> = None;
    let mut vps_db_id: Option<i32> = None;
    let mut server_message_id_counter: u64 = 1;

    while let Some(result) = agent_stream.next().await {
        match result {
            Ok(msg_to_server) => {
                let vps_db_id_from_msg = msg_to_server.vps_db_id;
                vps_db_id = Some(vps_db_id_from_msg);
                let agent_secret_from_msg = &msg_to_server.agent_secret;
                let mut auth_successful_for_msg = false;
                let mut error_message_for_ack = String::new();

                match services::get_vps_by_id(&pool, vps_db_id_from_msg).await {
                    Ok(Some(vps_record)) => {
                        if vps_record.agent_secret == *agent_secret_from_msg {
                            auth_successful_for_msg = true;
                        } else {
                            error_message_for_ack =
                                "Authentication failed: Invalid secret.".to_string();
                            warn!("Authentication failed: Invalid secret.");
                        }
                    }
                    Ok(None) => {
                        error_message_for_ack = format!(
                            "Authentication failed: VPS ID {vps_db_id_from_msg} not found."
                        );
                        warn!("Authentication failed: VPS ID not found.");
                    }
                    Err(e) => {
                        error_message_for_ack =
                            format!("Authentication failed: Database error ({e})");
                        error!(error = %e, "Authentication failed: Database error.");
                    }
                }

                if let Some(ServerPayload::AgentHandshake(handshake)) = &msg_to_server.payload {
                    info!(handshake = ?handshake, "Received AgentHandshake.");
                    if auth_successful_for_msg {
                        let assigned_agent_id = Uuid::new_v4().to_string();
                        current_session_agent_id = Some(assigned_agent_id.clone());

                        let tasks = match services::service_monitor_service::get_tasks_for_agent(
                            &pool,
                            vps_db_id_from_msg,
                        )
                        .await
                        {
                            Ok(tasks) => {
                                info!(
                                    count = tasks.len(),
                                    "Found service monitor tasks for agent."
                                );
                                tasks
                            }
                            Err(e) => {
                                error!(error = %e, "Error fetching service monitor tasks for agent. Defaulting to empty list.");
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
                            heartbeat_interval_seconds: 30,
                            service_monitor_tasks: tasks,
                        };

                        match services::update_vps_info_on_handshake(
                            &pool,
                            vps_db_id_from_msg,
                            handshake,
                        )
                        .await
                        {
                            Ok(rows_affected) => {
                                if rows_affected > 0 {
                                    info!("Successfully updated VPS info. Triggering broadcast.");
                                    if update_trigger_tx.send(()).await.is_err() {
                                        error!("Failed to send update trigger after handshake.");
                                    }
                                } else {
                                    warn!(
                                        "VPS info update on handshake affected 0 rows (handler level)."
                                    );
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
                            sender: agent_sender
                                .take()
                                .expect("AgentSender should be available for the first handshake"),
                        };

                        {
                            let mut agents_guard = connected_agents_arc.lock().await;
                            agents_guard
                                .agents
                                .insert(assigned_agent_id.clone(), agent_state);
                            info!(agent_id = %assigned_agent_id, "New agent session registered.");
                        }

                        let ack = ServerHandshakeAck {
                            authentication_successful: true,
                            error_message: String::new(),
                            assigned_agent_id,
                            initial_config: Some(initial_config),
                            new_agent_secret: String::new(),
                            server_time_unix_ms: Utc::now().timestamp_millis(),
                        };
                        if futures_util::SinkExt::send(
                            &mut agent_stream,
                            MessageToAgent {
                                server_message_id: server_message_id_counter,
                                payload: Some(AgentPayload::ServerHandshakeAck(ack)),
                            },
                        )
                        .await
                        .is_err()
                        {
                            error!("Failed to send successful ServerHandshakeAck to agent.");
                        }
                        server_message_id_counter += 1;
                    } else {
                        let ack = ServerHandshakeAck {
                            authentication_successful: false,
                            error_message: error_message_for_ack,
                            assigned_agent_id: String::new(),
                            initial_config: None,
                            new_agent_secret: String::new(),
                            server_time_unix_ms: Utc::now().timestamp_millis(),
                        };
                        let _ = futures_util::SinkExt::send(
                            &mut agent_stream,
                            MessageToAgent {
                                server_message_id: server_message_id_counter,
                                payload: Some(AgentPayload::ServerHandshakeAck(ack)),
                            },
                        )
                        .await;
                        error!("Handshake authentication failed. Closing stream.");
                        return;
                    }
                } else {
                    if !auth_successful_for_msg {
                        warn!(
                            client_msg_id = msg_to_server.client_message_id,
                            "Authentication failed for non-handshake message. Ignoring."
                        );
                        continue;
                    }

                    if let Some(session_id) = &current_session_agent_id {
                        if let Some(payload) = msg_to_server.payload {
                            match payload {
                                ServerPayload::Heartbeat(heartbeat) => {
                                    debug!(
                                        client_msg_id = msg_to_server.client_message_id,
                                        ts = heartbeat.timestamp_unix_ms,
                                        "Received Heartbeat."
                                    );
                                    let mut agents_guard = connected_agents_arc.lock().await;
                                    if let Some(state) = agents_guard.agents.get_mut(session_id) {
                                        state.last_heartbeat_ms = Utc::now().timestamp_millis();
                                    } else {
                                        warn!(
                                            "Received Heartbeat from unknown/deregistered agent. Ignoring."
                                        );
                                    }
                                }
                                ServerPayload::PerformanceBatch(batch) => {
                                    match services::save_performance_snapshot_batch(
                                        &pool,
                                        vps_db_id_from_msg,
                                        &batch,
                                    )
                                    .await
                                    {
                                        Ok(_) => {
                                            if update_trigger_tx.send(()).await.is_err() {
                                                error!(
                                                    "Failed to send update trigger after metrics batch."
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            error!(error = %e, "Failed to save performance batch.");
                                        }
                                    }
                                }
                                ServerPayload::UpdateConfigResponse(response) => {
                                    info!(
                                        success = response.success,
                                        version_id = response.config_version_id,
                                        "Received UpdateConfigResponse."
                                    );
                                    let status = if response.success { "synced" } else { "failed" };
                                    let error_msg = if response.success {
                                        None
                                    } else {
                                        Some(response.error_message.as_str())
                                    };
                                    match services::update_vps_config_status(
                                        &pool,
                                        vps_db_id_from_msg,
                                        status,
                                        error_msg,
                                    )
                                    .await
                                    {
                                        Ok(_) => {
                                            info!(
                                                "Successfully updated config status. Triggering broadcast."
                                            );
                                            if update_trigger_tx.send(()).await.is_err() {
                                                error!(
                                                    "Failed to send update trigger after config update."
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            error!(error = %e, "Failed to update config status.")
                                        }
                                    }
                                }
                                ServerPayload::BatchCommandOutputStream(output_stream) => {
                                    debug!(command_id = %output_stream.command_id, "Received BatchCommandOutputStream.");
                                    match Uuid::parse_str(&output_stream.command_id) {
                                        Ok(child_task_id) => {
                                            let stream_type =
                                                GrpcOutputType::try_from(output_stream.stream_type)
                                                    .unwrap_or(GrpcOutputType::Unspecified);
                                            if let Err(e) = batch_command_manager
                                                .record_child_task_output(
                                                    child_task_id,
                                                    stream_type,
                                                    output_stream.chunk.into_bytes(),
                                                    Some(output_stream.timestamp),
                                                )
                                                .await
                                            {
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
                                            let new_status = match GrpcCommandStatus::try_from(
                                                command_result.status,
                                            ) {
                                                Ok(GrpcCommandStatus::Success) => {
                                                    ChildCommandStatus::CompletedSuccessfully
                                                }
                                                Ok(GrpcCommandStatus::Failure) => {
                                                    ChildCommandStatus::CompletedWithFailure
                                                }
                                                Ok(GrpcCommandStatus::Terminated) => {
                                                    ChildCommandStatus::Terminated
                                                }
                                                _ => ChildCommandStatus::AgentError,
                                            };
                                            let error_message =
                                                if command_result.error_message.is_empty() {
                                                    None
                                                } else {
                                                    Some(command_result.error_message)
                                                };
                                            if let Err(e) = batch_command_manager
                                                .update_child_task_status(
                                                    child_task_id,
                                                    new_status,
                                                    Some(command_result.exit_code),
                                                    error_message,
                                                )
                                                .await
                                            {
                                                error!(child_task_id = %child_task_id, error = ?e, "Error updating child task status.");
                                            }
                                        }
                                        Err(e) => {
                                            error!(command_id = %command_result.command_id, error = %e, "Failed to parse command_id from BatchCommandResult.");
                                        }
                                    }
                                }
                                ServerPayload::ServiceMonitorResult(result) => {
                                    if let Err(e) =
                                        services::service_monitor_service::record_monitor_result(
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
                                                    let message =
                                                        WsMessage::ServiceMonitorResult(detail);
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
                                    warn!(client_msg_id = msg_to_server.client_message_id, payload_type = ?payload, "Received unhandled message type.");
                                }
                            }
                        } else {
                            debug!(
                                client_msg_id = msg_to_server.client_message_id,
                                "Received message with no payload."
                            );
                        }
                    } else if msg_to_server.payload.is_some() {
                        warn!(
                            "Received non-handshake message before handshake was completed. Ignoring."
                        );
                    }
                }
            }
            Err(status) => {
                error!(
                    ?status,
                    "Error receiving message from agent. Stream broken."
                );
                break;
            }
        }
    }

    if let Some(session_id_to_remove) = current_session_agent_id {
        info!("Stream ended for agent session.");

        {
            let mut agents_guard = connected_agents_arc.lock().await;
            if agents_guard.agents.remove(&session_id_to_remove).is_some() {
                info!(
                    total_agents = agents_guard.agents.len(),
                    "Agent session deregistered."
                );
            }
        }

        if let Some(id) = vps_db_id {
            info!("Setting status to 'offline'.");
            match services::update_vps_status(&pool, id, "offline").await {
                Ok(rows_affected) if rows_affected > 0 => {
                    info!("Successfully set status to 'offline'. Triggering broadcast.");
                    if update_trigger_tx.send(()).await.is_err() {
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
}
