use std::error::Error;
use backend::agent_service::agent_communication_service_client::AgentCommunicationServiceClient;
use serde::Deserialize; // Added for TOML parsing
use std::fs; // Added for reading config file
use backend::agent_service::message_to_server::Payload;
use backend::agent_service::{
    AgentConfig, AgentHandshake, Heartbeat, MessageToAgent, MessageToServer, NetworkInterfaceStats,
    OsType, PerformanceSnapshot, PerformanceSnapshotBatch,
};
use std::sync::Arc;
use std::time::Duration;
use sysinfo::{System, Disks, Networks, DiskRefreshKind};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use uuid::Uuid;

// Structure for agent configuration from TOML file
#[derive(Deserialize, Debug, Clone)]
struct AgentCliConfig {
    server_address: String,
    vps_id: i32,
    agent_secret: String,
}

// Helper function to get next client_message_id
async fn get_next_client_message_id(counter: &Arc<Mutex<u64>>) -> u64 {
    let mut num = counter.lock().await;
    let id = *num;
    *num += 1;
    id
}

fn collect_performance_snapshot(sys: &mut System) -> PerformanceSnapshot {
    sys.refresh_all();

    let cpu_usage = sys.global_cpu_usage();
    let mem_used = sys.used_memory();
    let mem_total = sys.total_memory();

    // Cumulative Disk I/O
    let mut total_disk_read_bytes: u64 = 0;
    let mut total_disk_write_bytes: u64 = 0;
    let mut disks = Disks::new_with_refreshed_list();
    for disk in disks.list_mut() {
        disk.refresh_specifics(DiskRefreshKind::everything());
        let disk_usage = disk.usage();
        total_disk_read_bytes += disk_usage.total_read_bytes;
        total_disk_write_bytes += disk_usage.total_written_bytes;
    }

    // Network I/O per interface (cumulative)
    let mut network_stats_list = Vec::new();
    let networks = Networks::new_with_refreshed_list();
    for (if_name, data) in networks.iter() {
        network_stats_list.push(NetworkInterfaceStats {
            interface_name: if_name.clone(),
            rx_bytes_per_sec: data.received(), // CUMULATIVE
            tx_bytes_per_sec: data.transmitted(), // CUMULATIVE
            rx_packets_per_sec: data.packets_received(),
            tx_packets_per_sec: data.packets_transmitted(),
            rx_errors_total_cumulative: data.errors_on_received(),
            tx_errors_total_cumulative: data.errors_on_transmitted(),
        });
    }

    PerformanceSnapshot {
        timestamp_unix_ms: chrono::Utc::now().timestamp_millis(),
        cpu_overall_usage_percent: cpu_usage,
        memory_usage_bytes: mem_used,
        memory_total_bytes: mem_total,
        swap_usage_bytes: sys.used_swap(),
        swap_total_bytes: sys.total_swap(),
        disk_total_io_read_bytes_per_sec: total_disk_read_bytes, // CUMULATIVE
        disk_total_io_write_bytes_per_sec: total_disk_write_bytes, // CUMULATIVE
        disk_usages: Vec::new(), // Not part of core metrics per plan
        network_interface_stats: network_stats_list,
        load_average_one_min: System::load_average().one as f32,
        load_average_five_min: System::load_average().five as f32,
        load_average_fifteen_min: System::load_average().fifteen as f32,
        uptime_seconds: System::uptime(),
        total_processes_count: sys.processes().len() as u32,
        running_processes_count: 0, // Placeholder, sysinfo does not directly provide this easily
        tcp_established_connection_count: 0, // Placeholder, requires specific parsing or library
    }
}

async fn metrics_collection_loop(
    tx_to_server: mpsc::Sender<MessageToServer>,
    agent_config: AgentConfig,
    agent_id: String,
    client_message_id_counter: Arc<Mutex<u64>>,
    vps_db_id: i32, // Added
    agent_secret: String, // Added
) {
    let mut sys = System::new_all(); // System instance for this task
    let mut collect_interval_duration = agent_config.metrics_collect_interval_seconds;
    if collect_interval_duration == 0 { collect_interval_duration = 60; } // Default to 60s if 0
    let mut collect_interval = tokio::time::interval(Duration::from_secs(collect_interval_duration as u64));

    let mut upload_interval_duration = agent_config.metrics_upload_interval_seconds;
    if upload_interval_duration == 0 { upload_interval_duration = 60; } // Default to 60s if 0
    let mut upload_interval = tokio::time::interval(Duration::from_secs(upload_interval_duration as u64));
    
    let mut batch_max_size = agent_config.metrics_upload_batch_max_size;
    if batch_max_size == 0 { batch_max_size = 10; } // Default to 10 if 0

    let mut snapshot_batch_vec = Vec::new();
    println!("[Agent:{}] Metrics collection task started. Collect interval: {}s, Upload interval: {}s, Batch size: {}",
        agent_id, collect_interval_duration, upload_interval_duration, batch_max_size);

    loop {
        tokio::select! {
            _ = collect_interval.tick() => {
                let snapshot = collect_performance_snapshot(&mut sys);
                snapshot_batch_vec.push(snapshot);
                // println!("[Agent:{}] Collected metrics snapshot. Batch size: {}", agent_id, snapshot_batch_vec.len());

                if snapshot_batch_vec.len() >= batch_max_size as usize {
                    let batch_to_send_vec = std::mem::take(&mut snapshot_batch_vec);
                    if !batch_to_send_vec.is_empty() {
                        let batch_len = batch_to_send_vec.len();
                        let batch_payload = PerformanceSnapshotBatch { snapshots: batch_to_send_vec };
                        let msg_id = get_next_client_message_id(&client_message_id_counter).await;
                        if let Err(e) = tx_to_server.send(MessageToServer {
                            client_message_id: msg_id,
                            payload: Some(Payload::PerformanceBatch(batch_payload)),
                            vps_db_id,
                            agent_secret: agent_secret.clone(),
                        }).await {
                            eprintln!("[Agent:{}] Failed to send metrics batch (size trigger): {}", agent_id, e);
                        } else {
                            println!("[Agent:{}] Sent metrics batch (size trigger). Msg ID: {}. Actual batch size: {}", agent_id, msg_id, batch_len);
                        }
                    }
                }
            }
            _ = upload_interval.tick() => {
                let batch_to_send_vec = std::mem::take(&mut snapshot_batch_vec);
                if !batch_to_send_vec.is_empty() {
                    let batch_len = batch_to_send_vec.len(); // Get length BEFORE move
                    let batch_payload = PerformanceSnapshotBatch { snapshots: batch_to_send_vec }; // Move happens here
                    let msg_id = get_next_client_message_id(&client_message_id_counter).await;
                     if let Err(e) = tx_to_server.send(MessageToServer {
                        client_message_id: msg_id,
                        payload: Some(Payload::PerformanceBatch(batch_payload)), // batch_payload is moved here
                        vps_db_id,
                        agent_secret: agent_secret.clone(),
                    }).await {
                        eprintln!("[Agent:{}] Failed to send metrics batch (interval trigger): {}", agent_id, e);
                    } else {
                        println!("[Agent:{}] Sent metrics batch (interval trigger). Msg ID: {}. Actual batch size: {}", agent_id, msg_id, batch_len); // Use batch_len
                    }
                }
            }
            // TODO: Add a way to receive config updates and adjust intervals/batch_size
        }
    }
}

async fn heartbeat_loop(
    tx_to_server: mpsc::Sender<MessageToServer>,
    agent_config: AgentConfig,
    agent_id: String,
    client_message_id_counter: Arc<Mutex<u64>>,
    vps_db_id: i32, // Added
    agent_secret: String, // Added
) {
    let mut interval_duration = agent_config.heartbeat_interval_seconds;
    if interval_duration == 0 { interval_duration = 30; } // Default to 30s if 0
    let mut interval = tokio::time::interval(Duration::from_secs(interval_duration as u64));
    println!("[Agent:{}] Heartbeat task started. Interval: {}s", agent_id, interval_duration);

    loop {
        interval.tick().await;
        let heartbeat_payload = Heartbeat {
            timestamp_unix_ms: chrono::Utc::now().timestamp_millis(),
        };
        let msg_id = get_next_client_message_id(&client_message_id_counter).await;
        if let Err(e) = tx_to_server.send(MessageToServer {
            client_message_id: msg_id,
            payload: Some(Payload::Heartbeat(heartbeat_payload)),
            vps_db_id,
            agent_secret: agent_secret.clone(),
        }).await {
            eprintln!("[Agent:{}] Failed to send heartbeat: {}. Exiting heartbeat task.", agent_id, e);
            break;
        } else {
            // println!("[Agent:{}] Heartbeat sent. Msg ID: {}", agent_id, msg_id);
        }
    }
}

async fn server_message_handler_loop(
    mut in_stream: tonic::Streaming<MessageToAgent>,
    agent_id: String,
    // TODO: Add Arc<Mutex<AgentConfig>> to update config for other tasks
) {
    println!("[Agent:{}] Listening for messages from server...", agent_id);
    while let Some(message_result) = in_stream.next().await {
        match message_result {
            Ok(message_to_agent) => {
                // println!("[Agent:{}] Received message from server. ID: {}", agent_id, message_to_agent.server_message_id);
                if let Some(payload) = message_to_agent.payload {
                    match payload {
                        backend::agent_service::message_to_agent::Payload::AgentConfig(new_config) => {
                            println!("[Agent:{}] Received new AgentConfig from server: {:?}", agent_id, new_config);
                            // TODO: Implement dynamic config update logic
                        }
                        backend::agent_service::message_to_agent::Payload::CommandRequest(cmd_req) => {
                            println!("[Agent:{}] Received CommandRequest: {:?}", agent_id, cmd_req);
                            // TODO: Implement command execution
                        }
                        _ => {
                            // println!("[Agent:{}] Received unhandled payload type from server: {:?}", agent_id, payload);
                        }
                    }
                }
            }
            Err(status) => {
                eprintln!("[Agent:{}] Error receiving message from server: {}. Stream broken.", agent_id, status);
                break;
            }
        }
    }
    println!("[Agent:{}] Server message stream ended.", agent_id);
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load configuration from agent_config.toml
    // Expects agent_config.toml in the same directory as the executable, or a predefined path.
    // For simplicity, let's try to read from "agent_config.toml" in the current directory.
    let current_dir = std::env::current_dir()
        .map_err(|e| format!("Failed to get current directory: {}", e))?;
    println!("[Agent] Current directory: {:?}", current_dir);
    let config_path = "agent_config.toml";
    let config_str = fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read agent config file '{}': {}", config_path, e))?;
    
    let agent_cli_config: AgentCliConfig = toml::from_str(&config_str)
        .map_err(|e| format!("Failed to parse agent config file '{}': {}", config_path, e))?;

    println!("[Agent] Loaded config: {:?}", agent_cli_config);
    println!("[Agent] Connecting to server: {}", agent_cli_config.server_address);

    let mut client = AgentCommunicationServiceClient::connect(agent_cli_config.server_address.clone()).await
        .map_err(|e| Box::new(e))?;

    let (tx_to_server, rx_for_stream) = mpsc::channel(128);
    let response_stream = client.establish_communication_stream(ReceiverStream::new(rx_for_stream)).await
        .map_err(|e| Box::new(e))?;
    let mut in_stream = response_stream.into_inner();
    
    // sys_for_handshake is not strictly needed to be mutable or refreshed for System::name() and System::host_name()
    // as those are static methods on System or available after System::new().
    // let mut sys_for_handshake = System::new();

    let os_type_proto = if cfg!(target_os = "linux") {
        OsType::Linux
    } else if cfg!(target_os = "macos") {
        OsType::Macos
    } else if cfg!(target_os = "windows") {
        OsType::Windows
    } else {
        OsType::default() // Use default for UNKNOWN_OS
    };
    
    let handshake_payload = AgentHandshake {
        agent_id_hint: Uuid::new_v4().to_string(),
        agent_version: env!("CARGO_PKG_VERSION").to_string(),
        os_type: i32::from(os_type_proto),
        os_name: System::name().unwrap_or_else(|| "Unknown".to_string()),
        arch: std::env::consts::ARCH.to_string(),
        hostname: System::host_name().unwrap_or_else(|| "Unknown".to_string()),
    };
    
    tx_to_server.send(MessageToServer {
        client_message_id: 1, // Dedicated ID for handshake
        payload: Some(Payload::AgentHandshake(handshake_payload)),
        vps_db_id: agent_cli_config.vps_id,
        agent_secret: agent_cli_config.agent_secret.clone(),
    }).await.map_err(|e| Box::new(e) as Box<dyn Error>)?;
    
    let (assigned_agent_id, initial_agent_config) = match in_stream.next().await {
        Some(Ok(response_msg)) => {
            if let Some(backend::agent_service::message_to_agent::Payload::ServerHandshakeAck(ack)) = response_msg.payload {
                if ack.authentication_successful {
                    println!("[Agent:{}] Authenticated successfully.", ack.assigned_agent_id);
                    (ack.assigned_agent_id, ack.initial_config.unwrap_or_default())
                } else {
                    eprintln!("[Agent] Authentication failed: {}", ack.error_message);
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::PermissionDenied, ack.error_message) ) as Box<dyn Error>);
                }
            } else {
                eprintln!("[Agent] Unexpected first message from server (not HandshakeAck).");
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Unexpected handshake response")));
            }
        }
        Some(Err(status)) => {
            eprintln!("[Agent] Error receiving handshake response: {}", status);
            return Err(Box::new(status) as Box<dyn Error>);
        }
        None => {
            eprintln!("[Agent] Server closed stream during handshake.");
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "Stream closed during handshake")));
        }
    };

    let client_message_id_counter = Arc::new(Mutex::new(2u64)); // Start after handshake ID 1

    let metrics_task_handle = tokio::spawn(metrics_collection_loop(
        tx_to_server.clone(),
        initial_agent_config.clone(),
        assigned_agent_id.clone(),
        client_message_id_counter.clone(),
        agent_cli_config.vps_id,
        agent_cli_config.agent_secret.clone(),
    ));

    let heartbeat_task_handle = tokio::spawn(heartbeat_loop(
        tx_to_server.clone(),
        initial_agent_config.clone(),
        assigned_agent_id.clone(),
        client_message_id_counter.clone(),
        agent_cli_config.vps_id,
        agent_cli_config.agent_secret.clone(),
    ));

    let server_listener_task_handle = tokio::spawn(server_message_handler_loop(
        in_stream, // in_stream is moved here
        assigned_agent_id.clone(),
    ));

    tokio::select! {
        res = metrics_task_handle => eprintln!("[Agent:{}] Metrics task ended: {:?}", assigned_agent_id, res),
        res = heartbeat_task_handle => eprintln!("[Agent:{}] Heartbeat task ended: {:?}", assigned_agent_id, res),
        res = server_listener_task_handle => eprintln!("[Agent:{}] Server listener task ended: {:?}", assigned_agent_id, res),
        // _ = tokio::signal::ctrl_c() => println!("[Agent:{}] Received Ctrl-C, initiating shutdown.", assigned_agent_id), // Commented out: Tokio 'signal' feature likely not enabled.
        // To enable, add `features = ["signal"]` to tokio in Cargo.toml.
        // For now, agent will run until one of the main tasks errors or stream closes.
    }
    
    println!("[Agent:{}] Shutting down.", assigned_agent_id);
    Ok(())
}
