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
  time: string;
  vpsId: number;
  avgCpuUsagePercent?: number | null;
  cpuUsagePercent?: number | null;
  avgMemoryUsageBytes?: number | null;
  memoryUsageBytes?: number | null;
  maxMemoryTotalBytes?: number | null;
  memoryTotalBytes?: number | null;
  memoryUsagePercent?: number | null;
  avgNetworkRxInstantBps?: number | null;
  avgNetworkTxInstantBps?: number | null;
  networkRxInstantBps?: number | null;
  networkTxInstantBps?: number | null;
  avgDiskIoReadBps?: number | null;
  avgDiskIoWriteBps?: number | null;
  diskIoReadBps?: number | null;
  diskIoWriteBps?: number | null;
  swapUsageBytes?: number | null;
  swapTotalBytes?: number | null;
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
  agentVersion?: string | null;
  agentSecret?: string; // camelCase, optional as it's only in detail view
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

  // Renewal Info Fields
  renewalCycle?: string | null;
  renewalCycleCustomDays?: number | null;
  renewalPrice?: number | null;
  renewalCurrency?: string | null;
  nextRenewalDate?: string | null; // DateTime<Utc> as string
  lastRenewalDate?: string | null; // DateTime<Utc> as string
  serviceStartDate?: string | null; // DateTime<Utc> as string
  paymentMethod?: string | null;
  autoRenewEnabled?: boolean | null;
  renewalNotes?: string | null;
  reminderActive?: boolean | null;
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
  metricsCollectIntervalSeconds: number;
  metricsUploadBatchMaxSize: number;
  metricsUploadIntervalSeconds: number;
  dockerInfoCollectIntervalSeconds: number;
  dockerInfoUploadIntervalSeconds: number;
  genericMetricsUploadBatchMaxSize: number;
  genericMetricsUploadIntervalSeconds: number;
  featureFlags: Record<string, string>;
  logLevel: string;
  heartbeatIntervalSeconds: number;
  serviceMonitorTasks: ServiceMonitorTask[];
}

/**
 * Represents a single service monitoring task as defined in `config.proto`.
 * This is part of the AgentConfig.
 */
export interface ServiceMonitorTask {
  monitorId: number;
  name: string;
  monitorType: string;
  target: string;
  frequencySeconds: number;
  monitorConfigJson: string;
  timeoutSeconds: number;
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
  configParams?: Record<string, unknown>; // Renamed from config and matches backend
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

// --- Service Monitoring Types ---

/**
 * Represents the detailed configuration for a specific monitor type.
 * Matches the `monitor_config` JSON blob in the database.
 */
export interface HttpMonitorConfig {
  method?: string;
  expected_status_codes?: number[];
  request_headers?: Record<string, string>;
  request_body?: string;
  response_body_match?: string;
  ignore_tls_errors?: boolean;
}

export interface PingMonitorConfig {
    packet_count?: number;
}

export type TcpMonitorConfig = Record<string, never>;

export type MonitorConfig = HttpMonitorConfig | PingMonitorConfig | TcpMonitorConfig;


/**
 * Represents a service monitor task.
 * This should match the `ServiceMonitorResponse` struct from the backend.
 */
export interface ServiceMonitor {
  id: number;
  userId: number;
  name: string;
  monitorType: 'http' | 'ping' | 'tcp';
  target: string;
  frequencySeconds: number;
  timeoutSeconds: number;
  isActive: boolean;
  monitorConfig: MonitorConfig;
  createdAt: string;
  updatedAt: string;
  agentIds: number[];
  tagIds: number[];
}

/**
 * Represents the payload for creating or updating a service monitor.
 * This should match the `CreateMonitorRequest` and `UpdateMonitorRequest` structs from the backend.
 */
export interface ServiceMonitorInput {
  name: string;
  monitorType: 'http' | 'ping' | 'tcp';
  target: string;
  frequencySeconds?: number;
  timeoutSeconds?: number;
  isActive?: boolean;
  monitorConfig?: MonitorConfig;
  assignments?: {
    agentIds?: number[];
    tagIds?: number[];
  };
}

/**
 * Represents a single result from a service monitor check.
 * This should match the `service_monitor_results` table schema.
 */
export interface ServiceMonitorResult {
  time: string; // TIMESTAMPTZ
  monitorId: number;
  agentId: number;
  agentName: string;
  monitorName: string; // Added from the JOIN in the backend
  isUp: boolean;
  latencyMs: number | null;
  details?: {
      status_code?: number;
      error?: string;
      message?: string;
  };
}

// --- Batch Command & Scripting Types ---

export interface ChildCommandTaskDetail {
  child_command_id: string;
  vps_id: number;
  status: string;
  exit_code: number | null;
  error_message: string | null;
  created_at: string;
  updated_at: string;
  agent_started_at: string | null;
  agent_completed_at: string | null;
  last_output_at: string | null;
}

export interface BatchCommandTaskDetailResponse {
  batch_command_id: string;
  overall_status: string;
  execution_alias: string | null;
  user_id: string;
  original_request_payload: Record<string, unknown>;
  tasks: ChildCommandTaskDetail[];
  created_at: string;
  updated_at: string;
  completed_at: string | null;
}

export interface CommandScript {
  id: number;
  user_id: number;
  name: string;
  description: string | null;
  script_content: string;
  working_directory: string;
  created_at: string;
  updated_at: string;
}

export interface BulkActionResponse {
  message: string;
  successfulCount: number;
  failedCount: number;
}