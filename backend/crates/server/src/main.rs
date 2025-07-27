
pub mod axum_embed;
pub mod db;
pub mod services;
pub mod web;
pub mod server;

pub mod alerting; // Added alerting module
pub mod notifications;
pub mod version;


#[macro_use]
extern crate rust_i18n;

// Load all translations from the locales directory
i18n!("locales", fallback = "en");

use nodenexus_common::agent_service::agent_communication_service_server::AgentCommunicationServiceServer;
use crate::alerting::evaluation_service::EvaluationService; // Added EvaluationService
use crate::db::{duckdb_service, services as db_services};
use crate::db::duckdb_service::{tasks::DuckDBTaskManager, DuckDBService};
// use crate::db::services::{AlertService, BatchCommandManager}; // Added BatchCommandManager
use crate::notifications::{encryption::EncryptionService, service::NotificationService};
use crate::server::agent_state::{ConnectedAgents, LiveServerDataCache}; // Added LiveServerDataCache
use crate::server::config::ServerConfig;
use crate::server::metric_broadcaster::MetricBroadcaster;
use crate::server::result_broadcaster::{BatchCommandUpdateMsg, ResultBroadcaster}; // Added ResultBroadcaster
use crate::server::service::MyAgentCommService;
use crate::server::update_service; // Added for cache population
use crate::version::VERSION;
use crate::web::models::websocket_models::{ServerWithDetails, WsMessage};
 
use chrono::Utc;
use clap::Parser;
use dotenv::dotenv;
use futures_util::SinkExt;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::{Mutex, broadcast}; // Added Mutex for LiveServerDataCache and broadcast
use tokio::time::{Duration, interval}; // For the periodic push task
use tracing::{debug, error, info, warn};
use tracing_appender::rolling;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use tower::Service;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long)]
    config: Option<String>,
}

fn init_logging() {
    // Log to a file: JSON format, daily rotation
    let file_appender = rolling::daily("logs", "server.log");
    let file_layer = fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false) // No ANSI colors in file
        .json(); // Log as JSON

    // Log to stdout: human-readable format
    let stdout_layer = fmt::layer().with_writer(std::io::stdout);

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
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Added Send + Sync for tokio::spawn
    // Manually check for --version before full parsing to keep the original simple output.
    if std::env::args().any(|arg| arg == "--version") {
        println!("Server version: {VERSION}");
        return Ok(());
    }

    let args = Args::parse();

    init_logging(); // Initialize logging first
    info!("Starting server, version: {}", VERSION);
    dotenv().ok(); // Load .env file

    // --- Debounce Update Trigger Channel ---
    let (update_trigger_tx, mut update_trigger_rx) = mpsc::channel::<()>(100);

    // --- Server Config Setup ---
    let server_config = match ServerConfig::load(args.config.as_deref()) {
        Ok(config) => Arc::new(config),
        Err(e) => {
            error!("Failed to load server configuration: {}", e);
            // Exit if critical configuration is missing
            return Err(e.into());
        }
    };

    // --- Database Pool Setup ---
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env file");
    let mut opt = ConnectOptions::new(database_url.to_owned());
    opt.max_connections(10)
       // .sqlx_logging(true) // 您可以根据需要启用日志记录
       // .sqlx_logging_level(log::LevelFilter::Info) // 设置日志级别
       ;

    let db_pool: DatabaseConnection = Database::connect(opt)
        .await
        .expect("Failed to create database connection.");

   // --- DuckDB Setup ---
   let duckdb_path = "nodenexus.duckdb";
   let duckdb_manager = duckdb::DuckdbConnectionManager::file(duckdb_path).unwrap();
   let duckdb_pool = r2d2::Pool::new(duckdb_manager).expect("Failed to create DuckDB connection pool.");
   let duckdb_service = match DuckDBService::new(duckdb_pool.clone()) {
       Ok(service) => {
           info!("Successfully initialized DuckDB service.");
           service
       }
       Err(e) => {
           error!("Failed to create DuckDB service: {}", e);
           return Err(e.into());
       }
   };
   let duckdb_metric_sender = duckdb_service.get_sender();

   // --- DuckDB Background Tasks ---
   let duckdb_task_manager = Arc::new(DuckDBTaskManager::new(duckdb_path, duckdb_pool.clone()));
   let duckdb_task_handle = tokio::spawn({
       let manager = duckdb_task_manager.clone();
       async move {
           // Run tasks every hour
           manager.run_periodic_tasks(Duration::from_secs(3600)).await;
       }
   });

   // --- gRPC Server Setup ---
    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    let connected_agents = ConnectedAgents::new();

    // --- Shared State Initialization for WebSocket and gRPC ---
    // Initialize the broadcast channel for WebSocket updates
    let (ws_data_broadcaster_tx, _) = broadcast::channel::<WsMessage>(100); // For private, full data
    let (public_ws_data_broadcaster_tx, _) = broadcast::channel::<WsMessage>(100); // For public, desensitized data
    let (batch_command_updates_tx, _rx) = broadcast::channel::<BatchCommandUpdateMsg>(100); // Channel for batch command updates

    // --- Metric Broadcaster Setup ---
    let (metric_broadcaster, metric_sender) = MetricBroadcaster::new(ws_data_broadcaster_tx.clone());
    metric_broadcaster.run();


    // Initialize the live server data cache
    let initial_cache_data_result = db::duckdb_service::vps_detail_service::get_all_vps_with_details_for_cache(duckdb_pool.clone()).await;
    let initial_cache_map: HashMap<i32, ServerWithDetails> = match initial_cache_data_result {
        Ok(servers) => {
            info!(
                server_count = servers.len(),
                "Successfully fetched servers for initial cache."
            );
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
    let encryption_service =
        Arc::new(EncryptionService::new(&key_bytes).expect("Failed to create encryption service."));
    let notification_service = Arc::new(NotificationService::new(
        db_pool.clone(),
        encryption_service.clone(),
    ));
    let result_broadcaster = Arc::new(ResultBroadcaster::new(batch_command_updates_tx.clone())); // Create ResultBroadcaster

    // --- gRPC Server Setup (continued) ---
    let agent_comm_service = MyAgentCommService::new(
        connected_agents.clone(),
        Arc::from(db_pool.clone()),
        duckdb_pool.clone(),
        live_server_data_cache.clone(), // Pass cache to gRPC service
        ws_data_broadcaster_tx.clone(), // Pass broadcaster to gRPC service
        update_trigger_tx.clone(),      // Pass update trigger to gRPC service
        metric_sender.clone(),          // Pass the sender for the metric broadcaster
        duckdb_metric_sender.clone(),
    );

    let grpc_service = AgentCommunicationServiceServer::new(agent_comm_service);

    // --- Agent Liveness Check Task ---
    let connected_agents_for_check = connected_agents.clone();
    let pool_for_check = Arc::new(db_pool.clone());
    let trigger_for_check = update_trigger_tx.clone();

    let duckdb_pool1 = duckdb_pool.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(60)); // Check every 60 seconds
        info!("Agent liveness check task started.");

        loop {
            interval.tick().await;
            let mut agents_guard = connected_agents_for_check.lock().await;

            let now = Utc::now().timestamp_millis();
            let timeout_duration_ms = 60 * 1000; // 60-second threshold

            let timed_out_agent_ids: Vec<i32> = agents_guard
                .agents
                .iter()
                .filter(|(_, state)| now - state.last_seen_ms > timeout_duration_ms)
                .map(|(id, _)| *id)
                .collect();

            let mut disconnected_vps_ids = Vec::new();
            for agent_id in timed_out_agent_ids {
                if let Some(mut state) = agents_guard.agents.remove(&agent_id) {
                    warn!(
                        vps_id = state.vps_db_id,
                        "Agent timed out. Closing connection gracefully."
                    );
                    disconnected_vps_ids.push(state.vps_db_id);

                    // Spawn a new task to close the connection gracefully without blocking the liveness check loop.
                    tokio::spawn(async move {
                        if let Err(e) = state.sender.close().await {
                            warn!(vps_id = %state.vps_db_id, error = %e, "Error closing agent sender gracefully.");
                        }
                    });
                }
            }

            drop(agents_guard); // Release the lock before async DB operations

            if !disconnected_vps_ids.is_empty() {
                warn!(
                    count = disconnected_vps_ids.len(),
                    "Found disconnected agents. Updating status to 'offline'."
                );
                let mut needs_broadcast = false;
                for vps_id in disconnected_vps_ids {
                    match duckdb_service::vps_service::update_vps_status(duckdb_pool1.clone(), vps_id, "offline").await {
                        // Dereference Arc
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
                        error!("Failed to send update trigger from liveness check task.");
                    }
                }
            }
        }
    });

    // --- Axum HTTP Server Setup ---
    let http_router = crate::web::create_axum_router(
        db_pool.clone(),
        live_server_data_cache.clone(),
        duckdb_pool.clone(),
        ws_data_broadcaster_tx.clone(),
        public_ws_data_broadcaster_tx.clone(), // Pass public broadcaster
        connected_agents.clone(),
        update_trigger_tx.clone(),
        notification_service.clone(),
        batch_command_updates_tx.clone(),
        result_broadcaster.clone(),
        server_config.clone(),
        metric_sender.clone(),
        duckdb_metric_sender.clone(),
    );

    // --- Debounced Broadcast Task ---
    let pool_for_debounce = duckdb_pool.clone();
    let cache_for_debounce = live_server_data_cache.clone();
    let private_broadcaster_for_debounce = ws_data_broadcaster_tx.clone();
    let public_broadcaster_for_debounce = public_ws_data_broadcaster_tx.clone();
    let debouncer_task = tokio::spawn(async move {
        use tokio::time::{Duration, sleep};
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
            while update_trigger_rx.try_recv().is_ok() {
                // Discard additional signals.
            }

            // Now that the stream of updates has settled, perform the actual broadcast.
            debug!("Debounce window finished. Triggering broadcast to both channels.");
            update_service::broadcast_full_state_update_to_all(
                pool_for_debounce.clone(),
                &cache_for_debounce,
                &private_broadcaster_for_debounce,
                &public_broadcaster_for_debounce,
            )
            .await;
        }
    });

    // --- Alert Evaluation Service Task ---
    let alert_evaluation_service = Arc::new(EvaluationService::new(
        duckdb_pool.clone(),
        notification_service.clone(),
    ));
    let evaluation_task = tokio::spawn(async move {
        // Define the evaluation interval (e.g., 60 seconds)
        alert_evaluation_service.start_periodic_evaluation(60).await;
    });


    // --- Renewal Reminder Check Task ---
    let pool_for_renewal_reminder = db_pool.clone();
    let trigger_for_renewal_reminder = update_trigger_tx.clone();
    const REMINDER_THRESHOLD_DAYS: i64 = 7; // Remind 7 days in advance
    const RENEWAL_REMINDER_CHECK_INTERVAL_SECONDS: u64 = 6 * 60 * 60; // Check every 6 hours

    let duckdb_pool1 = duckdb_pool.clone();
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
            match duckdb_service::vps_renewal_service::check_and_generate_reminders(
                duckdb_pool1.clone(),
                REMINDER_THRESHOLD_DAYS,
            )
            .await
            {
                Ok(reminders_generated) => {
                    if reminders_generated > 0 {
                        info!(
                            count = reminders_generated,
                            "Renewal reminders were generated/updated. Triggering state update."
                        );
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
            match duckdb_service::vps_renewal_service::process_all_automatic_renewals(duckdb_pool.clone()).await {
                Ok(renewed_count) => {
                    if renewed_count > 0 {
                        info!(
                            count = renewed_count,
                            "VPS were automatically renewed. Triggering state update."
                        );
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
    let socket = if addr.is_ipv4() {
        tokio::net::TcpSocket::new_v4()?
    } else {
        tokio::net::TcpSocket::new_v6()?
    };
    socket.set_reuseaddr(true)?;
    socket.set_keepalive(true)?;
    socket.bind(addr)?;
    let listener = socket.listen(1024)?;
    info!(address = %addr, "HTTP and gRPC server listening with TCP Keepalive");

    let static_file_service = crate::web::create_static_file_service();

    let app = http_router.fallback_service(tower::service_fn(
        move |req: axum::http::Request<axum::body::Body>| {
            let mut grpc_service = grpc_service.clone();
            let mut static_file_service = static_file_service.clone();

            info!("Received request: {} {}", req.method(), req.uri());

            async move {
                if req
                    .headers()
                    .get("content-type")
                    .map(|v| v.as_bytes().starts_with(b"application/grpc"))
                    .unwrap_or(false)
                {
                    grpc_service
                        .call(req)
                        .await
                        .map(|res| res.map(axum::body::Body::new))
                        .map_err(|err| match err {})
                } else {
                    static_file_service
                        .call(req)
                        .await
                        .map(|res| res.map(axum::body::Body::new))
                        .map_err(|err| match err {})
                }
            }
        },
    ));

    axum::serve(listener, app.into_make_service())
        .await
        .map_err(Box::new)?;

    // The debouncer_task will be aborted when main exits. For a graceful shutdown,
    // a cancellation token would be needed, but this is sufficient for now.
    let _ = debouncer_task;
    let _ = evaluation_task;
    let _ = duckdb_task_handle; // Keep the DuckDB task handle
    // The renewal reminder task also runs in the background and will be aborted when main exits.
 
    Ok(())
}
