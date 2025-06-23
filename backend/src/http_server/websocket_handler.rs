use axum::{
    debug_handler, extract::{
        ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade}, Query, State
    }, response::IntoResponse
};
use axum_extra::extract::cookie::CookieJar;
use futures_util::stream::StreamExt;
use std::sync::Arc;
use serde::Deserialize;
use jsonwebtoken::{decode, DecodingKey, Validation}; // Added for JWT decoding
use tracing::{info, error, warn, debug};

use crate::http_server::auth_logic::{AuthenticatedUser, Claims}; // Import Claims
use crate::http_server::AppState;
use crate::websocket_models::{FullServerListPush, WsMessage};
use crate::http_server::AppError; // For error handling

#[derive(Deserialize, Debug)]
pub struct WebSocketAuthQuery {
    token: Option<String>,
}

// Authenticate WebSocket connection using JWT from query parameter
async fn authenticate_ws_connection(
    app_state: Arc<AppState>,
    token_option: Option<String>,
) -> Result<AuthenticatedUser, AppError> {
    let token_str = token_option.ok_or_else(|| {
        warn!("WebSocket Auth: Missing authentication token in query.");
        AppError::Unauthorized("Missing authentication token".to_string())
    })?;

    let jwt_secret = &app_state.config.jwt_secret;
    let decoding_key = DecodingKey::from_secret(jwt_secret.as_ref());
    
    // Use default validation for now. Consider adding audience/issuer checks if needed.
    let validation = Validation::default();

    match decode::<Claims>(&token_str, &decoding_key, &validation) {
        Ok(token_data) => {
            // Token is valid, extract claims
            let claims = token_data.claims;
            // TODO: Optionally, you could re-verify user existence in DB here if strictness is required,
            // but for WebSocket, usually a valid token is sufficient if it hasn't expired.
            Ok(AuthenticatedUser {
                id: claims.user_id,
                username: claims.sub, // Assuming 'sub' is username
            })
        }
        Err(e) => {
            warn!(error = ?e, "WebSocket Auth: JWT decoding error.");
            // Map jsonwebtoken::errors::Error to AppError
            // Consider more specific error mapping based on e.kind()
            match e.kind() {
                jsonwebtoken::errors::ErrorKind::InvalidToken => Err(AppError::Unauthorized("Invalid token".to_string())),
                jsonwebtoken::errors::ErrorKind::InvalidSignature => Err(AppError::Unauthorized("Invalid token signature".to_string())),
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => Err(AppError::Unauthorized("Token has expired".to_string())),
                _ => Err(AppError::Unauthorized(format!("Token validation failed: {}", e))),
            }
        }
    }
}

#[debug_handler]
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(app_state): State<Arc<AppState>>,
    Query(query): Query<WebSocketAuthQuery>, // Get token from query params
    jar: CookieJar, // Get cookies
) -> impl IntoResponse {
    // Try to get token from cookie first, fallback to query param
    let token = jar.get("token").map(|c| c.value().to_string()).or(query.token);
    
    // Authenticate the connection
    let user = match authenticate_ws_connection(app_state.clone(), token).await {
        Ok(usr) => usr,
        Err(e) => return e.into_response(), // Return error if authentication fails
    };
    
    info!(user_id = user.id, username = %user.username, "User authenticated for WebSocket connection.");

    ws.on_upgrade(move |socket| handle_socket(socket, app_state, user))
}

async fn handle_socket(mut socket: WebSocket, app_state: Arc<AppState>, user: AuthenticatedUser) { // Changed parameter type
    info!("WebSocket connection established.");

    // 1. Send initial data snapshot
    let initial_data_message = {
        let cache_guard = app_state.live_server_data_cache.lock().await;
        let servers_list: Vec<crate::websocket_models::ServerWithDetails> = cache_guard.values().cloned().collect();
        WsMessage::FullServerList(FullServerListPush { servers: servers_list })
    };

    if let Ok(json_data) = serde_json::to_string(&initial_data_message) {
        if socket.send(Message::Text(Utf8Bytes::from(json_data))).await.is_err() {
            error!("Error sending initial WebSocket data. Closing connection.");
            return;
        }
        info!("Sent initial data snapshot.");
    } else {
        error!("Failed to serialize initial data. Closing connection.");
        return;
    }

    // 2. Subscribe to broadcast channel for updates
    let mut rx = app_state.ws_data_broadcaster_tx.subscribe();

    // 3. Main loop to listen for updates and client messages
    loop {
        tokio::select! {
            // Receive updates from the broadcast channel
            Ok(ws_message) = rx.recv() => {
                if let Ok(json_data) = serde_json::to_string(&ws_message) {
                    if socket.send(Message::Text(Utf8Bytes::from(json_data))).await.is_err() {
                        warn!("Error sending WebSocket data update. Breaking loop.");
                        break; // Error sending, client might have disconnected
                    }
                     // debug!("Sent data update via broadcast.");
                } else {
                    error!("Failed to serialize broadcast data.");
                }
            }
            // Receive messages from the client (e.g., ping, commands)
            Some(Ok(msg)) = socket.next() => {
                match msg {
                    Message::Text(t) => {
                        debug!(message = ?t, "Received text message.");
                        if t == "ping" {
                            if socket.send(Message::Text(Utf8Bytes::from("pong"))).await.is_err() {
                                warn!("Error sending pong. Breaking loop.");
                                break;
                            }
                        }
                    }
                    Message::Binary(b) => {
                        debug!(bytes_len = b.len(), "Received binary message.");
                    }
                    Message::Ping(p) => {
                         debug!("Received ping, sending pong.");
                        if socket.send(Message::Pong(p)).await.is_err() {
                             warn!("Error sending pong. Breaking loop.");
                            break;
                        }
                    }
                    Message::Pong(_) => {
                        debug!("Received pong.");
                    }
                    Message::Close(c) => {
                        if let Some(cf) = c {
                            info!(code = cf.code, reason = %cf.reason, "Received close message. Closing connection.");
                        } else {
                            info!("Received close message. Closing connection.");
                        }
                        break;
                    }
                }
            }
            // Client disconnected without sending a close message
            else => {
                info!("Client disconnected. Breaking loop.");
                break;
            }
        }
    }
    info!("WebSocket connection closed.");
}


// --- Public WebSocket Handler ---

#[debug_handler]
pub async fn public_websocket_handler(
    ws: WebSocketUpgrade,
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    info!("Public WebSocket connection request.");
    ws.on_upgrade(move |socket| handle_public_socket(socket, app_state))
}

async fn handle_public_socket(mut socket: WebSocket, app_state: Arc<AppState>) {
    info!("Public WebSocket connection established.");

    // 1. Send initial data snapshot (desensitized)
    let initial_data_message = {
        let cache_guard = app_state.live_server_data_cache.lock().await;
        let public_servers_list: Vec<crate::websocket_models::ServerWithDetails> = cache_guard
            .values()
            .map(|s| s.desensitize()) // Use the new desensitize method
            .collect();
        
        WsMessage::FullServerList(FullServerListPush {
            servers: public_servers_list,
        })
    };

    if let Ok(json_data) = serde_json::to_string(&initial_data_message) {
        if socket
            .send(Message::Text(Utf8Bytes::from(json_data)))
            .await
            .is_err()
        {
            error!("Error sending initial public WebSocket data. Closing connection.");
            return;
        }
        info!("Sent initial public data snapshot.");
    } else {
        error!("Failed to serialize initial public data. Closing connection.");
        return;
    }

    // 2. Subscribe to the public broadcast channel.
    let mut rx = app_state.public_ws_data_broadcaster_tx.subscribe();

    // 3. Main loop to listen for updates and client pings
    loop {
        tokio::select! {
            Ok(ws_message) = rx.recv() => {
                // The public channel now sends FullServerList messages, just like the private one.
                // No need to filter by message type, as the public broadcaster is dedicated.
                if let Ok(json_data) = serde_json::to_string(&ws_message) {
                    if socket.send(Message::Text(Utf8Bytes::from(json_data))).await.is_err() {
                        warn!("Error sending public WebSocket data update. Breaking loop.");
                        break;
                    }
                } else {
                    error!("Failed to serialize public broadcast data.");
                }
            }
            Some(Ok(msg)) = socket.next() => {
                match msg {
                    Message::Ping(p) => {
                        if socket.send(Message::Pong(p)).await.is_err() {
                            warn!("Error sending pong on public socket. Breaking loop.");
                            break;
                        }
                    }
                    Message::Close(_) => {
                        info!("Public client sent close message. Closing connection.");
                        break;
                    }
                    _ => {} // Ignore other message types
                }
            }
            else => {
                info!("Public client disconnected. Breaking loop.");
                break;
            }
        }
    }
    info!("Public WebSocket connection closed.");
}