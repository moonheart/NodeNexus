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
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> { // Added Send + Sync for tokio::spawn
    dotenv().ok(); // Load .env file

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
    );
    
    let grpc_service = AgentCommunicationServiceServer::new(agent_comm_service);
    let grpc_server_future = TonicServer::builder()
        .add_service(grpc_service)
        .serve(grpc_addr);

    println!("gRPC AgentCommunicationService listening on {}", grpc_addr);
    
    // --- Agent Heartbeat Check Task ---
    let connected_agents_for_check = connected_agents.clone();
    let pool_for_check = Arc::new(db_pool.clone());
    let cache_for_check = live_server_data_cache.clone();
    let broadcaster_for_check = ws_data_broadcaster_tx.clone();

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
                    update_service::broadcast_full_state_update(
                        &pool_for_check,
                        &cache_for_check,
                        &broadcaster_for_check,
                    )
                    .await;
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
        connected_agents.clone(), // Pass connected agents state to HTTP service
    );

    // Run all servers and tasks concurrently
    let grpc_handle = tokio::spawn(async move {
        if let Err(e) = grpc_server_future.await {
            eprintln!("gRPC server error: {}", e);
        }
    });

    let http_handle = tokio::spawn(async move {
        if let Err(e) = http_server_future.await {
            eprintln!("HTTP server error: {}", e);
        }
    });
    
    // Also await the periodic push task if you want the main function to keep it alive explicitly,
    // or ensure it's handled correctly on shutdown. For now, it runs as a detached task.
    // If we await it, main will only exit if this task errors or completes.
    // For a continuously running server, we might not await it directly in try_join unless it's designed to complete.
    // let (grpc_result, http_result, _push_task_result) = tokio::try_join!(grpc_handle, http_handle, periodic_push_task_future)?;


    let (_grpc_result, _http_result) = tokio::try_join!(grpc_handle, http_handle)?;
    // The periodic_push_task_future will run in the background.
    // To ensure it's properly managed, especially on shutdown, more sophisticated handling might be needed,
    // e.g., using a shutdown signal. For now, it will stop when main exits.
    // If periodic_push_task_future is awaited, it needs to be fallible or main won't exit cleanly on other errors.
    // For simplicity, we are not awaiting it in the try_join.
    // To make it awaitable and handle its potential error:
    // let periodic_push_handle = tokio::spawn(periodic_push_task_future);
    // let (grpc_res, http_res, periodic_res) = tokio::try_join!(grpc_handle, http_handle, periodic_push_handle)?;
    // This would require periodic_push_task_future to return a Result.

    Ok(())
}
