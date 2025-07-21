use nodenexus_common::agent_service::{
    AgentConfig, MessageToServer, PerformanceSnapshot, PerformanceSnapshotBatch,
    message_to_server::Payload,
};
use netdev::interface::InterfaceType;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use sysinfo::{DiskKind, Disks, Networks, System};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

// PreviousNetworkState struct is no longer needed
// PreviousDiskState struct is no longer needed

/// Collects a performance snapshot focusing ONLY on the default network interface
/// for both cumulative and instantaneous network data, and total disk I/O rates.
fn collect_performance_snapshot(
    sys: &System,
    disks: &mut Disks,
    networks: &mut Networks,
    prev_collection_time_opt: &Option<Instant>,
    current_time: Instant,
    excluded_fs_types: &HashSet<&str>,
    active_interface_name: &Option<String>, // Accept pre-determined interface name
) -> PerformanceSnapshot {
    // Refresh logic is now handled in the main loop.

    // Refresh the persistent Networks instance. This updates delta values.
    networks.refresh(false);

    let cpu_usage = sys.global_cpu_usage();
    let mem_used = sys.used_memory();
    let mem_total = sys.total_memory();

    // Cumulative Disk I/O (Remains the same - sums across all disks)
    disks.refresh(false); // Refresh stats for all disks in the list.

    let mut delta_total_disk_read_bytes: u64 = 0;
    let mut delta_total_disk_written_bytes: u64 = 0;

    for disk_info in disks.list() {
        let fs_type_str = disk_info.file_system().to_string_lossy();
        if disk_info.total_space() > 0
            && matches!(disk_info.kind(), DiskKind::HDD | DiskKind::SSD)
            && !excluded_fs_types.contains(fs_type_str.as_ref())
        {
            let disk_usage_stats = disk_info.usage();
            delta_total_disk_read_bytes += disk_usage_stats.read_bytes;
            delta_total_disk_written_bytes += disk_usage_stats.written_bytes;
        }
    }

    // Calculate Disk I/O BPS
    let mut disk_read_bps: u64 = 0;
    let mut disk_write_bps: u64 = 0;
    if let Some(prev_time) = prev_collection_time_opt {
        let duration = current_time.duration_since(*prev_time);
        let duration_secs_for_disk = duration.as_secs_f64();
        if duration_secs_for_disk > 0.0 {
            disk_read_bps = (delta_total_disk_read_bytes as f64 / duration_secs_for_disk) as u64;
            disk_write_bps =
                (delta_total_disk_written_bytes as f64 / duration_secs_for_disk) as u64;
        }
    }

    // Collect detailed disk usages
    let mut collected_disk_usages = Vec::new();
    for disk_info in disks.list() {
        let fs_type_str = disk_info.file_system().to_string_lossy();
        if disk_info.total_space() > 0
            && matches!(disk_info.kind(), DiskKind::HDD | DiskKind::SSD)
            && !excluded_fs_types.contains(fs_type_str.as_ref())
        {
            let total_space = disk_info.total_space();
            let available_space = disk_info.available_space();
            let used_space = total_space.saturating_sub(available_space);
            let usage_percent = if total_space > 0 {
                (used_space as f64 / total_space as f64) * 100.0
            } else {
                0.0
            };
            collected_disk_usages.push(nodenexus_common::agent_service::DiskUsage {
                mount_point: disk_info.mount_point().to_string_lossy().into_owned(),
                used_bytes: used_space,
                total_bytes: total_space,
                fstype: fs_type_str.into_owned(),
                usage_percent,
            });
        }
    }

    let (total_disk_space_bytes, used_disk_space_bytes) = collected_disk_usages
        .iter()
        .fold((0, 0), |(total_acc, used_acc), disk| {
            (total_acc + disk.total_bytes, used_acc + disk.used_bytes)
        });

    // --- Network I/O (Default Interface Only) ---
    let mut cumulative_rx_bytes: u64 = 0;
    let mut cumulative_tx_bytes: u64 = 0;
    let mut delta_rx_bytes_for_rate: u64 = 0;
    let mut delta_tx_bytes_for_rate: u64 = 0;

    // Use the pre-determined active interface name
    match active_interface_name {
        Some(interface_name) => {
            let mut found_in_sysinfo = false;
            for (if_name, data) in networks.iter() {
                if if_name == interface_name.as_str() {
                    cumulative_rx_bytes = data.total_received();
                    cumulative_tx_bytes = data.total_transmitted();
                    delta_rx_bytes_for_rate = data.received();
                    delta_tx_bytes_for_rate = data.transmitted();
                    found_in_sysinfo = true;
                    debug!(
                        interface = %if_name,
                        cum_rx = cumulative_rx_bytes,
                        cum_tx = cumulative_tx_bytes,
                        delta_rx = delta_rx_bytes_for_rate,
                        delta_tx = delta_tx_bytes_for_rate,
                        "Using active interface."
                    );
                    break;
                }
            }
            if !found_in_sysinfo {
                warn!(interface_name = %interface_name, "Active interface found at startup, but not in sysinfo list now. Network stats will be 0.");
            }
        }
        None => {
            // Warning is now logged at startup, no need to log every time here.
        }
    }

    // Calculate Instantaneous BPS using the default interface's delta (or 0 if not found)
    let mut network_rx_bps: u64 = 0;
    let mut network_tx_bps: u64 = 0;

    if let Some(prev_time) = prev_collection_time_opt {
        let duration = current_time.duration_since(*prev_time);
        let duration_secs = duration.as_secs_f64();

        // Only calculate if delta is non-zero and duration is positive
        if duration_secs > 0.0 && (delta_rx_bytes_for_rate > 0 || delta_tx_bytes_for_rate > 0) {
            network_rx_bps = (delta_rx_bytes_for_rate as f64 / duration_secs) as u64;
            network_tx_bps = (delta_tx_bytes_for_rate as f64 / duration_secs) as u64;

            // --- Debugging Output ---
            debug!(
                duration_secs,
                delta_rx = delta_rx_bytes_for_rate,
                delta_tx = delta_tx_bytes_for_rate,
                rx_bps = network_rx_bps,
                tx_bps = network_tx_bps,
                "BPS calculation details."
            );
            // --- End Debugging ---
        } else if duration_secs <= 0.0 {
            warn!("Duration is zero or negative, cannot calculate BPS.");
        } else {
            warn!("Delta RX/TX is zero, BPS is 0.");
        }
    } else {
        warn!("No previous network state, cannot calculate BPS for the first snapshot.");
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
        disk_total_io_read_bytes_per_sec: disk_read_bps, // NOW ACTUAL BPS
        disk_total_io_write_bytes_per_sec: disk_write_bps, // NOW ACTUAL BPS
        disk_usages: collected_disk_usages,              // MODIFIED
        total_disk_space_bytes,                          // NEW
        used_disk_space_bytes,                           // NEW
        // Cumulative network data (Default Interface Only)
        network_rx_bytes_cumulative: cumulative_rx_bytes, // Field 10
        network_tx_bytes_cumulative: cumulative_tx_bytes, // Field 11
        // Load averages removed
        // Removed load_average fields
        uptime_seconds: System::uptime(), // Renumbered field 12
        total_processes_count: sys.processes().len() as u32, // Renumbered field 13
        running_processes_count: 0,       // Placeholder, Renumbered field 14
        tcp_established_connection_count: 0, // Placeholder, Renumbered field 15
        // Instantaneous network speed (Default Interface Only)
        network_rx_bytes_per_sec: network_rx_bps, // Renumbered field 16
        network_tx_bytes_per_sec: network_tx_bps, // Renumbered field 17
    }
}

pub async fn metrics_collection_loop(
    tx_to_server: mpsc::Sender<MessageToServer>,
    shared_agent_config: Arc<RwLock<AgentConfig>>,
    mut id_provider: impl FnMut() -> u64 + Send + 'static,
    vps_db_id: i32,
    agent_secret: String,
    mut sys: System,
    mut shutdown_rx: tokio::sync::watch::Receiver<()>,
) {
    let mut disks = Disks::new_with_refreshed_list();
    let mut networks = Networks::new_with_refreshed_list();
    let mut snapshot_batch_vec = Vec::new();

    // Define a set of file system types to exclude.
    let excluded_fs_types: HashSet<&str> = ["squashfs", "overlay", "devtmpfs", "tmpfs"]
        .iter()
        .cloned()
        .collect();

    // Find the active network interface once at startup to avoid repeated lookups.
    let active_interface_name = {
        let interfaces = netdev::get_interfaces();
        interfaces
            .into_iter()
            .find(|iface| {
                (!iface.ipv4.is_empty() || !iface.ipv6.is_empty())
                    && iface.gateway.is_some()
                    && iface.if_type == InterfaceType::Ethernet
            })
            .map(|iface| iface.friendly_name.unwrap_or(iface.name))
    };

    if let Some(name) = &active_interface_name {
        info!(interface_name = %name, "Found active network interface for metrics.");
    } else {
        warn!(
            "Could not find an active network interface with a gateway. Network stats will be 0."
        );
    }

    // --- Dynamic Configuration Setup ---
    let (mut collect_interval_duration, mut upload_interval_duration, mut batch_max_size) = {
        let config = shared_agent_config.read().unwrap();
        let initial_collect_interval = config.metrics_collect_interval_seconds;
        info!(
            initial_value = initial_collect_interval,
            "Metrics loop initializing with collect interval."
        );
        (
            initial_collect_interval,
            config.metrics_upload_interval_seconds,
            config.metrics_upload_batch_max_size,
        )
    };
    if collect_interval_duration == 0 {
        warn!("Initial collect interval is 0, falling back to 60 seconds.");
        collect_interval_duration = 60;
    }
    if upload_interval_duration == 0 {
        upload_interval_duration = 60;
    }
    if batch_max_size == 0 {
        batch_max_size = 10;
    }

    let mut collect_interval =
        tokio::time::interval(Duration::from_secs(collect_interval_duration as u64));
    let mut upload_interval =
        tokio::time::interval(Duration::from_secs(upload_interval_duration as u64));
    // --- End Dynamic Configuration Setup ---

    info!(
        collect_interval_seconds = collect_interval_duration,
        upload_interval_seconds = upload_interval_duration,
        batch_size = batch_max_size,
        "Metrics collection task started."
    );

    // Initial refresh to set the baseline for the *next* delta calculation by sysinfo
    disks.refresh(true);
    networks.refresh(true);
    let mut prev_collection_time: Option<Instant> = Some(Instant::now());

    loop {
        // Refresh system data at the start of each loop iteration for efficiency.
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        // Use a minimal process refresh kind. We only need the process count,
        // not expensive details like command lines or environment variables.
        sys.refresh_processes_specifics(
            sysinfo::ProcessesToUpdate::All,
            true,
            sysinfo::ProcessRefreshKind::nothing().without_tasks(),
        );


        // --- Check for configuration changes ---
        {
            let config = shared_agent_config.read().unwrap();
            let new_collect_interval = if config.metrics_collect_interval_seconds > 0 {
                config.metrics_collect_interval_seconds
            } else {
                60
            };
            debug!(
                current_interval = collect_interval_duration,
                new_interval_from_config = new_collect_interval,
                "Checking for metrics configuration changes."
            );
            let new_upload_interval = if config.metrics_upload_interval_seconds > 0 {
                config.metrics_upload_interval_seconds
            } else {
                60
            };
            let new_batch_size = if config.metrics_upload_batch_max_size > 0 {
                config.metrics_upload_batch_max_size
            } else {
                10
            };

            if new_collect_interval != collect_interval_duration {
                info!(
                    new_interval = new_collect_interval,
                    "Updating metrics collect interval."
                );
                collect_interval_duration = new_collect_interval;
                collect_interval =
                    tokio::time::interval(Duration::from_secs(collect_interval_duration as u64));
            }
            if new_upload_interval != upload_interval_duration {
                info!(
                    new_interval = new_upload_interval,
                    "Updating metrics upload interval."
                );
                upload_interval_duration = new_upload_interval;
                upload_interval =
                    tokio::time::interval(Duration::from_secs(upload_interval_duration as u64));
            }
            if new_batch_size != batch_max_size {
                info!(new_size = new_batch_size, "Updating metrics batch size.");
                batch_max_size = new_batch_size;
            }
        }
        // --- End Check ---

        tokio::select! {
            biased;

            _ = shutdown_rx.changed() => {
                info!("Shutdown signal received, terminating metrics collection loop.");
                break;
            }

            _ = collect_interval.tick() => {
                let current_time = Instant::now();
                let snapshot = collect_performance_snapshot(
                    &sys,
                    &mut disks,
                    &mut networks,
                    &prev_collection_time,
                    current_time,
                    &excluded_fs_types,
                    &active_interface_name, // Pass the cached interface name
                );
                snapshot_batch_vec.push(snapshot.clone());
                prev_collection_time = Some(current_time); // Update prev_collection_time for the next iteration

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
                            error!(error = %e, "Failed to send metrics batch (size trigger).");
                        } else {
                            debug!(msg_id = msg_id, batch_size = batch_len, "Sent metrics batch (size trigger).");
                        }
                    }
                }
            }
            _ = upload_interval.tick() => {
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
                        error!(error = %e, "Failed to send metrics batch (interval trigger).");
                    } else {
                        debug!(msg_id = msg_id, batch_size = batch_len, "Sent metrics batch (interval trigger).");
                    }
                }
            }
        }
    }
    info!("Metrics collection loop gracefully shut down.");
}
