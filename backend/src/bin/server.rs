// 主入口文件
use backend::server::agent_state::{ConnectedAgents, LiveServerDataCache}; // Added LiveServerDataCache
use backend::server::service::MyAgentCommService;
use backend::websocket_models::{ServerWithDetails, WsMessage};
use backend::db::services as db_services;
use backend::server::update_service; // Added for cache population
use backend::notifications::{encryption::EncryptionService, service::NotificationService};
use backend::db::services::{AlertService, BatchCommandManager}; // Added BatchCommandManager
use backend::alerting::evaluation_service::EvaluationService; // Added EvaluationService
use backend::server::result_broadcaster::{ResultBroadcaster, BatchCommandUpdateMsg}; // Added ResultBroadcaster
use tonic::transport::Server as TonicServer;
use backend::agent_service::agent_communication_service_server::AgentCommunicationServiceServer;
use backend::http_server;

use sea_orm::{Database, DatabaseConnection, ConnectOptions};
use std::net::SocketAddr;
use dotenv::dotenv;
use std::env;
use std::sync::Arc;
use std::collections::HashMap; // For initializing LiveServerDataCache
use tokio::sync::{broadcast, Mutex}; // Added Mutex for LiveServerDataCache and broadcast
use tokio::time::{interval, Duration}; // For the periodic push task
use chrono::Utc;
use tokio::sync::mpsc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, fmt};
use tracing_appender::rolling;
use tracing::{info, error, warn, debug};

fn init_logging() {
    // Log to a file: JSON format, daily rotation
    let file_appender = rolling::daily("logs", "server.log");
    let file_layer = fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false) // No ANSI colors in file
        .json(); // Log as JSON

    // Log to stdout: human-readable format
    let stdout_layer = fmt::layer()
        .with_writer(std::io::stdout);

    // Combine layers and filter based on RUST_LOG
    // Default to `info,sea_orm=warn` level if RUST_LOG is not set.
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,sea_orm=warn,sqlx::query=warn"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stdout_layer)
        .init();
    
    // This allows libraries using the `log` crate to work with `tracing`
    // tracing_log::LogTracer::init().expect("Failed to set logger");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> { // Added Send + Sync for tokio::spawn
    init_logging(); // Initialize logging first
    dotenv().ok(); // Load .env file

    // --- Debounce Update Trigger Channel ---
    let (update_trigger_tx, mut update_trigger_rx) = mpsc::channel::<()>(100);


    // --- Database Pool Setup ---
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env file");
    let mut opt = ConnectOptions::new(database_url.to_owned());
    opt.max_connections(10)
       // .sqlx_logging(true) // 您可以根据需要启用日志记录
       // .sqlx_logging_level(log::LevelFilter::Info) // 设置日志级别
       ;

    let db_pool: DatabaseConnection = Database::connect(opt)
        .await
        .expect("Failed to create database connection.");
    
    // --- gRPC Server Setup ---
    let grpc_addr: SocketAddr = "0.0.0.0:50051".parse()?;
    let connected_agents = ConnectedAgents::new();

    // --- Shared State Initialization for WebSocket and gRPC ---
    // Initialize the broadcast channel for WebSocket updates
    let (ws_data_broadcaster_tx, _) = broadcast::channel::<WsMessage>(100); // Capacity can be configured
    let (batch_command_updates_tx, _rx) = broadcast::channel::<BatchCommandUpdateMsg>(100); // Channel for batch command updates

    // Initialize the live server data cache
    let initial_cache_data_result = db_services::get_all_vps_with_details_for_cache(&db_pool).await;
    let initial_cache_map: HashMap<i32, ServerWithDetails> = match initial_cache_data_result {
        Ok(servers) => {
            info!(server_count = servers.len(), "Successfully fetched servers for initial cache.");
            servers.into_iter().map(|s| (s.basic_info.id, s)).collect()
        }
        Err(e) => {
            error!(error = %e, "Failed to fetch initial server data for cache. Initializing with empty cache.");
            HashMap::new()
        }
    };
    let live_server_data_cache: LiveServerDataCache = Arc::new(Mutex::new(initial_cache_map));

    // --- Notification Service Setup ---
    let encryption_key = env::var("NOTIFICATION_ENCRYPTION_KEY")
        .expect("NOTIFICATION_ENCRYPTION_KEY must be set as a 32-byte hex-encoded string.");
    // The hex crate will be added in the next step.
    let key_bytes = hex::decode(encryption_key).expect("Failed to decode encryption key.");
    let encryption_service = Arc::new(EncryptionService::new(&key_bytes).expect("Failed to create encryption service."));
    let notification_service = Arc::new(NotificationService::new(db_pool.clone(), encryption_service.clone()));
    let alert_service = Arc::new(AlertService::new(Arc::new(db_pool.clone()))); // Initialize AlertService
    let result_broadcaster = Arc::new(ResultBroadcaster::new(batch_command_updates_tx.clone())); // Create ResultBroadcaster
    let batch_command_manager = Arc::new(BatchCommandManager::new(Arc::new(db_pool.clone()), result_broadcaster.clone())); // Create BatchCommandManager

    // --- gRPC Server Setup (continued) ---
    let agent_comm_service = MyAgentCommService::new(
        connected_agents.clone(),
        Arc::from(db_pool.clone()),
        live_server_data_cache.clone(), // Pass cache to gRPC service
        ws_data_broadcaster_tx.clone(), // Pass broadcaster to gRPC service
        update_trigger_tx.clone(),      // Pass update trigger to gRPC service
        batch_command_manager.clone(),  // Pass BatchCommandManager to gRPC service
    );

    let grpc_service = AgentCommunicationServiceServer::new(agent_comm_service);
    let grpc_server_future = TonicServer::builder()
        .add_service(grpc_service)
        .serve(grpc_addr);

    info!(address = %grpc_addr, "gRPC AgentCommunicationService listening");
    
    // --- Agent Heartbeat Check Task ---
    let connected_agents_for_check = connected_agents.clone();
    let pool_for_check = Arc::new(db_pool.clone());
    let trigger_for_check = update_trigger_tx.clone();

    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(60)); // Check every 60 seconds
        info!("Agent heartbeat check task started.");

        loop {
            interval.tick().await;
            let mut disconnected_vps_ids = Vec::new();
            let mut agents_guard = connected_agents_for_check.lock().await;

            // Use retain to iterate and remove in-place
            agents_guard.agents.retain(|_agent_id, state| {
                let is_alive = (Utc::now().timestamp_millis() - state.last_heartbeat_ms) < 90_000; // 90-second threshold
                if !is_alive {
                    warn!(vps_id = state.vps_db_id, "Agent is considered disconnected due to heartbeat timeout.");
                    disconnected_vps_ids.push(state.vps_db_id);
                }
                is_alive
            });

            drop(agents_guard); // Release the lock before async operations

            if !disconnected_vps_ids.is_empty() {
                warn!(count = disconnected_vps_ids.len(), "Found disconnected agents. Updating status to 'offline'.");
                let mut needs_broadcast = false;
                for vps_id in disconnected_vps_ids {
                    match db_services::update_vps_status(&*pool_for_check, vps_id, "offline").await { // Dereference Arc
                        Ok(rows_affected) if rows_affected > 0 => {
                            needs_broadcast = true;
                        }
                        Ok(_) => {} // No rows affected, maybe already offline
                        Err(e) => {
                            error!(vps_id = vps_id, error = %e, "Failed to update status to 'offline'.");
                        }
                    }
                }

                if needs_broadcast {
                    info!("Triggering broadcast after updating offline status.");
                    if trigger_for_check.send(()).await.is_err() {
                        error!("Failed to send update trigger from heartbeat check task.");
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
        notification_service.clone(),
        alert_service.clone(), // Pass AlertService to HTTP server
        batch_command_manager.clone(), // Pass BatchCommandManager to HTTP server
        batch_command_updates_tx.clone(), // Pass batch_command_updates_tx to HTTP server
        result_broadcaster.clone(), // Pass result_broadcaster to HTTP server
    );

    // --- Debounced Broadcast Task ---
    let pool_for_debounce = db_pool.clone();
    let cache_for_debounce = live_server_data_cache.clone();
    let broadcaster_for_debounce = ws_data_broadcaster_tx.clone();
    let debouncer_task = tokio::spawn(async move {
        use tokio::time::{sleep, Duration};
        const DEBOUNCE_DURATION: Duration = Duration::from_millis(2000);

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
            debug!("Debounce window finished. Triggering broadcast.");
            update_service::broadcast_full_state_update(
                &pool_for_debounce,
                &cache_for_debounce,
                &broadcaster_for_debounce,
            )
            .await;
        }
    });

    // --- Alert Evaluation Service Task ---
    let alert_evaluation_service = Arc::new(EvaluationService::new(
        Arc::new(db_pool.clone()), // Wrap db_pool in Arc
        notification_service.clone(),
        alert_service.clone(),
    ));
    let evaluation_task = tokio::spawn(async move {
        // Define the evaluation interval (e.g., 60 seconds)
        alert_evaluation_service.start_periodic_evaluation(60).await;
    });

    // --- Traffic Reset Check Task ---
    let pool_for_traffic_reset = db_pool.clone();
    let trigger_for_traffic_reset = update_trigger_tx.clone();
    let traffic_reset_task = tokio::spawn(async move {
        // Check every 5 minutes, for example. This can be configurable.
        let mut interval = interval(Duration::from_secs(5 * 60));
        info!("Traffic reset check task started. Interval: 5 minutes.");

        loop {
            interval.tick().await;
            info!("Performing scheduled VPS traffic reset check...");
            match db_services::get_vps_due_for_traffic_reset(&pool_for_traffic_reset).await {
                Ok(vps_ids) => {
                    if vps_ids.is_empty() {
                        debug!("No VPS due for traffic reset at this time.");
                        continue;
                    }
                    info!(count = vps_ids.len(), vps_ids = ?vps_ids, "Found VPS(s) due for traffic reset.");
                    let mut reset_performed_for_any_vps = false;
                    for vps_id in vps_ids {
                        match db_services::process_vps_traffic_reset(&pool_for_traffic_reset, vps_id).await {
                            Ok(reset_performed) => {
                                if reset_performed {
                                    info!(vps_id = vps_id, "Traffic reset successfully processed.");
                                    reset_performed_for_any_vps = true;
                                } else {
                                    debug!(vps_id = vps_id, "Traffic reset not performed (either not due or already handled).");
                                }
                            }
                            Err(e) => {
                                error!(vps_id = vps_id, error = %e, "Error processing traffic reset.");
                            }
                        }
                    }
                    if reset_performed_for_any_vps {
                        info!("Traffic reset performed for one or more VPS. Triggering state update broadcast.");
                        if trigger_for_traffic_reset.send(()).await.is_err() {
                            error!("Failed to send update trigger from traffic reset task.");
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "Error fetching VPS due for traffic reset.");
                }
            }
        }
    });

    // --- Renewal Reminder Check Task ---
    let pool_for_renewal_reminder = db_pool.clone();
    let trigger_for_renewal_reminder = update_trigger_tx.clone();
    const REMINDER_THRESHOLD_DAYS: i64 = 7; // Remind 7 days in advance
    const RENEWAL_REMINDER_CHECK_INTERVAL_SECONDS: u64 = 6 * 60 * 60; // Check every 6 hours

    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(RENEWAL_REMINDER_CHECK_INTERVAL_SECONDS));
        info!(
            interval_seconds = RENEWAL_REMINDER_CHECK_INTERVAL_SECONDS,
            threshold_days = REMINDER_THRESHOLD_DAYS,
            "Renewal reminder check task started."
        );

        loop {
            interval.tick().await;
            info!("Performing scheduled renewal reminder check...");
            match db_services::check_and_generate_reminders(&pool_for_renewal_reminder, REMINDER_THRESHOLD_DAYS).await {
                Ok(reminders_generated) => {
                    if reminders_generated > 0 {
                        info!(count = reminders_generated, "Renewal reminders were generated/updated. Triggering state update.");
                        if trigger_for_renewal_reminder.send(()).await.is_err() {
                            error!("Failed to send update trigger from renewal reminder task.");
                        }
                    } else {
                        debug!("No new renewal reminders generated at this time.");
                    }
                }
                Err(e) => {
                    error!(error = %e, "Error checking/generating renewal reminders.");
                }
            }
        }
    });
 
    // --- Automatic Renewal Processing Task ---
    let pool_for_auto_renewal = db_pool.clone();
    let trigger_for_auto_renewal = update_trigger_tx.clone();
    const AUTO_RENEWAL_CHECK_INTERVAL_SECONDS: u64 = 6 * 60 * 60; // Check every 6 hours

    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(AUTO_RENEWAL_CHECK_INTERVAL_SECONDS));
        info!(
            interval_seconds = AUTO_RENEWAL_CHECK_INTERVAL_SECONDS,
            "Automatic renewal processing task started."
        );

        loop {
            interval.tick().await;
            info!("Performing scheduled automatic renewal processing...");
            match db_services::process_all_automatic_renewals(&pool_for_auto_renewal).await {
                Ok(renewed_count) => {
                    if renewed_count > 0 {
                        info!(count = renewed_count, "VPS were automatically renewed. Triggering state update.");
                        if trigger_for_auto_renewal.send(()).await.is_err() {
                            error!("Failed to send update trigger from automatic renewal task.");
                        }
                    } else {
                        debug!("No VPS were automatically renewed at this time.");
                    }
                }
                Err(e) => {
                    error!(error = %e, "Error processing automatic renewals.");
                }
            }
        }
    });


    // --- Run all servers and tasks concurrently ---
    let grpc_handle = tokio::spawn(grpc_server_future);
    let http_handle = tokio::spawn(http_server_future);

    // Use `try_join!` to await the main server tasks. The debouncer runs in the background.
    let (grpc_res, http_res) = tokio::try_join!(grpc_handle, http_handle)?;

    if let Err(e) = grpc_res {
        error!(error = %e, "gRPC server exited with an error.");
    }
    if let Err(e) = http_res {
        error!(error = %e, "HTTP server exited with an error.");
    }

    // The debouncer_task will be aborted when main exits. For a graceful shutdown,
    // a cancellation token would be needed, but this is sufficient for now.
    let _ = debouncer_task;
    let _ = evaluation_task; // Keep the evaluation task handle
    let _ = traffic_reset_task; // Keep the traffic reset task handle
    // The renewal reminder task also runs in the background and will be aborted when main exits.

    Ok(())
}
