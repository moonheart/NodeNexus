use crate::agent_service::{
    AgentConfig, MessageToServer, PerformanceSnapshot, PerformanceSnapshotBatch,
    message_to_server::Payload,
    // NetworkInterfaceStats, // No longer needed
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
// Removed NetworkData import as we only need Networks iterator now
use sysinfo::{Disks, Networks, ProcessRefreshKind, System};
use netdev; // Added for getting default network interface
use tokio::sync::mpsc;

// Note: get_next_client_message_id will be handled by the communication module or passed appropriately.

/// Collects a performance snapshot including cumulative network data for the first interface found.
pub fn collect_performance_snapshot(sys: &mut System) -> PerformanceSnapshot {
    // Refresh relevant parts of System
    sys.refresh_cpu_all();
    sys.refresh_memory();
    sys.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, ProcessRefreshKind::nothing()); // For process count

    let cpu_usage = sys.global_cpu_usage();
    let mem_used = sys.used_memory();
    let mem_total = sys.total_memory();

    // Cumulative Disk I/O
    let mut total_disk_read_bytes: u64 = 0;
    let mut total_disk_write_bytes: u64 = 0;
    // Refresh disks for I/O counters
    // Creating Disks list each time. Consider optimizing if performance critical.
    let mut disks = Disks::new_with_refreshed_list();
    disks.refresh(true);
    for disk in disks.list() {
        let disk_usage = disk.usage();
        total_disk_read_bytes += disk_usage.total_read_bytes;
        total_disk_write_bytes += disk_usage.total_written_bytes;
    }

    // --- Network I/O for Default Interface (Cumulative) ---
    let networks = Networks::new_with_refreshed_list(); // Get sysinfo network list
    let mut cumulative_rx_bytes: u64 = 0;
    let mut cumulative_tx_bytes: u64 = 0;

    // Try to get the default interface using netdev
    match netdev::get_default_interface() {
        Ok(default_interface) => {
            let default_if_name = default_interface.name;
            let mut found_default = false;
            // Find the default interface in sysinfo's list
            for (if_name, data) in networks.iter() {
                // Compare &String with &str
                if if_name == default_if_name.as_str() {
                    cumulative_rx_bytes = data.received();
                    cumulative_tx_bytes = data.transmitted();
                    found_default = true;
                    // println!("Using default interface '{}' for network stats.", default_if_name); // Debug
                    break;
                }
            }
            if !found_default {
                eprintln!("Warning: Default interface '{}' found by netdev, but not found in sysinfo's network list. Falling back to first interface.", default_if_name);
                // Fallback to first interface if default wasn't in sysinfo list
                if let Some((_if_name, data)) = networks.iter().next() {
                    cumulative_rx_bytes = data.received();
                    cumulative_tx_bytes = data.transmitted();
                }
            }
        }
        Err(e) => {
            eprintln!("Warning: Failed to get default network interface using netdev: {}. Falling back to first interface.", e);
            // Fallback to first interface if netdev failed
            if let Some((_if_name, data)) = networks.iter().next() {
                cumulative_rx_bytes = data.received();
                cumulative_tx_bytes = data.transmitted();
            }
        }
    }
    // --- End Network I/O ---

    PerformanceSnapshot {
        timestamp_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64,
        cpu_overall_usage_percent: cpu_usage,
        memory_usage_bytes: mem_used,
        memory_total_bytes: mem_total,
        swap_usage_bytes: sys.used_swap(),
        swap_total_bytes: sys.total_swap(),
        // These disk IO fields represent CUMULATIVE bytes.
        disk_total_io_read_bytes_per_sec: total_disk_read_bytes, // CUMULATIVE
        disk_total_io_write_bytes_per_sec: total_disk_write_bytes, // CUMULATIVE
        disk_usages: Vec::new(), // Not part of core metrics per plan
        // Assign to the cumulative fields as per the final proto definition
        network_rx_bytes_cumulative: cumulative_rx_bytes, // Field 10 (Using default or fallback)
        network_tx_bytes_cumulative: cumulative_tx_bytes, // Field 11 (Using default or fallback)
        // Renumbered fields according to user edits in proto
        load_average_one_min: System::load_average().one as f32, // Field 12
        load_average_five_min: System::load_average().five as f32, // Field 13
        load_average_fifteen_min: System::load_average().fifteen as f32, // Field 14
        uptime_seconds: System::uptime(), // Field 15
        total_processes_count: sys.processes().len() as u32, // Field 16
        running_processes_count: 0, // Placeholder, Field 17
        tcp_established_connection_count: 0, // Placeholder, Field 18
    }
}


pub async fn metrics_collection_loop(
    tx_to_server: mpsc::Sender<MessageToServer>,
    agent_config: AgentConfig,
    agent_id: String,
    mut id_provider: impl FnMut() -> u64 + Send + 'static,
    vps_db_id: i32,
    agent_secret: String,
) {
    let mut sys = System::new_all();
    // No longer need to manage Networks state here

    let mut collect_interval_duration = agent_config.metrics_collect_interval_seconds;
    if collect_interval_duration == 0 { collect_interval_duration = 60; } // Default collect interval
    let mut collect_interval = tokio::time::interval(Duration::from_secs(collect_interval_duration as u64));

    let mut upload_interval_duration = agent_config.metrics_upload_interval_seconds;
    if upload_interval_duration == 0 { upload_interval_duration = 60; } // Default upload interval
    let mut upload_interval = tokio::time::interval(Duration::from_secs(upload_interval_duration as u64));

    let mut batch_max_size = agent_config.metrics_upload_batch_max_size;
    if batch_max_size == 0 { batch_max_size = 10; } // Default batch size

    let mut snapshot_batch_vec = Vec::new();

    println!("[Agent:{}] Metrics collection task started. Collect interval: {}s, Upload interval: {}s, Batch size: {}",
        agent_id, collect_interval_duration, upload_interval_duration, batch_max_size);

    loop {
        tokio::select! {
            _ = collect_interval.tick() => {
                // Call snapshot function without networks argument
                let snapshot = collect_performance_snapshot(&mut sys);
                snapshot_batch_vec.push(snapshot);

                // Check if batch needs to be sent due to size trigger
                if snapshot_batch_vec.len() >= batch_max_size as usize {
                    let batch_to_send_vec = std::mem::take(&mut snapshot_batch_vec);
                    if !batch_to_send_vec.is_empty() {
                        let batch_len = batch_to_send_vec.len();
                        let batch_payload = PerformanceSnapshotBatch { snapshots: batch_to_send_vec };
                        let msg_id = id_provider();
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
                // Send whatever is in the batch due to interval trigger
                let batch_to_send_vec = std::mem::take(&mut snapshot_batch_vec);
                if !batch_to_send_vec.is_empty() {
                    let batch_len = batch_to_send_vec.len();
                    let batch_payload = PerformanceSnapshotBatch { snapshots: batch_to_send_vec };
                    let msg_id = id_provider();
                     if let Err(e) = tx_to_server.send(MessageToServer {
                        client_message_id: msg_id,
                        payload: Some(Payload::PerformanceBatch(batch_payload)),
                        vps_db_id,
                        agent_secret: agent_secret.clone(),
                    }).await {
                        eprintln!("[Agent:{}] Failed to send metrics batch (interval trigger): {}", agent_id, e);
                    } else {
                        println!("[Agent:{}] Sent metrics batch (interval trigger). Msg ID: {}. Actual batch size: {}", agent_id, msg_id, batch_len);
                    }
                }
            }
            // TODO: Add a way to receive config updates and adjust intervals/batch_size
        }
    }
}