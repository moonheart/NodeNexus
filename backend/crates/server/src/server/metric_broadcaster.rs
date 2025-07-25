use std::sync::Arc;
use std::time::Duration;

use dashmap::DashMap;
use tokio::sync::{broadcast, mpsc};
use tokio::time;
use tracing::{debug, info};

use crate::db::entities::performance_metric;
use crate::web::models::websocket_models::{
    PerformanceMetricBatch, PerformanceMetricPoint, WsMessage,
};

/// A service that buffers performance metrics and broadcasts them in batches periodically.
/// This helps to reduce the frequency of WebSocket messages.
#[derive(Debug)]
pub struct MetricBroadcaster {
    /// Receives individual metric points from the gRPC service.
    metric_receiver: mpsc::Receiver<performance_metric::Model>,
    /// Broadcasts batched metrics to all connected WebSocket clients.
    ws_broadcaster: broadcast::Sender<WsMessage>,
    /// A concurrent map to buffer metrics per VPS.
    /// Key: vps_id, Value: Vec of metric points.
    buffer: Arc<DashMap<i32, Vec<PerformanceMetricPoint>>>,
}

impl MetricBroadcaster {
    /// Creates a new `MetricBroadcaster` and the sender part of its channel.
    pub fn new(
        ws_broadcaster: broadcast::Sender<WsMessage>,
    ) -> (Self, mpsc::Sender<performance_metric::Model>) {
        let (metric_sender, metric_receiver) = mpsc::channel(2048); // Buffer up to 2048 metrics
        let broadcaster = Self {
            metric_receiver,
            ws_broadcaster,
            buffer: Arc::new(DashMap::new()),
        };
        (broadcaster, metric_sender)
    }

    /// Starts the broadcasting service.
    /// This will spawn two background tasks: one for receiving metrics and one for broadcasting them.
    pub fn run(mut self) {
        // Task 1: Receive incoming metrics and put them into the buffer.
        let buffer_clone = Arc::clone(&self.buffer);
        tokio::spawn(async move {
            while let Some(metric) = self.metric_receiver.recv().await {
                let point = PerformanceMetricPoint {
                    time: metric.time,
                    vps_id: metric.vps_id,
                    cpu_usage_percent: Some(metric.cpu_usage_percent),
                    memory_usage_bytes: Some(metric.memory_usage_bytes),
                    memory_total_bytes: Some(metric.memory_total_bytes),
                    network_rx_instant_bps: Some(metric.network_rx_instant_bps),
                    network_tx_instant_bps: Some(metric.network_tx_instant_bps),
                    disk_io_read_bps: Some(metric.disk_io_read_bps),
                    disk_io_write_bps: Some(metric.disk_io_write_bps),
                    swap_usage_bytes: Some(metric.swap_usage_bytes),
                    swap_total_bytes: Some(metric.swap_total_bytes),
                    disk_used_bytes: Some(metric.used_disk_space_bytes),
                    disk_total_bytes: Some(metric.total_disk_space_bytes),
                };
                buffer_clone.entry(metric.vps_id).or_default().push(point);
            }
        });

        // Task 2: Periodically broadcast the buffered metrics.
        let buffer_clone = Arc::clone(&self.buffer);
        let ws_broadcaster_clone = self.ws_broadcaster.clone();
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                let receiver_count = ws_broadcaster_clone.receiver_count();
                let is_empty = buffer_clone.is_empty();
                if receiver_count == 0 {
                    // No clients, clear buffer to prevent memory leak
                    if !is_empty {
                        buffer_clone.clear();
                        debug!("Cleared metric buffer as no clients are connected.");
                    }
                    continue;
                }

                // Step 1: Atomically take the metrics from the buffer, minimizing lock time.
                let mut all_metrics = Vec::new();
                for mut entry in buffer_clone.iter_mut() {
                    if !entry.value().is_empty() {
                        let mut metrics = std::mem::take(entry.value_mut());
                        all_metrics.append(&mut metrics);
                    }
                }

                // Step 2: Drop the lock by ending the iteration, then send the data.
                if !all_metrics.is_empty() {
                    debug!(
                        "Broadcasting {} metric points in a single batch.",
                        all_metrics.len()
                    );
                    let batch = PerformanceMetricBatch {
                        metrics: all_metrics,
                    };
                    let message = WsMessage::PerformanceMetricBatch(batch);
                    if let Err(e) = ws_broadcaster_clone.send(message) {
                        debug!(
                            "Failed to broadcast performance metric batch (no subscribers?): {}",
                            e
                        );
                    }
                }
            }
        });

        info!("MetricBroadcaster started.");
    }
}