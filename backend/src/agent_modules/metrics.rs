use crate::agent_service::{
    AgentConfig, MessageToServer, PerformanceSnapshot, PerformanceSnapshotBatch,
    message_to_server::Payload,
};
use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sysinfo::{Disks, Networks, ProcessRefreshKind, System};
use netdev;
use tokio::sync::mpsc;

// Structure to hold previous network state for rate calculation
#[derive(Clone, Debug)]
struct PreviousNetworkState {
    time: Instant,
}

/// Collects a performance snapshot focusing ONLY on the default network interface
/// for both cumulative and instantaneous network data.
pub fn collect_performance_snapshot(
    sys: &mut System,
    networks: &mut Networks,
    prev_net_state: &Option<PreviousNetworkState>, // Pass previous state time
    current_time: Instant, // Pass current time for rate calculation
) -> PerformanceSnapshot {
    // Refresh relevant parts of System
    sys.refresh_cpu_all();
    sys.refresh_memory();
    sys.refresh_processes_specifics(sysinfo::ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());

    // Refresh the persistent Networks instance. This updates delta values.
    networks.refresh(true);

    let cpu_usage = sys.global_cpu_usage();
    let mem_used = sys.used_memory();
    let mem_total = sys.total_memory();

    // Cumulative Disk I/O (Remains the same - sums across all disks)
    let mut total_disk_read_bytes: u64 = 0;
    let mut total_disk_write_bytes: u64 = 0;
    let mut disks = Disks::new_with_refreshed_list();
    disks.refresh(true);
    for disk in disks.list() {
        let disk_usage = disk.usage();
        total_disk_read_bytes += disk_usage.total_read_bytes;
        total_disk_write_bytes += disk_usage.total_written_bytes;
    }

    // --- Network I/O (Default Interface Only) ---
    let mut cumulative_rx_bytes: u64 = 0;
    let mut cumulative_tx_bytes: u64 = 0;
    let mut delta_rx_bytes_for_rate: u64 = 0; // Delta used for BPS calculation
    let mut delta_tx_bytes_for_rate: u64 = 0; // Delta used for BPS calculation

    // Try to find the default interface and get its stats
    match netdev::get_default_interface() {
        Ok(default_interface) => {
            let default_if_name = default_interface.friendly_name.unwrap_or(default_interface.name);
            let mut found_default = false;
            // Find the default interface in the refreshed sysinfo networks list
            for (if_name, data) in networks.iter() {
                if if_name == default_if_name.as_str() {
                    // Get cumulative totals from the default interface
                    cumulative_rx_bytes = data.total_received();
                    cumulative_tx_bytes = data.total_transmitted();
                    // Get delta values from the default interface for rate calculation
                    delta_rx_bytes_for_rate = data.received();
                    delta_tx_bytes_for_rate = data.transmitted();
                    found_default = true;
                    // println!("[Debug] Using default interface '{}'. Cumulative RX: {}, TX: {}. Delta RX: {}, TX: {}",
                    //          if_name, cumulative_rx_bytes, cumulative_tx_bytes, delta_rx_bytes_for_rate, delta_tx_bytes_for_rate);
                    break;
                }
            }
            if !found_default {
                 eprintln!("Warning: Default interface '{}' found by netdev, but not in sysinfo list. Network stats will be 0.", default_if_name);
                 // Keep cumulative and delta values at 0
            }
        }
        Err(e) => {
            eprintln!("Warning: Failed to get default interface using netdev: {}. Network stats will be 0.", e);
            // Keep cumulative and delta values at 0
        }
    }

    // Calculate Instantaneous BPS using the default interface's delta (or 0 if not found)
    let mut network_rx_bps: u64 = 0;
    let mut network_tx_bps: u64 = 0;

    if let Some(prev_state) = prev_net_state {
        let duration = current_time.duration_since(prev_state.time);
        let duration_secs = duration.as_secs_f64();

        // Only calculate if delta is non-zero and duration is positive
        if duration_secs > 0.0 && (delta_rx_bytes_for_rate > 0 || delta_tx_bytes_for_rate > 0) {
            network_rx_bps = (delta_rx_bytes_for_rate as f64 / duration_secs) as u64;
            network_tx_bps = (delta_tx_bytes_for_rate as f64 / duration_secs) as u64;

            // --- Debugging Output ---
            // println!("[Debug] Time Delta: {:.2}s", duration_secs);
            // println!("[Debug] Using Delta RX for Rate: {}", delta_rx_bytes_for_rate);
            // println!("[Debug] Using Delta TX for Rate: {}", delta_tx_bytes_for_rate);
            // println!("[Debug] Calculated RX BPS: {}", network_rx_bps);
            // println!("[Debug] Calculated TX BPS: {}", network_tx_bps);
            // --- End Debugging ---
        } else if duration_secs <= 0.0 {
             // println!("[Debug] Duration is zero or negative, cannot calculate BPS.");
        } else {
             // println!("[Debug] Delta RX/TX is zero, BPS is 0.");
        }
    } else {
         // println!("[Debug] No previous network state, cannot calculate BPS for the first snapshot.");
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
        disk_total_io_read_bytes_per_sec: total_disk_read_bytes, // CUMULATIVE (All Disks)
        disk_total_io_write_bytes_per_sec: total_disk_write_bytes, // CUMULATIVE (All Disks)
        disk_usages: Vec::new(),
        // Cumulative network data (Default Interface Only)
        network_rx_bytes_cumulative: cumulative_rx_bytes, // Field 10
        network_tx_bytes_cumulative: cumulative_tx_bytes, // Field 11
        // Load averages removed
        // Removed load_average fields
        uptime_seconds: System::uptime(), // Renumbered field 12
        total_processes_count: sys.processes().len() as u32, // Renumbered field 13
        running_processes_count: 0, // Placeholder, Renumbered field 14
        tcp_established_connection_count: 0, // Placeholder, Renumbered field 15
        // Instantaneous network speed (Default Interface Only)
        network_rx_bytes_per_sec: network_rx_bps, // Renumbered field 16
        network_tx_bytes_per_sec: network_tx_bps, // Renumbered field 17
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
    let mut networks = Networks::new_with_refreshed_list();
    // Only need to store the time of the previous collection
    let mut prev_net_state: Option<PreviousNetworkState> = None;

    let mut collect_interval_duration = agent_config.metrics_collect_interval_seconds;
    if collect_interval_duration == 0 { collect_interval_duration = 60; }
    let mut collect_interval = tokio::time::interval(Duration::from_secs(collect_interval_duration as u64));

    let mut upload_interval_duration = agent_config.metrics_upload_interval_seconds;
    if upload_interval_duration == 0 { upload_interval_duration = 60; }
    let mut upload_interval = tokio::time::interval(Duration::from_secs(upload_interval_duration as u64));

    let mut batch_max_size = agent_config.metrics_upload_batch_max_size;
    if batch_max_size == 0 { batch_max_size = 10; }

    let mut snapshot_batch_vec = Vec::new();

    println!("[Agent:{}] Metrics collection task started. Collect interval: {}s, Upload interval: {}s, Batch size: {}",
        agent_id, collect_interval_duration, upload_interval_duration, batch_max_size);

    // Initial refresh to set the baseline for the *next* delta calculation by sysinfo
    networks.refresh(true);
    prev_net_state = Some(PreviousNetworkState { time: Instant::now() });


    loop {
        tokio::select! {
            _ = collect_interval.tick() => {
                let current_time = Instant::now();

                // collect_performance_snapshot internally calls networks.refresh(true)
                let snapshot = collect_performance_snapshot(&mut sys, &mut networks, &prev_net_state, current_time);
                snapshot_batch_vec.push(snapshot.clone());

                // Update previous state time for the next iteration's calculation
                prev_net_state = Some(PreviousNetworkState { time: current_time });

                // Batch sending logic (remains the same)
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
                // Batch sending logic (remains the same)
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
            // TODO: Add a way to receive config updates
        }
    }
}