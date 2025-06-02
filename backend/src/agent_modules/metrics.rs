use crate::agent_service::{
    AgentConfig, MessageToServer, NetworkInterfaceStats, PerformanceSnapshot,
    PerformanceSnapshotBatch, message_to_server::Payload,
};
use std::time::Duration;
use sysinfo::{System, Disks, Networks, DiskRefreshKind};
use tokio::sync::mpsc;

// Note: get_next_client_message_id will be handled by the communication module or passed appropriately.
// For now, metrics_collection_loop will take an id_provider.

pub fn collect_performance_snapshot(sys: &mut System) -> PerformanceSnapshot {
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

pub async fn metrics_collection_loop(
    tx_to_server: mpsc::Sender<MessageToServer>,
    agent_config: AgentConfig,
    agent_id: String,
    mut id_provider: impl FnMut() -> u64 + Send + 'static, // Closure to provide message IDs
    vps_db_id: i32,
    agent_secret: String,
) {
    let mut sys = System::new_all();
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

    loop {
        tokio::select! {
            _ = collect_interval.tick() => {
                let snapshot = collect_performance_snapshot(&mut sys);
                snapshot_batch_vec.push(snapshot);

                if snapshot_batch_vec.len() >= batch_max_size as usize {
                    let batch_to_send_vec = std::mem::take(&mut snapshot_batch_vec);
                    if !batch_to_send_vec.is_empty() {
                        let batch_len = batch_to_send_vec.len();
                        let batch_payload = PerformanceSnapshotBatch { snapshots: batch_to_send_vec };
                        let msg_id = id_provider(); // Use the closure
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
                    let batch_len = batch_to_send_vec.len();
                    let batch_payload = PerformanceSnapshotBatch { snapshots: batch_to_send_vec };
                    let msg_id = id_provider(); // Use the closure
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