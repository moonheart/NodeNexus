// This file can be used to store all common TypeScript types for the frontend.
export type ServerStatus =
  | 'online'
  | 'offline'
  | 'rebooting'
  | 'provisioning'
  | 'error'
  | 'unknown';
// Add other statuses from your backend if they exist as string literals

// It's also good practice to define constants for these statuses if you need to refer to them programmatically.
export const STATUS_ONLINE = 'online';
export const STATUS_OFFLINE = 'offline';
export const STATUS_REBOOTING = 'rebooting';
export const STATUS_PROVISIONING = 'provisioning';
export const STATUS_ERROR = 'error';
export const STATUS_UNKNOWN = 'unknown';
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
  status: ServerStatus;
  metadata: VpsMetadata | null; // Refined metadata type
  created_at: string; // Represents a `DateTime<Utc>` string, e.g., "2025-06-02T12:34:56.789Z"
  updated_at: string; // Represents a `DateTime<Utc>` string
  tags?: Tag[];
  group?: string | null;
  latest_metrics?: LatestPerformanceMetric | null; // Added for real-time display
  // Traffic Monitoring Fields
  traffic_limit_bytes?: number | null;
  traffic_billing_rule?: string | null;
  traffic_current_cycle_rx_bytes?: number | null;
  traffic_current_cycle_tx_bytes?: number | null;
  traffic_last_reset_at?: string | null; // DateTime<Utc> as string
  traffic_reset_config_type?: string | null;
  traffic_reset_config_value?: string | null;
  next_traffic_reset_at?: string | null; // DateTime<Utc> as string
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
  network_rx_instant_bps?: number | null; // From raw performance_metrics
  network_tx_instant_bps?: number | null; // From raw performance_metrics
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
  status: ServerStatus;
  metadata: VpsMetadata | null; // Refined metadata type
  createdAt: string; // camelCase
  updatedAt: string; // camelCase
  tags?: Tag[];
  group?: string | null;
  latestMetrics?: LatestPerformanceMetric | null; // camelCase
  configStatus: string;
  lastConfigUpdateAt?: string | null;
  lastConfigError?: string | null;
  agentConfigOverride?: Record<string, unknown> | null;
  // Traffic Monitoring Fields from Vps (matching ServerBasicInfo in backend)
  trafficLimitBytes?: number | null;
  trafficBillingRule?: string | null;
  trafficCurrentCycleRxBytes?: number | null;
  trafficCurrentCycleTxBytes?: number | null;
  trafficLastResetAt?: string | null;
  trafficResetConfigType?: string | null;
  trafficResetConfigValue?: string | null;
  nextTrafficResetAt?: string | null;
}

/**
 * Represents the top-level structure of the data pushed via WebSocket.
 * It contains a list of all servers with their details and latest metrics.
 */
export interface FullServerListPushType {
  servers: VpsListItemResponse[];
}
export type ViewMode = 'card' | 'list';
/**
 * Represents the configuration for an agent.
 * This should match the structure of the `AgentConfig` message in `backend/proto/config.proto`.
 */
export interface AgentConfig {
  metrics_collect_interval_seconds: number;
  metrics_upload_batch_max_size: number;
  metrics_upload_interval_seconds: number;
  docker_info_collect_interval_seconds: number;
  docker_info_upload_interval_seconds: number;
  generic_metrics_upload_batch_max_size: number;
  generic_metrics_upload_interval_seconds: number;
  feature_flags: Record<string, string>;
  log_level: string;
  heartbeat_interval_seconds: number;
}

/**
 * Represents a Tag that can be associated with a VPS.
 * This should match the `Tag` and `TagWithCount` structs from the backend.
 */
export interface Tag {
  id: number;
  userId: number;
  name: string;
  color: string;
  icon?: string | null;
  url?: string | null;
  isVisible: boolean;
  createdAt: string;
  updatedAt: string;
  vpsCount?: number; // From TagWithCount
}

/**
 * Type for creating a new tag, matches backend CreateTagRequest
 */
export interface CreateTagPayload {
  name: string;
  color: string;
  icon?: string;
  url?: string;
  is_visible?: boolean;
}

/**
 * Type for updating a tag, matches backend UpdateTagRequest
 */
export interface UpdateTagPayload {
  name: string;
  color: string;
  icon?: string;
  url?: string;
  is_visible: boolean;
}

// --- Notification Channel Types ---

/**
 * Defines the structure for a field in a channel template for the frontend.
 * This should match the `ChannelTemplateField` struct from the backend.
 */
export interface ChannelTemplateField {
  name: string;
  type: string; // e.g., "text", "textarea", "password"
  required: boolean;
  label: string;
  helpText?: string | null;
}

/**
 * Defines the template for a channel type, used to dynamically generate UI.
 * This should match the `ChannelTemplate` struct from the backend.
 */
export interface ChannelTemplate {
  channelType: string;
  name: string;
  fields: ChannelTemplateField[];
}

/**
 * Represents the API response for a single notification channel.
 * This should match the `ChannelResponse` struct from the backend.
 */
export interface ChannelResponse {
  id: number;
  name: string;
  channelType: string;
  config?: Record<string, unknown>; // Added optional config for editing
}

/**
 * Represents the API request payload for creating a new notification channel.
 * This should match the `CreateChannelRequest` struct from the backend.
 */
export interface CreateChannelRequest {
  name: string;
  channelType: string;
  config: Record<string, unknown>; // The raw config JSON from the frontend
}

/**
 * Represents the API request payload for updating an existing notification channel.
 * This should match the `UpdateChannelRequest` struct from the backend.
 */
export interface UpdateChannelRequest {
  name?: string;
  config?: Record<string, unknown>;
}
// --- Alert Rule Types ---

export interface AlertRule {
  id: number;
  userId: number;
  name: string; // Added name field for better identification
  vpsId?: number | null;
  metricType: string;
  threshold: number;
  comparisonOperator: string;
  durationSeconds: number;
  notificationChannelIds?: number[]; // To link to notification_channels table
  cooldownSeconds?: number;
  isActive: boolean; // Added
  createdAt: string;
  updatedAt: string;
}

export interface CreateAlertRulePayload {
  name: string;
  vpsId?: number | null;
  metricType: string;
  threshold: number;
  comparisonOperator: string;
  durationSeconds: number;
  notificationChannelIds?: number[]; // Array of channel IDs
  cooldownSeconds?: number; // Added
}

export type UpdateAlertRulePayload = Partial<CreateAlertRulePayload>;

// --- VPS Metadata Types ---
export interface CpuStaticInfo {
  name: string;
  frequency: number; // Assuming uint64 from backend can be represented as number
  vendorId: string;
  brand: string;
}

export interface VpsMetadata {
  os_name?: string;
  arch?: string;
  hostname?: string;
  public_ip_addresses?: string[];
  kernel_version?: string;
  os_version_detail?: string;
  long_os_version?: string;
  distribution_id?: string;
  physical_core_count?: number;
  total_memory_bytes?: number; // uint64 in backend
  total_swap_bytes?: number;   // uint64 in backend
  cpu_static_info?: CpuStaticInfo;
  country_code?: string; // Added for flag display
  // Add any other known metadata fields that might be present
  // Ensure keys match exactly what's sent from the backend (e.g., snake_case or camelCase)
  // Based on backend vps_service.rs, keys are snake_case
}