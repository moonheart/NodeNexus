// 主入口文件
use backend::server::agent_state::{ConnectedAgents, LiveServerDataCache}; // Added LiveServerDataCache
use backend::server::service::MyAgentCommService;
use backend::websocket_models::{FullServerListPush, ServerWithDetails}; // Added ServerWithDetails
use backend::db::services as db_services; // Added for cache population
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
    );
    
    let grpc_service = AgentCommunicationServiceServer::new(agent_comm_service);
    let grpc_server_future = TonicServer::builder()
        .add_service(grpc_service)
        .serve(grpc_addr);

    println!("gRPC AgentCommunicationService listening on {}", grpc_addr);
    
    // --- Periodic WebSocket Push Task Setup ---
    let cache_for_push_task = live_server_data_cache.clone();
    let broadcaster_for_push_task = ws_data_broadcaster_tx.clone();
    let push_interval_seconds = env::var("WS_PUSH_INTERVAL_SECONDS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2); // Default to 2 seconds

    let periodic_push_task_future = tokio::spawn(async move {
        println!("WebSocket periodic push task started with interval: {}s", push_interval_seconds);
        let mut tick_interval = interval(Duration::from_secs(push_interval_seconds));
        loop {
            tick_interval.tick().await;
            if broadcaster_for_push_task.receiver_count() == 0 {
                // No active WebSocket clients, skip building and sending data
                continue;
            }

            let data_to_push_arc = {
                let cache_guard = cache_for_push_task.lock().await;
                let servers_list: Vec<ServerWithDetails> = cache_guard.values().cloned().collect();
                // println!("Periodic push: Found {} servers in cache.", servers_list.len());
                Arc::new(FullServerListPush { servers: servers_list })
            };

            if let Err(e) = broadcaster_for_push_task.send(data_to_push_arc) {
                // This error typically means there are no active subscribers,
                // though we check receiver_count() above. It could also be other internal errors.
                eprintln!("Failed to broadcast WebSocket data ({} receivers): {}", broadcaster_for_push_task.receiver_count(), e);
            } else {
                // println!("Successfully broadcasted WebSocket data to {} receivers.", broadcaster_for_push_task.receiver_count());
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
