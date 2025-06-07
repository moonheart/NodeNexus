// 主入口文件
use backend::server::agent_state::{ConnectedAgents, LiveServerDataCache}; // Added LiveServerDataCache
use backend::server::service::MyAgentCommService;
use backend::websocket_models::{FullServerListPush, ServerWithDetails}; // Added ServerWithDetails
use backend::db::services as db_services;
use backend::server::update_service; // Added for cache population
use tonic::transport::Server as TonicServer;
use backend::agent_service::agent_communication_service_server::AgentCommunicationServiceServer;
use backend::http_server;

use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use dotenv::dotenv;
use std::env;
use std::sync::Arc;
use std::collections::HashMap; // For initializing LiveServerDataCache
use tokio::sync::{broadcast, Mutex}; // Added Mutex for LiveServerDataCache and broadcast
use tokio::time::{interval, Duration}; // For the periodic push task
use chrono::Utc;
use tokio::sync::mpsc;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> { // Added Send + Sync for tokio::spawn
    dotenv().ok(); // Load .env file

    // --- Debounce Update Trigger Channel ---
    let (update_trigger_tx, mut update_trigger_rx) = mpsc::channel::<()>(100);


    // --- Database Pool Setup ---
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env file");
    let db_pool = PgPoolOptions::new()
        .max_connections(10) // Adjust as needed
        .connect(&database_url)
        .await
        .expect("Failed to create database pool.");
    
    // --- gRPC Server Setup ---
    let grpc_addr: SocketAddr = "0.0.0.0:50051".parse()?;
    let connected_agents = ConnectedAgents::new();

    // --- Shared State Initialization for WebSocket and gRPC ---
    // Initialize the broadcast channel for WebSocket updates
    let (ws_data_broadcaster_tx, _) = broadcast::channel::<Arc<FullServerListPush>>(100); // Capacity can be configured

    // Initialize the live server data cache
    let initial_cache_data_result = db_services::get_all_vps_with_details_for_cache(&db_pool).await;
    let initial_cache_map: HashMap<i32, ServerWithDetails> = match initial_cache_data_result {
        Ok(servers) => {
            println!("Successfully fetched {} servers for initial cache.", servers.len());
            servers.into_iter().map(|s| (s.basic_info.id, s)).collect()
        }
        Err(e) => {
            eprintln!("Failed to fetch initial server data for cache: {}. Initializing with empty cache.", e);
            HashMap::new()
        }
    };
    let live_server_data_cache: LiveServerDataCache = Arc::new(Mutex::new(initial_cache_map));

    // --- gRPC Server Setup (continued) ---
    let agent_comm_service = MyAgentCommService::new(
        connected_agents.clone(),
        Arc::from(db_pool.clone()),
        live_server_data_cache.clone(), // Pass cache to gRPC service
        ws_data_broadcaster_tx.clone(), // Pass broadcaster to gRPC service
        update_trigger_tx.clone(),      // Pass update trigger to gRPC service
    );
    
    let grpc_service = AgentCommunicationServiceServer::new(agent_comm_service);
    let grpc_server_future = TonicServer::builder()
        .add_service(grpc_service)
        .serve(grpc_addr);

    println!("gRPC AgentCommunicationService listening on {}", grpc_addr);
    
    // --- Agent Heartbeat Check Task ---
    let connected_agents_for_check = connected_agents.clone();
    let pool_for_check = Arc::new(db_pool.clone());
    let trigger_for_check = update_trigger_tx.clone();

    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(60)); // Check every 60 seconds
        println!("Agent heartbeat check task started.");

        loop {
            interval.tick().await;
            let mut disconnected_vps_ids = Vec::new();
            let mut agents_guard = connected_agents_for_check.lock().await;

            // Use retain to iterate and remove in-place
            agents_guard.agents.retain(|_agent_id, state| {
                let is_alive = (Utc::now().timestamp_millis() - state.last_heartbeat_ms) < 90_000; // 90-second threshold
                if !is_alive {
                    println!("Agent for VPS ID {} is considered disconnected.", state.vps_db_id);
                    disconnected_vps_ids.push(state.vps_db_id);
                }
                is_alive
            });

            drop(agents_guard); // Release the lock before async operations

            if !disconnected_vps_ids.is_empty() {
                println!("Found {} disconnected agents. Updating status to 'offline'.", disconnected_vps_ids.len());
                let mut needs_broadcast = false;
                for vps_id in disconnected_vps_ids {
                    match db_services::update_vps_status(&pool_for_check, vps_id, "offline").await {
                        Ok(rows_affected) if rows_affected > 0 => {
                            needs_broadcast = true;
                        }
                        Ok(_) => {} // No rows affected, maybe already offline
                        Err(e) => {
                            eprintln!("Failed to update status to 'offline' for VPS ID {}: {}", vps_id, e);
                        }
                    }
                }

                if needs_broadcast {
                    println!("Triggering broadcast after updating offline status.");
                    if trigger_for_check.send(()).await.is_err() {
                        eprintln!("Failed to send update trigger from heartbeat check task.");
                    }
                }
            }
        }
    });

    // --- Axum HTTP Server Setup ---
    let http_addr: SocketAddr = "0.0.0.0:8080".parse()?;
    // Pass db_pool, cache, and broadcaster_tx to the http_server run function
    let http_server_future = http_server::run_http_server(
        db_pool.clone(),
        http_addr,
        live_server_data_cache.clone(), // Pass cache to HTTP service
        ws_data_broadcaster_tx.clone(), // Pass broadcaster to HTTP service (for AppState)
        connected_agents.clone(),       // Pass connected agents state to HTTP service
        update_trigger_tx.clone(),      // Pass update trigger to HTTP service
    );

    // --- Debounced Broadcast Task ---
    let pool_for_debounce = db_pool.clone();
    let cache_for_debounce = live_server_data_cache.clone();
    let broadcaster_for_debounce = ws_data_broadcaster_tx.clone();
    let debouncer_task = tokio::spawn(async move {
        use tokio::time::{sleep, Duration};
        const DEBOUNCE_DURATION: Duration = Duration::from_millis(500);

        loop {
            // Wait for the first trigger to start a debounce window.
            if update_trigger_rx.recv().await.is_none() {
                // Channel has been closed, the task can exit.
                break;
            }

            // After receiving the first signal, sleep for the debounce duration.
            // This creates a "quiet window" where subsequent signals are ignored.
            sleep(DEBOUNCE_DURATION).await;

            // After the quiet window, drain all other signals that have queued up.
            while let Ok(_) = update_trigger_rx.try_recv() {
                // Discard additional signals.
            }

            // Now that the stream of updates has settled, perform the actual broadcast.
            println!("Debounce window finished. Triggering broadcast.");
            update_service::broadcast_full_state_update(
                &pool_for_debounce,
                &cache_for_debounce,
                &broadcaster_for_debounce,
            )
            .await;
        }
    });

    // --- Run all servers and tasks concurrently ---
    let grpc_handle = tokio::spawn(grpc_server_future);
    let http_handle = tokio::spawn(http_server_future);

    // Use `try_join!` to await the main server tasks. The debouncer runs in the background.
    let (grpc_res, http_res) = tokio::try_join!(grpc_handle, http_handle)?;

    if let Err(e) = grpc_res {
        eprintln!("gRPC server error: {}", e);
    }
    if let Err(e) = http_res {
        eprintln!("HTTP server error: {}", e);
    }

    // The debouncer_task will be aborted when main exits. For a graceful shutdown,
    // a cancellation token would be needed, but this is sufficient for now.
    let _ = debouncer_task;

    Ok(())
}
