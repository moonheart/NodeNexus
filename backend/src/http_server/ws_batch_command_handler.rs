use axum::{
    extract::{
        ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::http_server::AppState;

// A struct to deserialize the payload part of the WebSocket message for filtering.
#[derive(Deserialize, Debug)]
struct WebSocketMessagePayload {
    batch_command_id: Uuid,
}

// A struct to deserialize the overall WebSocket message.
#[derive(Deserialize, Debug)]
struct WebSocketMessage {
    payload: WebSocketMessagePayload,
}

pub async fn batch_command_ws_handler(
    ws: WebSocketUpgrade,
    State(app_state): State<Arc<AppState>>,
    Path(batch_command_id): Path<Uuid>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_batch_socket(socket, app_state, batch_command_id))
}

async fn handle_batch_socket(
    mut socket: WebSocket,
    app_state: Arc<AppState>,
    batch_command_id: Uuid,
) {
    let mut rx = app_state.result_broadcaster.subscribe();
    println!(
        "[WS Batch] New client connected for batch_command_id: {}",
        batch_command_id
    );

    loop {
        tokio::select! {
            // Receive message from broadcast channel
            Ok(msg) = rx.recv() => {
                // Attempt to deserialize the message to check the batch_command_id
                if let Ok(ws_msg) = serde_json::from_str::<WebSocketMessage>(&msg) {
                    // Check if the message's batch_command_id matches the one for this WebSocket connection
                    if ws_msg.payload.batch_command_id == batch_command_id {
                        // If it matches, send the message to the WebSocket client
                        if socket.send(Message::Text(Utf8Bytes::from(msg.clone()))).await.is_err() {
                            // Client disconnected or error sending
                            println!("[WS Batch] Client for {} disconnected or error sending message.", batch_command_id);
                            break;
                        }
                    }
                    // If the ID does not match, we simply do nothing, effectively filtering the message out.
                } else {
                    // This could happen if a message is broadcast that doesn't fit the expected structure.
                    println!("[WS Batch] Failed to parse broadcast message: {}", msg);
                }
            }
            // Receive message from WebSocket client (optional, for ping/pong or client commands)
            Some(Ok(msg)) = socket.next() => {
                match msg {
                    Message::Text(t) => {
                        println!("[WS Batch] Received text from client: {:?}", t);
                        // Here you could handle client messages, e.g., specific subscriptions if needed
                    }
                    Message::Binary(b) => {
                        println!("[WS Batch] Received binary from client: {:?}", b);
                    }
                    Message::Ping(p) => {
                        println!("[WS Batch] Received ping from client: {:?}", p);
                        if socket.send(Message::Pong(p)).await.is_err() {
                            println!("[WS Batch] Error sending pong to client.");
                            break;
                        }
                    }
                    Message::Pong(p) => {
                        println!("[WS Batch] Received pong from client: {:?}", p);
                    }
                    Message::Close(c) => {
                        if let Some(cf) = c {
                            println!("[WS Batch] Client closed connection: code={}, reason={}", cf.code, cf.reason);
                        } else {
                            println!("[WS Batch] Client closed connection without close frame.");
                        }
                        break;
                    }
                }
            }
            else => {
                // All other arms are closed, client disconnected or error
                println!("[WS Batch] Client disconnected or channel closed for {}.", batch_command_id);
                break;
            }
        }
    }
    println!("[WS Batch] Connection handler for {} finished.", batch_command_id);
}