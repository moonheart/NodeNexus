-- Add an ID primary key to performance_metrics and new columns for full PerformanceSnapshot
ALTER TABLE performance_metrics
ADD COLUMN id SERIAL PRIMARY KEY,
ADD COLUMN swap_usage_bytes BIGINT NOT NULL DEFAULT 0,
ADD COLUMN swap_total_bytes BIGINT NOT NULL DEFAULT 0,
ADD COLUMN load_average_one_min DOUBLE PRECISION NOT NULL DEFAULT 0.0,
ADD COLUMN load_average_five_min DOUBLE PRECISION NOT NULL DEFAULT 0.0,
ADD COLUMN load_average_fifteen_min DOUBLE PRECISION NOT NULL DEFAULT 0.0,
ADD COLUMN uptime_seconds BIGINT NOT NULL DEFAULT 0,
ADD COLUMN total_processes_count INTEGER NOT NULL DEFAULT 0,
ADD COLUMN running_processes_count INTEGER NOT NULL DEFAULT 0,
ADD COLUMN tcp_established_connection_count INTEGER NOT NULL DEFAULT 0;

-- Create table for detailed disk usages
CREATE TABLE performance_disk_usages (
    id SERIAL PRIMARY KEY,
    performance_metric_id INTEGER NOT NULL REFERENCES performance_metrics(id) ON DELETE CASCADE,
    mount_point TEXT NOT NULL,
    used_bytes BIGINT NOT NULL,
    total_bytes BIGINT NOT NULL,
    fstype TEXT, -- Can be null if not available
    usage_percent DOUBLE PRECISION NOT NULL,
    CONSTRAINT uq_disk_usage_metric_mount UNIQUE (performance_metric_id, mount_point)
);

-- Add index for faster lookups on performance_disk_usages
CREATE INDEX idx_performance_disk_usages_metric_id ON performance_disk_usages (performance_metric_id);

-- Create table for detailed network interface stats
CREATE TABLE performance_network_interface_stats (
    id SERIAL PRIMARY KEY,
    performance_metric_id INTEGER NOT NULL REFERENCES performance_metrics(id) ON DELETE CASCADE,
    interface_name TEXT NOT NULL,
    rx_bytes_per_sec BIGINT NOT NULL,
    tx_bytes_per_sec BIGINT NOT NULL,
    rx_packets_per_sec BIGINT NOT NULL,
    tx_packets_per_sec BIGINT NOT NULL,
    rx_errors_total_cumulative BIGINT NOT NULL,
    tx_errors_total_cumulative BIGINT NOT NULL,
    CONSTRAINT uq_network_stats_metric_interface UNIQUE (performance_metric_id, interface_name)
);

-- Add index for faster lookups on performance_network_interface_stats
CREATE INDEX idx_performance_network_interface_stats_metric_id ON performance_network_interface_stats (performance_metric_id);

-- Note: The existing index on performance_metrics (vps_id, time DESC) is still relevant.
-- If performance_metrics is a TimescaleDB hypertable, these changes should be compatible.
-- SELECT create_hypertable('performance_metrics', 'time', if_not_exists => TRUE, migrate_data => TRUE);
-- If converting to hypertable after adding data, ensure 'time' and 'vps_id' (partitioning key) are suitable.
-- The new tables performance_disk_usages and performance_network_interface_stats are regular PostgreSQL tables.