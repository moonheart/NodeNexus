use std::error::Error;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::task::JoinHandle; // For task handles
use tonic::transport::Server as TonicServer; // Added for gRPC server

// Correct approach:
// 1. In backend/src/lib.rs, add `pub mod agent_modules;`
// 2. Here, use `use backend::agent_modules::config::{load_cli_config, AgentCliConfig};` etc.

// Let's proceed assuming agent_modules are part of the backend crate library.
use backend::agent_modules::config::{load_cli_config, AgentCliConfig};
use backend::agent_modules::communication::{ConnectionHandler, heartbeat_loop, server_message_handler_loop};
use backend::agent_modules::metrics::metrics_collection_loop;
use backend::agent_modules::command_tracker::RunningCommandsTracker;
use backend::agent_modules::service_monitor::ServiceMonitorManager;
// Removed: use backend::agent_modules::agent_command_service_impl::create_agent_command_service;
use backend::agent_service::AgentConfig;


const INITIAL_CLIENT_MESSAGE_ID: AtomicU64 = AtomicU64::new(1);
const MAX_RECONNECT_DELAY_SECONDS: u64 = 60 * 5;
const DEFAULT_RECONNECT_DELAY_SECONDS: u64 = 5;


async fn spawn_and_monitor_core_tasks(
    handler: ConnectionHandler,
    agent_cli_config: &AgentCliConfig,
    shared_agent_config: Arc<RwLock<AgentConfig>>,
    command_tracker: Arc<RunningCommandsTracker>, // Added command_tracker
) -> Vec<JoinHandle<()>> {
    let (
        in_stream,
        tx_to_server,
        client_message_id_counter, // This is an Arc<Mutex<u64>>
        assigned_agent_id,
        _initial_agent_config, // No longer the source of truth, config is now in shared_agent_config
    ) = handler.split_for_tasks();

    let mut tasks = Vec::new();

    // Metrics Task
    let metrics_tx = tx_to_server.clone();
    let metrics_agent_config = Arc::clone(&shared_agent_config);
    let metrics_agent_id = assigned_agent_id.clone();
    let metrics_id_provider_counter = client_message_id_counter.clone();
    let metrics_vps_id = agent_cli_config.vps_id;
    let metrics_agent_secret = agent_cli_config.agent_secret.clone();
    // Get the closure for ID generation
    let metrics_id_provider = backend::agent_modules::communication::ConnectionHandler::get_id_provider_closure(metrics_id_provider_counter);
    
    tasks.push(tokio::spawn(async move {
        let agent_id_for_log = metrics_agent_id.clone(); // Clone for logging
        metrics_collection_loop(
            metrics_tx,
            metrics_agent_config,
            metrics_agent_id, // Ownership moved here
            metrics_id_provider, // Pass the closure
            metrics_vps_id,
            metrics_agent_secret,
        ).await;
        println!("[Agent:{}] Metrics collection loop ended.", agent_id_for_log); // Use the clone
    }));

    // Heartbeat Task
    let heartbeat_tx = tx_to_server.clone(); // tx_to_server is an mpsc::Sender, clone is cheap
    let heartbeat_agent_config = Arc::clone(&shared_agent_config);
    let heartbeat_agent_id = assigned_agent_id.clone();
    let heartbeat_id_provider_counter = client_message_id_counter.clone();
    let heartbeat_vps_id = agent_cli_config.vps_id;
    let heartbeat_agent_secret = agent_cli_config.agent_secret.clone();
    let heartbeat_id_provider = backend::agent_modules::communication::ConnectionHandler::get_id_provider_closure(heartbeat_id_provider_counter);

    tasks.push(tokio::spawn(async move {
        let agent_id_for_log = heartbeat_agent_id.clone(); // Clone for logging
        heartbeat_loop(
            heartbeat_tx,
            heartbeat_agent_config,
            heartbeat_agent_id, // Ownership moved here
            heartbeat_id_provider, // Pass the closure
            heartbeat_vps_id,
            heartbeat_agent_secret,
        ).await;
        println!("[Agent:{}] Heartbeat loop ended.", agent_id_for_log); // Use the clone
    }));
    
    // Server Listener Task
    let listener_tx = tx_to_server.clone();
    let listener_agent_id = assigned_agent_id.clone();
    let listener_id_provider_counter = client_message_id_counter.clone();
    let listener_vps_id = agent_cli_config.vps_id;
    let listener_agent_secret = agent_cli_config.agent_secret.clone();
    let listener_id_provider = backend::agent_modules::communication::ConnectionHandler::get_id_provider_closure(listener_id_provider_counter);
    let listener_agent_config = Arc::clone(&shared_agent_config);
    let listener_config_path = "agent_config.toml".to_string();
    let listener_command_tracker = command_tracker.clone(); // Clone command_tracker for the listener task

    // Note: server_message_handler_loop takes ownership of in_stream
    tasks.push(tokio::spawn(async move {
        let agent_id_for_log = listener_agent_id.clone();
        server_message_handler_loop(
            in_stream,
            listener_tx,
            listener_agent_id,
            listener_id_provider,
            listener_vps_id,
            listener_agent_secret,
            listener_agent_config,
            listener_config_path,
            listener_command_tracker, // Pass command_tracker
        ).await;
        println!("[Agent:{}] Server message handler loop ended.", agent_id_for_log);
    }));

// Service Monitor Task
    let monitor_tx = tx_to_server.clone();
    let monitor_agent_config = Arc::clone(&shared_agent_config);
    let monitor_agent_id = assigned_agent_id.clone();
    let monitor_vps_id = agent_cli_config.vps_id;
    let monitor_agent_secret = agent_cli_config.agent_secret.clone();
    let monitor_id_provider =
        backend::agent_modules::communication::ConnectionHandler::get_id_provider_closure(
            client_message_id_counter.clone(),
        );
    tasks.push(tokio::spawn(async move {
        let mut monitor_manager = ServiceMonitorManager::new();
        monitor_manager
            .service_monitor_loop(
                monitor_agent_config,
                monitor_tx,
                monitor_vps_id,
                monitor_agent_secret,
                monitor_id_provider,
            )
            .await;
        println!("[Agent:{}] Service monitor loop ended.", monitor_agent_id);
    }));
    println!("[Agent:{}] All core tasks spawned.", assigned_agent_id);
    tasks
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Setup logging (optional, but good practice)
    // env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    // Or use tracing, etc.

    println!("[Agent] Starting agent...");

    let config_path = "agent_config.toml"; // Relative to current working directory
    let agent_cli_config = match load_cli_config(config_path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("[Agent] Critical error loading configuration: {}. Exiting.", e);
            return Err(e);
        }
    };

    // Create RunningCommandsTracker here, to be passed to tasks
    let command_tracker = Arc::new(RunningCommandsTracker::new());

    // --- Removed setup for Agent's own gRPC Command Service ---
    // The agent will handle commands received over the main communication stream.

    let mut reconnect_delay_seconds = DEFAULT_RECONNECT_DELAY_SECONDS;

    // Main loop for connecting to the primary server
    loop {
        println!("[Agent] Main client loop: Attempting connection to server {} (delay: {}s)...", agent_cli_config.server_address, reconnect_delay_seconds);
        
        // Attempt to connect and handshake (client role)
        // Load the initial value from AtomicU64
        let initial_id = INITIAL_CLIENT_MESSAGE_ID.load(std::sync::atomic::Ordering::SeqCst);
        match ConnectionHandler::connect_and_handshake(&agent_cli_config, initial_id).await {
            Ok(handler) => {
                let assigned_agent_id_log = handler.assigned_agent_id.clone();
                println!("[Agent:{}] Connection and handshake successful. Spawning tasks.", assigned_agent_id_log);
                reconnect_delay_seconds = DEFAULT_RECONNECT_DELAY_SECONDS; // Reset delay on successful connection

                // Create the shared, mutable configuration state
                let shared_agent_config = Arc::new(RwLock::new(handler.initial_agent_config.clone()));
                // Pass command_tracker to spawn_and_monitor_core_tasks
                let task_handles = spawn_and_monitor_core_tasks(handler, &agent_cli_config, shared_agent_config, command_tracker.clone()).await;

                // Monitor tasks. If any of them exit, it signifies a problem, and we should attempt to reconnect.
                if !task_handles.is_empty() {
                    // futures::future::select_all waits for the first task to complete.
                    // The result includes the completed task's output, its index, and the remaining futures.
                    let (first_task_result, _index, remaining_handles) = futures::future::select_all(task_handles).await;
                    
                    match first_task_result {
                        Ok(_) => { // Task completed without panic
                            // This is expected if a task like heartbeat_loop or server_message_handler_loop exits due to error (e.g., send fail, stream break)
                            eprintln!("[Agent:{}] A core task finished. This usually indicates a connection issue or an internal task error.", assigned_agent_id_log);
                        }
                        Err(join_error) => { // Task panicked
                            eprintln!("[Agent:{}] A core task panicked: {:?}. This is a critical issue.", assigned_agent_id_log, join_error);
                            // Depending on the error, might want a longer backoff or specific error handling.
                        }
                    }

                    // Abort all other running tasks to ensure a clean state before reconnecting.
                    println!("[Agent:{}] Aborting remaining tasks before reconnecting...", assigned_agent_id_log);
                    for handle in remaining_handles {
                        handle.abort();
                    }
                } else {
                     eprintln!("[Agent:{}] No tasks were spawned, which is unexpected after successful handshake. This should not happen.", assigned_agent_id_log);
                     // This case implies an issue in spawn_and_monitor_core_tasks or ConnectionHandler::split_for_tasks
                }
                
                eprintln!("[Agent:{}] A task ended or an issue occurred. Preparing to reconnect...", assigned_agent_id_log);

            }
            Err(e) => {
                eprintln!("[Agent] Failed to connect or handshake: {}. Will retry.", e);
                // Error already logged by connect_and_handshake or load_cli_config
            }
        }
        
        // Exponential backoff for retrying connection
        println!("[Agent] Sleeping for {} seconds before next connection attempt.", reconnect_delay_seconds);
        tokio::time::sleep(Duration::from_secs(reconnect_delay_seconds)).await;
        reconnect_delay_seconds = (reconnect_delay_seconds * 2).min(MAX_RECONNECT_DELAY_SECONDS);
    }
    // Loop is infinite, so Ok(()) is effectively unreachable but satisfies function signature.
    // For a real application, you might have a shutdown signal (e.g., Ctrl-C handler)
    // that breaks the loop and allows for graceful termination.
    // The agent_grpc_server_handle was removed as the agent no longer hosts its own gRPC service.
}
