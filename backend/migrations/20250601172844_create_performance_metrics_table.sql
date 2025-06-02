-- Create performance_metrics table
CREATE TABLE performance_metrics (
    time TIMESTAMPTZ NOT NULL,
    vps_id INTEGER NOT NULL REFERENCES vps(id) ON DELETE CASCADE,
    cpu_usage_percent DOUBLE PRECISION NOT NULL,
    memory_usage_bytes BIGINT NOT NULL,
    memory_total_bytes BIGINT NOT NULL,
    disk_io_read_bps BIGINT NOT NULL,
    disk_io_write_bps BIGINT NOT NULL,
    network_rx_bps BIGINT NOT NULL,
    network_tx_bps BIGINT NOT NULL
);

-- Optional: Convert to hypertable, partitioned by time.
-- This requires the TimescaleDB extension to be enabled.
-- SELECT create_hypertable('performance_metrics', 'time');

-- If not using TimescaleDB hypertable, or if hypertable creation is deferred,
-- an index on vps_id and time is crucial for query performance.
-- TimescaleDB typically creates a suitable index automatically when creating a hypertable.
CREATE INDEX idx_performance_metrics_vps_id_time_desc ON performance_metrics (vps_id, time DESC);