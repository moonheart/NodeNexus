use sea_orm::DatabaseConnection; // Replaced PgPool
use tokio::sync::broadcast;
use tracing::{debug, error};

use crate::db::services;
use crate::server::agent_state::LiveServerDataCache;
use crate::web::models::websocket_models::{FullServerListPush, ServerWithDetails, WsMessage};

/// The centralized function to trigger a full state update and broadcast to all WebSocket clients.
///
/// This function performs the following steps:
/// 1. Fetches the complete, latest details for ALL VPS instances from the database.
///    This ensures that any recent changes are captured.
/// 2. Locks the live server data cache.
/// 3. Clears the existing cache and replaces it entirely with the fresh data from the database.
/// 4. Constructs a `FullServerListPush` message containing the complete, updated list.
/// 5. Broadcasts this message to all connected WebSocket clients.
///
/// This function should be called after ANY event that modifies VPS data to ensure
/// data consistency across the entire system.
pub async fn broadcast_full_state_update(
    pool: &DatabaseConnection, // Changed PgPool to DatabaseConnection
    cache: &LiveServerDataCache,
    broadcaster: &broadcast::Sender<WsMessage>,
) {
    // 1. Fetch the complete, fresh state for all servers from the database.
    match services::get_all_vps_with_details_for_cache(pool).await {
        Ok(all_servers) => {
            // 2. Update the in-memory cache with the fresh, complete list.
            {
                let mut cache_guard = cache.lock().await;
                cache_guard.clear();
                for server in all_servers.iter() {
                    cache_guard.insert(server.basic_info.id, server.clone());
                }
            } // Lock is released here

            // 3. Broadcast the entire updated list to all clients.
            let servers_list_for_broadcast: Vec<
                crate::web::models::websocket_models::ServerWithDetails,
            > = all_servers;
            let full_list_push = FullServerListPush {
                servers: servers_list_for_broadcast,
            };
            let message = WsMessage::FullServerList(full_list_push);

            if broadcaster.receiver_count() > 0 {
                if broadcaster.send(message).is_err() {
                    // This can happen if all subscribers have disconnected between the check and the send.
                    debug!("Broadcast failed: No clients were listening.");
                } else {
                    debug!(
                        clients = broadcaster.receiver_count(),
                        "Successfully broadcasted full state update."
                    );
                }
            } else {
                debug!("No web clients listening, skipping broadcast.");
            }
        }
        Err(e) => {
            error!(error = %e, "CRITICAL: Failed to fetch full server details for broadcast. Cache and clients may be stale.");
        }
    }
}

pub async fn broadcast_full_state_update_to_all(
    pool: &DatabaseConnection,
    cache: &LiveServerDataCache,
    private_broadcaster: &broadcast::Sender<WsMessage>,
    public_broadcaster: &broadcast::Sender<WsMessage>,
) {
    // 1. Fetch the complete, fresh state for all servers from the database.
    match services::get_all_vps_with_details_for_cache(pool).await {
        Ok(all_servers) => {
            // 2. Update the in-memory cache with the fresh, complete list.
            {
                let mut cache_guard = cache.lock().await;
                cache_guard.clear();
                for server in all_servers.iter() {
                    cache_guard.insert(server.basic_info.id, server.clone());
                }
            } // Lock is released here

            // 3. Broadcast to private channel (full data)
            if private_broadcaster.receiver_count() > 0 {
                let full_list_push = FullServerListPush {
                    servers: all_servers.clone(), // Clone for the private broadcast
                };
                let message = WsMessage::FullServerList(full_list_push);
                if private_broadcaster.send(message).is_err() {
                    debug!("Private broadcast failed: No clients were listening.");
                } else {
                    debug!(
                        clients = private_broadcaster.receiver_count(),
                        "Successfully broadcasted full state update to private channel."
                    );
                }
            } else {
                debug!("No private web clients listening, skipping private broadcast.");
            }

            // 4. Broadcast to public channel (desensitized data)
            if public_broadcaster.receiver_count() > 0 {
                let public_servers_list: Vec<ServerWithDetails> = all_servers
                    .iter()
                    .map(|s| s.desensitize()) // Use the new method
                    .collect();

                let public_list_push = FullServerListPush {
                    servers: public_servers_list,
                };
                // Both public and private channels now use the same message type
                let message = WsMessage::FullServerList(public_list_push);

                if public_broadcaster.send(message).is_err() {
                    debug!("Public broadcast failed: No clients were listening.");
                } else {
                    debug!(
                        clients = public_broadcaster.receiver_count(),
                        "Successfully broadcasted desensitized state update to public channel."
                    );
                }
            } else {
                debug!("No public web clients listening, skipping public broadcast.");
            }
        }
        Err(e) => {
            error!(error = %e, "CRITICAL: Failed to fetch full server details for broadcast. Cache and clients may be stale.");
        }
    }
}
