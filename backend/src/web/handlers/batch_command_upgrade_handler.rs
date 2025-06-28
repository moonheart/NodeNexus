use axum::{
    extract::{
        ws::{Message, Utf8Bytes, WebSocket},
        State, WebSocketUpgrade,
    },
    response::IntoResponse,
    Extension,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    agent_service::CommandType as GrpcCommandType,
    db::enums::ChildCommandStatus,
    web::{
        models::{
            batch_command_models::CreateBatchCommandRequest, AuthenticatedUser,
        },
        AppState,
    },
};

// The main handler for the WebSocket upgrade request.
pub async fn batch_command_upgrade_handler(
    ws: WebSocketUpgrade,
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> impl IntoResponse {
    info!("Upgrading connection to WebSocket for batch command execution.");
    ws.on_upgrade(move |socket| {
        handle_socket(socket, app_state, authenticated_user)
    })
}

// Handles the WebSocket connection after the upgrade.
async fn handle_socket(
    mut socket: WebSocket,
    app_state: Arc<AppState>,
    authenticated_user: AuthenticatedUser,
) {
    info!("New WebSocket connection established. Waiting for command payload.");
    let user_id = authenticated_user.id;

    // 1. Wait for the first message, which should contain the command payload.
    // This has been moved inside the loop to handle multiple client messages.
    let (batch_command_id, command_manager) = {
        let first_msg = match socket.next().await {
            Some(Ok(msg)) => msg,
            _ => {
                warn!("Client disconnected before sending payload or sent invalid message type.");
                let _ = socket.close().await;
                return;
            }
        };

        let payload = if let Message::Text(text) = first_msg {
            match serde_json::from_str::<CreateBatchCommandRequest>(&text) {
                Ok(payload) => {
                    info!(user_id, "Received command payload via WebSocket.");
                    payload
                }
                Err(e) => {
                    error!("Failed to deserialize command payload: {}", e);
                    let _ = socket.close().await;
                    return;
                }
            }
        } else {
            warn!("First message was not of Text type.");
            let _ = socket.close().await;
            return;
        };

    // 2. Create and dispatch the batch command.
        // 2. Create and dispatch the batch command.
        let command_manager = app_state.batch_command_manager.clone();
        let dispatcher = app_state.command_dispatcher.clone();

        let batch_command_id = match command_manager
            .create_batch_command(user_id, payload.clone())
            .await
        {
            Ok((batch_task_model, child_tasks)) => {
                let batch_id = batch_task_model.batch_command_id;
                info!(%batch_id, "Successfully created batch command task in DB.");

                // Send the created ID back to the client immediately.
                let created_msg = json!({
                    "type": "BATCH_TASK_CREATED",
                    "payload": { "batch_command_id": batch_id }
                });
                if socket.send(Message::Text(Utf8Bytes::from(created_msg.to_string()))).await.is_err() {
                     warn!("Failed to send BATCH_TASK_CREATED message to client.");
                }


                // Asynchronously dispatch commands for each child task
                for child_task in child_tasks {
                    let dispatcher_clone = dispatcher.clone();
                    let command_manager_clone = command_manager.clone();
                    let payload_clone = payload.clone();
                    tokio::spawn(async move {
                        let command_content = payload_clone.command_content.unwrap_or_default();
                        let command_type = if payload_clone.script_id.is_some() {
                            GrpcCommandType::SavedScript
                        } else {
                            GrpcCommandType::AdhocCommand
                        };
                        let working_directory = payload_clone.working_directory;
                        let effective_command_content = if command_type == GrpcCommandType::SavedScript {
                            if command_content.is_empty() && payload_clone.script_id.is_some() {
                                payload_clone.script_id.unwrap_or_default()
                            } else {
                                command_content
                            }
                        } else {
                            command_content
                        };

                        let dispatch_result = dispatcher_clone
                            .dispatch_command_to_agent(
                                child_task.child_command_id,
                                child_task.vps_id,
                                &effective_command_content,
                                command_type,
                                working_directory,
                            )
                            .await;

                        let (new_status, error_message) = if let Err(e) = dispatch_result {
                            error!(child_task_id = %child_task.child_command_id, error = ?e, "Failed to dispatch command.");
                            (ChildCommandStatus::AgentUnreachable, Some(e.to_string()))
                        } else {
                            (ChildCommandStatus::SentToAgent, None)
                        };

                        if let Err(update_err) = command_manager_clone
                            .update_child_task_status(
                                child_task.child_command_id,
                                new_status,
                                None,
                                error_message,
                            )
                            .await
                        {
                            error!(child_task_id = %child_task.child_command_id, error = ?update_err, "Failed to update status after dispatch.");
                        }
                    });
                }
                // Return the ID for the next step
                batch_id
            }
            Err(e) => {
                error!(error = ?e, "Failed to create batch command in DB.");
                // Send an error message back to the client and close the connection.
                let error_msg = json!({
                    "type": "ERROR",
                    "payload": { "message": format!("Failed to create batch command: {}", e) }
                });
                if socket.send(Message::Text(Utf8Bytes::from(error_msg.to_string()))).await.is_err() {
                    warn!("Failed to send error message to client before closing.");
                }
                let _ = socket.close().await;
                return;
            }
        };
        (batch_command_id, command_manager)
    };

    // 3. Now, listen for broadcasted results and forward them to the client.
    // This logic is adapted from the original ws_batch_command_handler.
    let mut rx = app_state.result_broadcaster.subscribe();
    info!(%batch_command_id, "Subscribed to result broadcaster. Forwarding messages to client.");

    // Also need to deserialize the broadcast message to filter it.
    // Define message structures for both incoming and outgoing messages.
    #[derive(Deserialize)]
    struct ClientMessage {
        #[serde(rename = "type")]
        msg_type: String,
    }
    #[derive(Deserialize)]
    struct BroadcastMessagePayload {
        batch_command_id: Uuid,
    }
    #[derive(Deserialize)]
    struct BroadcastMessage {
        payload: BroadcastMessagePayload,
    }

    loop {
        tokio::select! {
            // Receive message from broadcast channel
            Ok(msg) = rx.recv() => {
                if let Ok(ws_msg) = serde_json::from_str::<BroadcastMessage>(&msg) {
                    if ws_msg.payload.batch_command_id == batch_command_id
                        && socket.send(Message::Text(Utf8Bytes::from(msg.clone()))).await.is_err() {
                            warn!(%batch_command_id, "Client disconnected or error sending message.");
                            break;
                        }
                } else {
                    error!(message = %msg, "CRITICAL: Failed to parse broadcast message.");
                }
            }
            // Receive message from WebSocket client (for ping/pong, etc.)
            Some(Ok(msg)) = socket.next() => {
                match msg {
                    Message::Text(text) => {
                        if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                            if client_msg.msg_type == "TERMINATE_TASK" {
                                info!(%batch_command_id, "Received TERMINATE_TASK request from client.");
                                match command_manager.terminate_batch_command(batch_command_id, user_id).await {
                                    Ok(tasks_to_terminate) => {
                                        info!(%batch_command_id, "Successfully marked tasks for termination. Sending signals to agents...");
                                        let dispatcher = app_state.command_dispatcher.clone();
                                        for task in tasks_to_terminate {
                                            let dispatcher_clone = dispatcher.clone();
                                            tokio::spawn(async move {
                                                if let Err(e) = dispatcher_clone.terminate_command_on_agent(task.child_command_id, task.vps_id).await {
                                                    error!(child_command_id = %task.child_command_id, error = ?e, "Failed to dispatch termination signal to agent.");
                                                }
                                            });
                                        }
                                    }
                                    Err(e) => {
                                        error!(%batch_command_id, error = ?e, "Failed to process batch command termination in DB.");
                                        // Optionally, send an error back to the client.
                                    }
                                }
                            }
                        } else {
                            warn!(%batch_command_id, "Received unparsable text message from client.");
                        }
                    }
                    Message::Ping(p) => {
                        if socket.send(Message::Pong(p)).await.is_err() {
                            warn!("Error sending pong to client.");
                            break;
                        }
                    }
                    Message::Close(_) => {
                        info!(%batch_command_id, "Client closed connection.");
                        break;
                    }
                    _ => {
                        // Ignore other message types like Binary, Pong
                    }
                }
            }
            else => {
                info!(%batch_command_id, "Client disconnected or channel closed.");
                break;
            }
        }
    }
    info!(%batch_command_id, "Connection handler finished.");
}