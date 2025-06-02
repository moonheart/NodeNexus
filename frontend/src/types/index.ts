// This file can be used to store all common TypeScript types for the frontend.

/**
 * Represents a Virtual Private Server (VPS) as defined by the backend.
 * This should match the structure of the `Vps` model in `backend/src/db/models.rs`.
 */
export interface Vps {
  id: number;
  user_id: number;
  name: string;
  ip_address: string | null;
  os_type: string | null;
  agent_secret: string;
  status: string;
  metadata: Record<string, unknown> | null; // Can be refined if the structure of metadata is known
  created_at: string; // Represents a `DateTime<Utc>` string, e.g., "2025-06-02T12:34:56.789Z"
  updated_at: string; // Represents a `DateTime<Utc>` string
  latest_metrics?: LatestPerformanceMetric | null; // Added for real-time display
}

/**
 * Represents a complete, latest performance metric snapshot for a VPS.
 * This should align with the `PerformanceMetric` model in `backend/src/db/models.rs`.
 */
export interface LatestPerformanceMetric {
  id: number;
  time: string; // DateTime<Utc>
  vps_id: number;
  cpu_usage_percent: number;
  memory_usage_bytes: number;
  memory_total_bytes: number;
  swap_usage_bytes: number;
  swap_total_bytes: number;
  disk_io_read_bps: number;
  disk_io_write_bps: number;
  network_rx_bps: number; // Downstream
  network_tx_bps: number; // Upstream
  load_average_one_min: number;
  load_average_five_min: number;
  load_average_fifteen_min: number;
  uptime_seconds: number;
  total_processes_count: number;
  running_processes_count: number;
  tcp_established_connection_count: number;
}

/**
 * Represents a single point in a time series for performance metrics.
 * This should align with the `AggregatedPerformanceMetric` from the backend,
 * or the raw `PerformanceMetric` if not aggregated.
 */
export interface PerformanceMetricPoint {
  time: string; // Timestamp string (ISO format from backend, or from time_bucket)
  vps_id: number; // Included for consistency, though often known from context
  avg_cpu_usage_percent?: number | null; // From AVG(cpu_usage_percent)
  cpu_usage_percent?: number | null; // From raw cpu_usage_percent
  avg_memory_usage_bytes?: number | null; // From AVG(memory_usage_bytes)
  memory_usage_bytes?: number | null; // From raw memory_usage_bytes
  max_memory_total_bytes?: number | null; // From MAX(memory_total_bytes)
  memory_total_bytes?: number | null; // From raw memory_total_bytes
  memory_usage_percent?: number | null; // Calculated: (memory_usage_bytes / memory_total_bytes) * 100 or from aggregated
  // Add other relevant fields that might come from backend (raw or aggregated)
}

/**
 * Represents the structure for CPU and Memory metrics to be displayed on charts.
 * Each array would contain points for a specific metric over time.
 */
export interface VpsChartMetrics {
  cpuUsage: PerformanceMetricPoint[];
  memoryUsage: PerformanceMetricPoint[]; // Points here will need memory_usage_percent calculated
}