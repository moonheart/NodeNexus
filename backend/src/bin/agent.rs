use std::error::Error;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::task::JoinHandle; // For task handles
// Added for gRPC server

// Correct approach:
// 1. In backend/src/lib.rs, add `pub mod agent_modules;`
// 2. Here, use `use backend::agent_modules::config::{load_cli_config, AgentCliConfig};` etc.

// Let's proceed assuming agent_modules are part of the backend crate library.
use backend::agent_modules::command::tracker::RunningCommandsTracker;
use backend::agent_modules::communication::{
    ConnectionHandler, server_message_handler_loop,
};
use backend::agent_modules::config::{AgentCliConfig, load_cli_config};
use backend::agent_modules::metrics::metrics_collection_loop;
use backend::agent_modules::service_monitor::ServiceMonitorManager;
// Removed: use backend::agent_modules::agent_command_service_impl::create_agent_command_service;
use backend::agent_service::AgentConfig;
use backend::version::VERSION;
use clap::{Parser, arg, command};
use tracing::{error, info, warn};
use tracing_appender::rolling;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(windows)]
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

#[cfg(windows)]
const SERVICE_NAME: &str = "NodeNexusAgent";
#[cfg(windows)]
define_windows_service!(ffi_service_main, service_main);

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long, default_value = "agent_config.toml")]
    config: String,
}

const INITIAL_CLIENT_MESSAGE_ID: AtomicU64 = AtomicU64::new(1);
const MAX_RECONNECT_DELAY_SECONDS: u64 = 60 * 5;
const DEFAULT_RECONNECT_DELAY_SECONDS: u64 = 5;

async fn spawn_and_monitor_core_tasks(
    handler: ConnectionHandler,
    agent_cli_config: &AgentCliConfig,
    shared_agent_config: Arc<RwLock<AgentConfig>>,
    command_tracker: Arc<RunningCommandsTracker>,
    update_lock: Arc<tokio::sync::Mutex<()>>,
) -> Vec<JoinHandle<()>> {
    let (
        in_stream,
        tx_to_server,
        client_message_id_counter, // This is an Arc<AtomicU64>
        _initial_agent_config, // No longer the source of truth, config is now in shared_agent_config
    ) = handler.split_for_tasks();

    let mut tasks = Vec::new();

    // Metrics Task
    let metrics_tx = tx_to_server.clone();
    let metrics_agent_config = Arc::clone(&shared_agent_config);
    let metrics_id_provider_counter = client_message_id_counter.clone();
    let metrics_vps_id = agent_cli_config.vps_id;
    let metrics_agent_secret = agent_cli_config.agent_secret.clone();
    // Get the closure for ID generation
    let metrics_id_provider =
        backend::agent_modules::communication::ConnectionHandler::get_id_provider_closure(
            metrics_id_provider_counter,
        );

    tasks.push(tokio::spawn(async move {
        metrics_collection_loop(
            metrics_tx,
            metrics_agent_config,
            metrics_id_provider, // Pass the closure
            metrics_vps_id,
            metrics_agent_secret,
        )
        .await;
        info!("Metrics collection loop ended.");
    }));


    // Server Listener Task
    let listener_tx = tx_to_server.clone();
    let listener_id_provider_counter = client_message_id_counter.clone();
    let listener_vps_id = agent_cli_config.vps_id;
    let listener_agent_secret = agent_cli_config.agent_secret.clone();
    let listener_id_provider =
        backend::agent_modules::communication::ConnectionHandler::get_id_provider_closure(
            listener_id_provider_counter,
        );
    let listener_agent_config = Arc::clone(&shared_agent_config);
    let listener_config_path = agent_cli_config.config_path.clone();
    let listener_command_tracker = command_tracker.clone(); // Clone command_tracker for the listener task
    let listener_update_lock = update_lock.clone();

    // Note: server_message_handler_loop takes ownership of in_stream
    tasks.push(tokio::spawn(async move {
        server_message_handler_loop(
            Box::pin(in_stream),
            listener_tx,
            listener_id_provider,
            listener_vps_id,
            listener_agent_secret,
            listener_agent_config,
            listener_config_path,
            listener_command_tracker, // Pass command_tracker
            listener_update_lock,
        )
        .await;
        info!("Server message handler loop ended.");
    }));

    // Service Monitor Task
    let monitor_tx = tx_to_server.clone();
    let monitor_agent_config = Arc::clone(&shared_agent_config);
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
        info!("Service monitor loop ended.");
    }));
    info!("All core tasks spawned.");
    tasks
}

fn init_logging() {
    // Get the directory of the executable
    let exe_path = std::env::current_exe().expect("Failed to get current exe path");
    let exe_dir = exe_path
        .parent()
        .expect("Failed to get parent directory of exe");
    let log_dir = exe_dir.join("logs");

    // Log to a file: JSON format, daily rotation
    let file_appender = rolling::daily(log_dir, "agent.log");
    let file_layer = fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false) // No ANSI colors in file
        .json(); // Log as JSON

    // Log to stdout: human-readable format
    let stdout_layer = fmt::layer().with_writer(std::io::stdout);

    // Combine layers and filter based on RUST_LOG
    // Default to `info` level if RUST_LOG is not set.
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(file_layer)
        .with(stdout_layer)
        .init();

    // This allows libraries using the `log` crate to work with `tracing`
    // tracing_log::LogTracer::init().expect("Failed to set logger");
}

#[cfg(windows)]
fn service_main(_arguments: Vec<std::ffi::OsString>) {
    if let Err(e) = std::panic::catch_unwind(run_service) {
        error!("Service panicked: {:?}", e);
    }
}

#[cfg(windows)]
fn run_service() {
    // Create a channel to be able to poll a stop event from the service worker loop.
    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::channel();

    // Define a handler for service events, primarily to handle stop requests.
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop => {
                info!("Received stop control event. Initiating shutdown...");
                shutdown_tx.send(()).unwrap();
                ServiceControlHandlerResult::NoError
            }
            // All other control events are ignored
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    // Register the handler with the service control manager.
    let status_handle = service_control_handler::register(SERVICE_NAME, event_handler).unwrap();

    // Tell the SCM that the service is starting
    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::StartPending,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::from_secs(5),
            process_id: None,
        })
        .unwrap();

    // IMPORTANT: Change the current working directory to the executable's directory
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            if let Err(e) = std::env::set_current_dir(exe_dir) {
                error!(error = %e, "Failed to set current directory to executable's directory. Relative paths may fail.");
            } else {
                info!(path = %exe_dir.display(), "Successfully set working directory.");
            }
        }
    }

    // Spawn a new thread to run the actual agent logic.
    let _agent_thread = std::thread::spawn(|| {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        if let Err(e) = runtime.block_on(run_agent_logic()) {
            error!(error = %e, "Agent logic returned an error. The service will stop.");
            // The service will stop naturally as this thread exits.
            // For a more robust solution, we could signal the main service thread
            // to report an error state to the SCM.
        }
    });

    // Tell the SCM that the service is now running.
    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::from_secs(0),
            process_id: None,
        })
        .unwrap();

    // Wait for the shutdown signal.
    shutdown_rx.recv().unwrap();
    info!("Shutdown signal received. Service is stopping.");

    // Tell the SCM that the service is stopping.
    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::StopPending,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 1,
            wait_hint: Duration::from_secs(5),
            process_id: None,
        })
        .unwrap();

    // Here you would ideally signal the agent_thread to shut down gracefully.
    // Since the main loop in run_agent_logic is infinite, we can't easily join it.
    // For a robust implementation, run_agent_logic would need a shutdown channel.
    // For now, we'll just let the service manager terminate the process.

    // Tell the SCM that the service has stopped.
    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::from_secs(0),
            process_id: None,
        })
        .unwrap();
}

fn main() {
    #[cfg(windows)]
    {
        // Attempt to run as a Windows service.
        // If `service_dispatcher::start` returns an error, it means we are not being run by the SCM.
        // In that case, we fall back to running as a console application.
        if service_dispatcher::start(SERVICE_NAME, ffi_service_main).is_err() {
            info!("Not running as a service, starting in console mode.");
            run_console_mode();
        }
    }

    #[cfg(not(windows))]
    {
        // On non-Windows platforms, always run in console mode.
        run_console_mode();
    }
}

/// Runs the agent logic as a standalone console application.
fn run_console_mode() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            if let Err(e) = run_agent_logic().await {
                error!(error = %e, "An error occurred in console mode.");
            }
        });
}

async fn run_agent_logic() -> Result<(), Box<dyn Error + Send + Sync>> {
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--version".to_string()) {
        println!("Agent version: {VERSION}");
        return Ok(());
    }
    // --- Health Check Argument Handling ---
    if args.contains(&"--health-check".to_string()) {
        // This is a very basic health check. A real one might try to load config,
        // or even briefly connect to the server. For now, just exiting successfully
        // proves the binary is executable and not corrupt.
        println!("Health check successful.");
        std::process::exit(0);
    }

    let cli_args = Args::parse();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install default crypto provider");
    init_logging();
    info!(version = VERSION, "Starting agent...");

    let agent_cli_config = match load_cli_config(&cli_args.config) {
        Ok(mut config) => {
            config.config_path = cli_args.config; // Store the config path
            config
        }
        Err(e) => {
            let err_msg = format!("Critical error loading configuration: {e}");
            error!(error = %err_msg);
            // Convert the error into a type that satisfies the Send + Sync bounds.
            return Err(err_msg.into());
        }
    };

    // Create RunningCommandsTracker here, to be passed to tasks
    let command_tracker = Arc::new(RunningCommandsTracker::new());
    let update_lock = Arc::new(tokio::sync::Mutex::new(()));

    // --- Removed setup for Agent's own gRPC Command Service ---
    // The agent will handle commands received over the main communication stream.

    let mut reconnect_delay_seconds = DEFAULT_RECONNECT_DELAY_SECONDS;

    // Main loop for connecting to the primary server
    loop {
        info!(
            server_address = %agent_cli_config.server_address,
            delay_seconds = reconnect_delay_seconds,
            "Main client loop: Attempting connection to server."
        );

        // Attempt to connect and handshake (client role)
        // Load the initial value from AtomicU64
        let initial_id = INITIAL_CLIENT_MESSAGE_ID.load(std::sync::atomic::Ordering::SeqCst);
        match ConnectionHandler::connect_and_handshake(&agent_cli_config, initial_id).await {
            Ok(handler) => {
                info!("Connection and handshake successful. Spawning tasks.");
                // Log the received config
                info!(config = ?handler.initial_agent_config, "Received initial config from server.");
                reconnect_delay_seconds = DEFAULT_RECONNECT_DELAY_SECONDS; // Reset delay on successful connection

                // Create the shared, mutable configuration state
                let shared_agent_config =
                    Arc::new(RwLock::new(handler.initial_agent_config.clone()));
                // Pass command_tracker to spawn_and_monitor_core_tasks
                let task_handles = spawn_and_monitor_core_tasks(
                    handler,
                    &agent_cli_config,
                    shared_agent_config,
                    command_tracker.clone(),
                    update_lock.clone(),
                )
                .await;

                // Monitor tasks. If any of them exit, it signifies a problem, and we should attempt to reconnect.
                if !task_handles.is_empty() {
                    // futures::future::select_all waits for the first task to complete.
                    // The result includes the completed task's output, its index, and the remaining futures.
                    let (first_task_result, _index, remaining_handles) =
                        futures::future::select_all(task_handles).await;

                    match first_task_result {
                        Ok(_) => {
                            // Task completed without panic
                            // This is expected if a task like heartbeat_loop or server_message_handler_loop exits due to error (e.g., send fail, stream break)
                            warn!("A core task finished. This usually indicates a connection issue or an internal task error.");
                        }
                        Err(join_error) => {
                            // Task panicked
                            error!(error = ?join_error, "A core task panicked. This is a critical issue.");
                            // Depending on the error, might want a longer backoff or specific error handling.
                        }
                    }

                    // Abort all other running tasks to ensure a clean state before reconnecting.
                    info!("Aborting remaining tasks before reconnecting...");
                    for handle in remaining_handles {
                        handle.abort();
                    }
                } else {
                    error!("No tasks were spawned, which is unexpected after successful handshake. This should not happen.");
                    // This case implies an issue in spawn_and_monitor_core_tasks or ConnectionHandler::split_for_tasks
                }

                warn!("A task ended or an issue occurred. Preparing to reconnect...");
            }
            Err(e) => {
                error!(error = %e, "Failed to connect or handshake. Will retry.");
                // Error already logged by connect_and_handshake or load_cli_config
            }
        }

        // Exponential backoff for retrying connection
        info!(
            delay_seconds = reconnect_delay_seconds,
            "Sleeping before next connection attempt."
        );
        tokio::time::sleep(Duration::from_secs(reconnect_delay_seconds)).await;
        reconnect_delay_seconds = (reconnect_delay_seconds * 2).min(MAX_RECONNECT_DELAY_SECONDS);
    }
    // Loop is infinite, so Ok(()) is effectively unreachable but satisfies function signature.
    // For a real application, you might have a shutdown signal (e.g., Ctrl-C handler)
    // that breaks the loop and allows for graceful termination.
    // The agent_grpc_server_handle was removed as the agent no longer hosts its own gRPC service.
}
