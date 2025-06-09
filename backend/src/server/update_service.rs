use std::sync::Arc;
use tokio::sync::broadcast;
use sqlx::PgPool;

use crate::db::services;
use crate::server::agent_state::LiveServerDataCache;
use crate::websocket_models::FullServerListPush;

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
    pool: &PgPool,
    cache: &LiveServerDataCache,
    broadcaster: &broadcast::Sender<Arc<FullServerListPush>>,
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
            let servers_list_for_broadcast: Vec<crate::websocket_models::ServerWithDetails> = all_servers;
            let full_list_push = Arc::new(FullServerListPush {
                servers: servers_list_for_broadcast,
            });

            if broadcaster.send(full_list_push).is_err() {
                // This is not a critical error, it just means no web clients are currently listening.
                // println!("No web clients listening, skipping broadcast.");
            } else {
                // println!("Successfully broadcasted full state update.");
            }
        }
        Err(e) => {
            eprintln!("CRITICAL: Failed to fetch full server details for broadcast: {}. Cache and clients may be stale.", e);
        }
    }
}