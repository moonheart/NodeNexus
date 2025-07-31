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
use crate::db::{duckdb_service};
use crate::db::duckdb_service::{tasks::DuckDBTaskManager, DuckDBService};
// use crate::db::services::{AlertService, BatchCommandManager}; // Added BatchCommandManager
use crate::notifications::encryption::EncryptionService;
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
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::{Mutex, broadcast, watch}; // Added watch
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
    // Default to `info` level if RUST_LOG is not set.
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stdout_layer)
        .init();

    // This allows libraries using the `log` crate to work with `tracing`
    // tracing_log::LogTracer::init().expect("Failed to set logger");
}

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Manually check for --version before full parsing to keep the original simple output.
    if std::env::args().any(|arg| arg == "--version") {
        println!("Server version: {VERSION}");
        return Ok(());
    }

    init_logging(); // Initialize logging first
    info!("Starting server, version: {}", VERSION);
    dotenv().ok(); // Load .env file

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    rt.block_on(async {
        let (shutdown_tx, shutdown_rx) = watch::channel(());

        let server_future = run_server(shutdown_rx);

        tokio::select! {
            res = server_future => {
                if let Err(e) = res {
                    error!(error = %e, "Server failed to run.");
                }
            },
            _ = tokio::signal::ctrl_c() => {
                info!("Ctrl-C received, shutting down gracefully.");
                // The drop of shutdown_tx will signal all receivers.
            },
        }
    });

    Ok(())
}

async fn run_server(mut shutdown_rx: watch::Receiver<()>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args = Args::parse();

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
       let mut shutdown_rx = shutdown_rx.clone();
       async move {
            tokio::select! {
                _ = manager.run_periodic_tasks(Duration::from_secs(3600)) => {},
                _ = shutdown_rx.changed() => {
                    info!("DuckDB task manager shutting down.");
                }
            }
       }
   });

   // --- gRPC Server Setup ---
    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    let connected_agents = ConnectedAgents::new();

    // --- Shared State Initialization for WebSocket and gRPC ---
    let (ws_data_broadcaster_tx, _) = broadcast::channel::<WsMessage>(100);
    let (public_ws_data_broadcaster_tx, _) = broadcast::channel::<WsMessage>(100);
    let (batch_command_updates_tx, _rx) = broadcast::channel::<BatchCommandUpdateMsg>(100);

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
    let key_bytes = hex::decode(encryption_key).expect("Failed to decode encryption key.");
    let encryption_service =
        Arc::new(EncryptionService::new(&key_bytes).expect("Failed to create encryption service."));
    let result_broadcaster = Arc::new(ResultBroadcaster::new(batch_command_updates_tx.clone()));

    // --- gRPC Server Setup (continued) ---
    let agent_comm_service = MyAgentCommService::new(
        connected_agents.clone(),
        duckdb_pool.clone(),
        live_server_data_cache.clone(),
        ws_data_broadcaster_tx.clone(),
        update_trigger_tx.clone(),
        metric_sender.clone(),
        duckdb_metric_sender.clone(),
        shutdown_rx.clone(),
        result_broadcaster.clone(),
    );

    let grpc_service = AgentCommunicationServiceServer::new(agent_comm_service);

    // --- Agent Liveness Check Task ---
    let connected_agents_for_check = connected_agents.clone();
    let trigger_for_check = update_trigger_tx.clone();
    let duckdb_pool1 = duckdb_pool.clone();
    let mut liveness_shutdown_rx = shutdown_rx.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(60));
        info!("Agent liveness check task started.");

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let mut agents_guard = connected_agents_for_check.lock().await;
                    let now = Utc::now().timestamp_millis();
                    let timeout_duration_ms = 60 * 1000;

                    let timed_out_agent_ids: Vec<i32> = agents_guard
                        .agents
                        .iter()
                        .filter(|(_, state)| now - state.last_seen_ms > timeout_duration_ms)
                        .map(|(id, _)| *id)
                        .collect();

                    let mut disconnected_vps_ids = Vec::new();
                    for agent_id in timed_out_agent_ids {
                        if let Some(mut state) = agents_guard.agents.remove(&agent_id) {
                            warn!(vps_id = state.vps_db_id, "Agent timed out. Closing connection gracefully.");
                            disconnected_vps_ids.push(state.vps_db_id);
                            tokio::spawn(async move {
                                if let Err(e) = state.sender.close().await {
                                    warn!(vps_id = %state.vps_db_id, error = %e, "Error closing agent sender gracefully.");
                                }
                            });
                        }
                    }
                    drop(agents_guard);

                    if !disconnected_vps_ids.is_empty() {
                        warn!(count = disconnected_vps_ids.len(), "Found disconnected agents. Updating status to 'offline'.");
                        let mut needs_broadcast = false;
                        for vps_id in disconnected_vps_ids {
                            match duckdb_service::vps_service::update_vps_status(duckdb_pool1.clone(), vps_id, "offline").await {
                                Ok(rows_affected) if rows_affected > 0 => needs_broadcast = true,
                                Ok(_) => {}
                                Err(e) => error!(vps_id = vps_id, error = %e, "Failed to update status to 'offline'."),
                            }
                        }
                        if needs_broadcast {
                            info!("Triggering broadcast after updating offline status.");
                            if trigger_for_check.send(()).await.is_err() {
                                error!("Failed to send update trigger from liveness check task.");
                            }
                        }
                    }
                },
                _ = liveness_shutdown_rx.changed() => {
                    info!("Agent liveness check task shutting down.");
                    break;
                }
            }
        }
    });

    // --- Axum HTTP Server Setup ---
    let http_router = crate::web::create_axum_router(
        live_server_data_cache.clone(),
        duckdb_pool.clone(),
        ws_data_broadcaster_tx.clone(),
        public_ws_data_broadcaster_tx.clone(),
        connected_agents.clone(),
        update_trigger_tx.clone(),
        encryption_service.clone(),
        batch_command_updates_tx.clone(),
        result_broadcaster.clone(),
        server_config.clone(),
        metric_sender.clone(),
        duckdb_metric_sender.clone(),
        shutdown_rx.clone(),
    );

    // --- Debounced Broadcast Task ---
    let pool_for_debounce = duckdb_pool.clone();
    let cache_for_debounce = live_server_data_cache.clone();
    let private_broadcaster_for_debounce = ws_data_broadcaster_tx.clone();
    let public_broadcaster_for_debounce = public_ws_data_broadcaster_tx.clone();
    let mut debouncer_shutdown_rx = shutdown_rx.clone();
    let debouncer_task = tokio::spawn(async move {
        use tokio::time::{Duration, sleep};
        const DEBOUNCE_DURATION: Duration = Duration::from_millis(2000);

        loop {
            tokio::select! {
                Some(_) = update_trigger_rx.recv() => {
                    sleep(DEBOUNCE_DURATION).await;
                    while update_trigger_rx.try_recv().is_ok() {}
                    debug!("Debounce window finished. Triggering broadcast to both channels.");
                    update_service::broadcast_full_state_update_to_all(
                        pool_for_debounce.clone(),
                        &cache_for_debounce,
                        &private_broadcaster_for_debounce,
                        &public_broadcaster_for_debounce,
                    ).await;
                },
                _ = debouncer_shutdown_rx.changed() => {
                    info!("Debouncer task shutting down.");
                    break;
                }
            }
        }
    });

    // --- Alert Evaluation Service Task ---
    let alert_evaluation_service = Arc::new(EvaluationService::new(
        duckdb_pool.clone(),
        encryption_service.clone(),
    ));
    let mut evaluation_shutdown_rx = shutdown_rx.clone();
    let evaluation_task = tokio::spawn(async move {
        tokio::select! {
            _ = alert_evaluation_service.start_periodic_evaluation(60) => {},
            _ = evaluation_shutdown_rx.changed() => {
                info!("Alert evaluation service shutting down.");
            }
        }
    });

    // --- Renewal Reminder Check Task ---
    let trigger_for_renewal_reminder = update_trigger_tx.clone();
    const REMINDER_THRESHOLD_DAYS: i64 = 7;
    const RENEWAL_REMINDER_CHECK_INTERVAL_SECONDS: u64 = 6 * 60 * 60;
    let duckdb_pool1 = duckdb_pool.clone();
    let mut renewal_shutdown_rx = shutdown_rx.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(RENEWAL_REMINDER_CHECK_INTERVAL_SECONDS));
        info!(interval_seconds = RENEWAL_REMINDER_CHECK_INTERVAL_SECONDS, threshold_days = REMINDER_THRESHOLD_DAYS, "Renewal reminder check task started.");
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    info!("Performing scheduled renewal reminder check...");
                    match duckdb_service::vps_renewal_service::check_and_generate_reminders(duckdb_pool1.clone(), REMINDER_THRESHOLD_DAYS).await {
                        Ok(reminders_generated) if reminders_generated > 0 => {
                            info!(count = reminders_generated, "Renewal reminders were generated/updated. Triggering state update.");
                            if trigger_for_renewal_reminder.send(()).await.is_err() {
                                error!("Failed to send update trigger from renewal reminder task.");
                            }
                        },
                        Ok(_) => debug!("No new renewal reminders generated at this time."),
                        Err(e) => error!(error = %e, "Error checking/generating renewal reminders."),
                    }
                },
                _ = renewal_shutdown_rx.changed() => {
                    info!("Renewal reminder check task shutting down.");
                    break;
                }
            }
        }
    });

    // --- Automatic Renewal Processing Task ---
    let trigger_for_auto_renewal = update_trigger_tx.clone();
    const AUTO_RENEWAL_CHECK_INTERVAL_SECONDS: u64 = 6 * 60 * 60;
    let mut auto_renewal_shutdown_rx = shutdown_rx.clone();
    tokio::spawn(async move {
        let mut interval = interval(Duration::from_secs(AUTO_RENEWAL_CHECK_INTERVAL_SECONDS));
        info!(interval_seconds = AUTO_RENEWAL_CHECK_INTERVAL_SECONDS, "Automatic renewal processing task started.");
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    info!("Performing scheduled automatic renewal processing...");
                    match duckdb_service::vps_renewal_service::process_all_automatic_renewals(duckdb_pool.clone()).await {
                        Ok(renewed_count) if renewed_count > 0 => {
                            info!(count = renewed_count, "VPS were automatically renewed. Triggering state update.");
                            if trigger_for_auto_renewal.send(()).await.is_err() {
                                error!("Failed to send update trigger from automatic renewal task.");
                            }
                        },
                        Ok(_) => debug!("No VPS were automatically renewed at this time."),
                        Err(e) => error!(error = %e, "Error processing automatic renewals."),
                    }
                },
                _ = auto_renewal_shutdown_rx.changed() => {
                    info!("Automatic renewal processing task shutting down.");
                    break;
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
            async move {
                if req.headers().get("content-type").map(|v| v.as_bytes().starts_with(b"application/grpc")).unwrap_or(false) {
                    grpc_service.call(req).await.map(|res| res.map(axum::body::Body::new)).map_err(|err| match err {})
                } else {
                    static_file_service.call(req).await.map(|res| res.map(axum::body::Body::new)).map_err(|err| match err {})
                }
            }
        },
    ));

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(async move {
            shutdown_rx.changed().await.ok();
            info!("Graceful shutdown signal received. Axum server is shutting down.");
        })
        .await
        .map_err(Box::new)?;

    // Wait for tasks to complete
    let _ = tokio::try_join!(debouncer_task, evaluation_task, duckdb_task_handle);
 
    Ok(())
}
