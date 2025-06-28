//! Agent-side module for managing and executing service monitoring tasks.
use rand::random;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info};

use crate::agent_service::{
    AgentConfig, MessageToServer, ServiceMonitorResult, ServiceMonitorTask,
    message_to_server::Payload as ServerPayload,
};

/// Manages the lifecycle of all service monitoring tasks on the agent.
pub struct ServiceMonitorManager {
    // A map from monitor_id to its running task handle and its configuration.
    running_tasks: HashMap<i32, (JoinHandle<()>, ServiceMonitorTask)>,
}

impl Default for ServiceMonitorManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ServiceMonitorManager {
    pub fn new() -> Self {
        Self {
            running_tasks: HashMap::new(),
        }
    }

    /// The main reconciliation loop for service monitors.
    ///
    /// This function continuously checks the agent's configuration and adjusts
    /// the running monitoring tasks to match the desired state. It handles
    /// creation, deletion, and updates of monitoring tasks.
    pub async fn service_monitor_loop<F>(
        &mut self,
        shared_agent_config: Arc<RwLock<AgentConfig>>,
        tx_to_server: mpsc::Sender<MessageToServer>,
        vps_db_id: i32,
        agent_secret: String,
        id_provider: F,
    ) where
        F: Fn() -> u64 + Send + Sync + Clone + 'static,
    {
        loop {
            // Periodically check for config changes.
            tokio::time::sleep(Duration::from_secs(5)).await;

            let desired_tasks_map: HashMap<i32, ServiceMonitorTask> = {
                let config_guard = shared_agent_config.read().unwrap();
                config_guard
                    .service_monitor_tasks
                    .iter()
                    .map(|t| (t.monitor_id, t.clone()))
                    .collect()
            };

            let running_ids: HashSet<i32> = self.running_tasks.keys().cloned().collect();
            let desired_ids: HashSet<i32> = desired_tasks_map.keys().cloned().collect();

            // 1. Stop tasks that are no longer in the desired configuration
            for monitor_id in running_ids.difference(&desired_ids) {
                if let Some((handle, _)) = self.running_tasks.remove(monitor_id) {
                    info!(monitor_id = monitor_id, "Stopping task for monitor.");
                    handle.abort();
                }
            }

            // 2. Check for new tasks and updates to existing tasks
            for (monitor_id, desired_task) in desired_tasks_map {
                if let Some((existing_handle, existing_task)) =
                    self.running_tasks.get_mut(&monitor_id)
                {
                    // Task exists, check if it needs an update.
                    // The ServiceMonitorTask struct from protobuf doesn't derive PartialEq,
                    // so we compare relevant fields manually.
                    if existing_task.monitor_type != desired_task.monitor_type
                        || existing_task.target != desired_task.target
                        || existing_task.frequency_seconds != desired_task.frequency_seconds
                        || existing_task.timeout_seconds != desired_task.timeout_seconds
                        || existing_task.monitor_config_json != desired_task.monitor_config_json
                    {
                        info!(monitor_id = monitor_id, "Updating task for monitor.");
                        existing_handle.abort(); // Stop the old task

                        // Spawn the new task with updated config
                        let (new_handle, _) = spawn_checker_task(
                            desired_task.clone(),
                            tx_to_server.clone(),
                            vps_db_id,
                            agent_secret.clone(),
                            id_provider.clone(),
                        );
                        // Replace the old entry
                        self.running_tasks
                            .insert(monitor_id, (new_handle, desired_task));
                    }
                } else {
                    // Task is new, start it.
                    info!(monitor_id = monitor_id, "Starting new task for monitor.");
                    let (new_handle, _) = spawn_checker_task(
                        desired_task.clone(),
                        tx_to_server.clone(),
                        vps_db_id,
                        agent_secret.clone(),
                        id_provider.clone(),
                    );
                    self.running_tasks
                        .insert(monitor_id, (new_handle, desired_task));
                }
            }
        }
    }
}

/// Spawns a specific checker task based on the monitor type.
fn spawn_checker_task<F>(
    task: ServiceMonitorTask,
    tx_to_server: mpsc::Sender<MessageToServer>,
    vps_db_id: i32,
    agent_secret: String,
    id_provider: F,
) -> (JoinHandle<()>, i32)
where
    F: Fn() -> u64 + Send + Sync + Clone + 'static,
{
    let monitor_id = task.monitor_id;
    let handle = tokio::spawn(async move {
        info!("Started checker task.");
        let id_provider_clone = id_provider.clone();
        match task.monitor_type.as_str() {
            "http" | "https" => {
                run_http_check(
                    task,
                    tx_to_server,
                    vps_db_id,
                    agent_secret,
                    id_provider_clone,
                )
                .await
            }
            "ping" => {
                run_ping_check(
                    task,
                    tx_to_server,
                    vps_db_id,
                    agent_secret,
                    id_provider_clone,
                )
                .await
            }
            "tcp" => {
                run_tcp_check(
                    task,
                    tx_to_server,
                    vps_db_id,
                    agent_secret,
                    id_provider_clone,
                )
                .await
            }
            _ => {
                error!("Unknown monitor type. Task will not run.");
            }
        }
        info!("Checker task finished.");
    });
    (handle, monitor_id)
}

// --- Placeholder Implementations for Checkers ---
async fn run_http_check<F>(
    task: ServiceMonitorTask,
    tx: mpsc::Sender<MessageToServer>,
    vps_db_id: i32,
    agent_secret: String,
    id_provider: F,
) where
    F: Fn() -> u64 + Send + Sync + 'static,
{
    let interval = Duration::from_secs(task.frequency_seconds.max(1) as u64);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(task.timeout_seconds.max(1) as u64))
        .build()
        .unwrap(); // Should not fail with default settings

    loop {
        let start_time = Instant::now();
        let result = client.get(&task.target).send().await;
        let response_time_ms = start_time.elapsed().as_millis() as i32;

        let (successful, details, latency) = match result {
            Ok(response) => {
                let status = response.status();
                let details_str = status.to_string();
                (status.is_success(), details_str, Some(response_time_ms))
            }
            Err(e) => {
                let error_details = if e.is_timeout() {
                    "Error: Request timed out".to_string()
                } else {
                    format!("Error: {e}")
                };
                (false, error_details, None)
            }
        };

        let monitor_result = ServiceMonitorResult {
            monitor_id: task.monitor_id,
            timestamp_unix_ms: chrono::Utc::now().timestamp_millis(),
            successful,
            response_time_ms: latency,
            details,
        };

        let msg = MessageToServer {
            client_message_id: id_provider(),
            payload: Some(ServerPayload::ServiceMonitorResult(monitor_result)),
            vps_db_id,
            agent_secret: agent_secret.clone(),
        };

        if let Err(e) = tx.send(msg).await {
            error!(error = %e, "Failed to send result to server. Terminating task.");
            break;
        }

        tokio::time::sleep(interval).await;
    }
}

async fn run_ping_check<F: Fn() -> u64 + Send + Sync + 'static>(
    task: ServiceMonitorTask,
    tx: mpsc::Sender<MessageToServer>,
    vps_db_id: i32,
    agent_secret: String,
    id_provider: F,
) {
    let interval = Duration::from_secs(task.frequency_seconds.max(1) as u64);
    // Resolve the target, which could be a domain name or an IP address.
    let target_clone = task.target.clone();
    let resolved_addr_result = tokio::task::spawn_blocking(move || {
        use std::net::ToSocketAddrs;
        let host_with_port = format!("{target_clone}:0");
        host_with_port.to_socket_addrs()
    })
    .await;

    let target_addr = match resolved_addr_result {
        Ok(Ok(mut addrs)) => {
            if let Some(addr) = addrs.next() {
                addr.ip()
            } else {
                error!("DNS resolution returned no addresses. Terminating task.");
                return;
            }
        }
        _ => {
            error!("Failed to resolve target host. Terminating task.");
            return;
        }
    };

    let client = surge_ping::Client::new(&surge_ping::Config::default()).unwrap();

    loop {
        let mut pinger = client
            .pinger(target_addr, surge_ping::PingIdentifier(random()))
            .await;
        let start_time = Instant::now();
        let (successful, details, latency) =
            match pinger.ping(surge_ping::PingSequence(0), &[]).await {
                Ok((_reply, duration)) => {
                    let rtt = duration.as_millis() as i32;
                    (true, format!("{rtt} ms"), Some(rtt))
                }
                Err(e) => (false, format!("Error: {e}"), None),
            };

        let monitor_result = ServiceMonitorResult {
            monitor_id: task.monitor_id,
            timestamp_unix_ms: chrono::Utc::now().timestamp_millis(),
            successful,
            response_time_ms: latency,
            details,
        };

        let msg = MessageToServer {
            client_message_id: id_provider(),
            payload: Some(ServerPayload::ServiceMonitorResult(monitor_result)),
            vps_db_id,
            agent_secret: agent_secret.clone(),
        };

        if let Err(e) = tx.send(msg).await {
            error!(error = %e, "Failed to send result to server. Terminating task.");
            break;
        }

        tokio::time::sleep(interval).await;
    }
}

async fn run_tcp_check<F: Fn() -> u64 + Send + Sync + 'static>(
    task: ServiceMonitorTask,
    tx: mpsc::Sender<MessageToServer>,
    vps_db_id: i32,
    agent_secret: String,
    id_provider: F,
) {
    let interval = Duration::from_secs(task.frequency_seconds.max(1) as u64);
    let timeout_duration = Duration::from_secs(task.timeout_seconds.max(1) as u64);

    loop {
        let start_time = Instant::now();
        let result = tokio::time::timeout(
            timeout_duration,
            tokio::net::TcpStream::connect(&task.target),
        )
        .await;
        let response_time_ms = start_time.elapsed().as_millis() as i32;

        let (successful, details, latency) = match result {
            Ok(Ok(_stream)) => (
                true,
                "Connection successful".to_string(),
                Some(response_time_ms),
            ),
            Ok(Err(e)) => (false, format!("Error: {e}"), None),
            Err(_) => (false, "Error: Connection timed out".to_string(), None),
        };

        let monitor_result = ServiceMonitorResult {
            monitor_id: task.monitor_id,
            timestamp_unix_ms: chrono::Utc::now().timestamp_millis(),
            successful,
            response_time_ms: latency,
            details,
        };

        let msg = MessageToServer {
            client_message_id: id_provider(),
            payload: Some(ServerPayload::ServiceMonitorResult(monitor_result)),
            vps_db_id,
            agent_secret: agent_secret.clone(),
        };

        if let Err(e) = tx.send(msg).await {
            error!(error = %e, "Failed to send result to server. Terminating task.");
            break;
        }

        tokio::time::sleep(interval).await;
    }
}
