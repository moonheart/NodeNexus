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
  vpsId: number; // camelCase
  cpuUsagePercent: number; // camelCase
  memoryUsageBytes: number; // camelCase
  memoryTotalBytes: number; // camelCase
  swapUsageBytes: number; // camelCase
  swapTotalBytes: number; // camelCase
  diskIoReadBps: number; // camelCase
  diskIoWriteBps: number; // camelCase
  networkRxBps: number; // camelCase (Cumulative RX bytes)
  networkTxBps: number; // camelCase (Cumulative TX bytes)
  networkRxInstantBps: number; // camelCase
  networkTxInstantBps: number; // camelCase
  uptimeSeconds: number; // camelCase
  totalProcessesCount: number; // camelCase
  runningProcessesCount: number; // camelCase
  tcpEstablishedConnectionCount: number; // camelCase
  diskTotalBytes?: number; // camelCase
  diskUsedBytes?: number;  // camelCase
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
  avg_network_rx_instant_bps?: number | null; // Calculated average Rx bytes per second (Matches backend AggregatedPerformanceMetric)
  avg_network_tx_instant_bps?: number | null; // Calculated average Tx bytes per second (Matches backend AggregatedPerformanceMetric)
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

/**
 * Represents the structure for a VPS item in a list or for detail view,
 * including its latest metrics. This matches the VpsListItemResponse from the backend.
 */
export interface VpsListItemResponse {
  id: number;
  userId: number; // camelCase
  name: string;
  ipAddress: string | null; // camelCase
  osType: string | null;    // camelCase
  agentSecret: string; // camelCase
  status: string;
  metadata: Record<string, unknown> | null;
  createdAt: string; // camelCase
  updatedAt: string; // camelCase
  latestMetrics?: LatestPerformanceMetric | null; // camelCase
}