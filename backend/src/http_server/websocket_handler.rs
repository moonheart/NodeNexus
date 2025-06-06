use axum::{
    debug_handler, extract::{
        ws::{Message, Utf8Bytes, WebSocket, WebSocketUpgrade}, Query, State
    }, response::IntoResponse
};
use futures_util::stream::StreamExt;
use std::sync::Arc;
use serde::Deserialize;
use jsonwebtoken::{decode, DecodingKey, Validation}; // Added for JWT decoding

use crate::http_server::auth_logic::{AuthenticatedUser, Claims, get_jwt_secret}; // Import Claims and get_jwt_secret
use crate::http_server::AppState;
use crate::websocket_models::FullServerListPush;
use crate::http_server::AppError; // For error handling

#[derive(Deserialize, Debug)]
pub struct WebSocketAuthQuery {
    token: Option<String>,
}

// Authenticate WebSocket connection using JWT from query parameter
async fn authenticate_ws_connection(
    _app_state: Arc<AppState>, // app_state might not be needed if not hitting DB here
    token_option: Option<String>,
) -> Result<AuthenticatedUser, AppError> {
    let token_str = token_option.ok_or_else(|| {
        eprintln!("WebSocket Auth: Missing authentication token in query.");
        AppError::Unauthorized("Missing authentication token".to_string())
    })?;

    let jwt_secret = get_jwt_secret();
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
                email: claims.email,
            })
        }
        Err(e) => {
            eprintln!("WebSocket Auth: JWT decoding error: {:?}", e);
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
) -> impl IntoResponse {
    // Authenticate the connection
    let user = match authenticate_ws_connection(app_state.clone(), query.token).await {
        Ok(usr) => usr,
        Err(e) => return e.into_response(), // Return error if authentication fails
    };
    
    println!("User {} (ID: {}) authenticated for WebSocket connection.", user.username, user.id);

    ws.on_upgrade(move |socket| handle_socket(socket, app_state, user))
}

async fn handle_socket(mut socket: WebSocket, app_state: Arc<AppState>, user: AuthenticatedUser) { // Changed parameter type
    println!("WebSocket connection established for user: {} (ID: {})", user.username, user.id);

    // 1. Send initial data snapshot
    let initial_data_arc = {
        let cache_guard = app_state.live_server_data_cache.lock().await;
        let servers_list: Vec<crate::websocket_models::ServerWithDetails> = cache_guard.values().cloned().collect();
        Arc::new(FullServerListPush { servers: servers_list })
    };

    if let Ok(json_data) = serde_json::to_string(&*initial_data_arc) {
        if socket.send(Message::Text(Utf8Bytes::from(json_data))).await.is_err() {
            eprintln!("[User: {}] Error sending initial WebSocket data. Closing connection.", user.username);
            return;
        }
        println!("[User: {}] Sent initial data snapshot.", user.username);
    } else {
        eprintln!("[User: {}] Failed to serialize initial data. Closing connection.", user.username);
        return;
    }

    // 2. Subscribe to broadcast channel for updates
    let mut rx = app_state.ws_data_broadcaster_tx.subscribe();

    // 3. Main loop to listen for updates and client messages
    loop {
        tokio::select! {
            // Receive updates from the broadcast channel
            Ok(data_arc) = rx.recv() => {
                if let Ok(json_data) = serde_json::to_string(&*data_arc) {
                    if socket.send(Message::Text(Utf8Bytes::from(json_data))).await.is_err() {
                        eprintln!("[User: {}] Error sending WebSocket data update. Breaking loop.", user.username);
                        break; // Error sending, client might have disconnected
                    }
                     // println!("[User: {}] Sent data update via broadcast.", user.username);
                } else {
                    eprintln!("[User: {}] Failed to serialize broadcast data.", user.username);
                }
            }
            // Receive messages from the client (e.g., ping, commands)
            Some(Ok(msg)) = socket.next() => {
                match msg {
                    Message::Text(t) => {
                        println!("[User: {}] Received text message: {:?}", user.username, t);
                        if t == "ping" {
                            if socket.send(Message::Text(Utf8Bytes::from("pong"))).await.is_err() {
                                eprintln!("[User: {}] Error sending pong. Breaking loop.", user.username);
                                break;
                            }
                        }
                    }
                    Message::Binary(b) => {
                        println!("[User: {}] Received binary message: {:?}", user.username, b);
                    }
                    Message::Ping(p) => {
                         println!("[User: {}] Received ping, sending pong.", user.username);
                        if socket.send(Message::Pong(p)).await.is_err() {
                             eprintln!("[User: {}] Error sending pong. Breaking loop.", user.username);
                            break;
                        }
                    }
                    Message::Pong(_) => {
                        // println!("[User: {}] Received pong.", user.username);
                        // Usually, you don't need to do anything with pong messages from client
                    }
                    Message::Close(c) => {
                        if let Some(cf) = c {
                            println!("[User: {}] Received close message with code {} and reason `{}`. Closing connection.", user.username, cf.code, cf.reason);
                        } else {
                            println!("[User: {}] Received close message. Closing connection.", user.username);
                        }
                        break;
                    }
                }
            }
            // Client disconnected without sending a close message
            else => {
                println!("[User: {}] Client disconnected. Breaking loop.", user.username);
                break;
            }
        }
    }
    println!("[User: {}] WebSocket connection closed.", user.username);
}