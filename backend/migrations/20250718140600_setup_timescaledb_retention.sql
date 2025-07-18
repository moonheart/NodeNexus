-- Migration to set up TimescaleDB for performance data, consolidating disk usage into the main metrics table.
-- This script is idempotent and handles existing data safely.

-- Step 1: Enable the TimescaleDB extension.
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Step 2: Add new disk usage columns to performance_metrics table.
-- This is done in a multi-step, safe way to support tables with existing data.

-- 2.1: Add columns, allowing NULLs for now.
ALTER TABLE public.performance_metrics ADD COLUMN IF NOT EXISTS total_disk_space_bytes BIGINT;
ALTER TABLE public.performance_metrics ADD COLUMN IF NOT EXISTS used_disk_space_bytes BIGINT;

-- 2.2: Update existing rows where the new columns are NULL, setting them to 0.
UPDATE public.performance_metrics SET total_disk_space_bytes = 0 WHERE total_disk_space_bytes IS NULL;
UPDATE public.performance_metrics SET used_disk_space_bytes = 0 WHERE used_disk_space_bytes IS NULL;

-- 2.3: Set the default for future rows and enforce the NOT NULL constraint.
ALTER TABLE public.performance_metrics ALTER COLUMN total_disk_space_bytes SET DEFAULT 0;
ALTER TABLE public.performance_metrics ALTER COLUMN used_disk_space_bytes SET DEFAULT 0;
ALTER TABLE public.performance_metrics ALTER COLUMN total_disk_space_bytes SET NOT NULL;
ALTER TABLE public.performance_metrics ALTER COLUMN used_disk_space_bytes SET NOT NULL;


-- Step 3: Drop the legacy 'id' primary key and set a new composite primary key.
-- This is necessary for TimescaleDB and removes the now-redundant 'id' column.
DO $$
BEGIN
    -- Drop the old primary key constraint if it exists (default name from sqlx)
    IF EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE table_name = 'performance_metrics' AND constraint_name = 'performance_metrics_pkey'
    ) THEN
        ALTER TABLE public.performance_metrics DROP CONSTRAINT performance_metrics_pkey;
    END IF;

    -- Drop the 'id' column if it exists
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'performance_metrics' AND column_name = 'id'
    ) THEN
        ALTER TABLE public.performance_metrics DROP COLUMN id;
    END IF;

    -- Add the composite primary key if it doesn't already exist.
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.table_constraints
        WHERE table_name = 'performance_metrics' AND constraint_type = 'PRIMARY KEY'
    ) THEN
        ALTER TABLE public.performance_metrics ADD PRIMARY KEY (vps_id, time);
    END IF;
END $$;


-- Step 4: Convert performance_metrics to a Hypertable.
-- We partition by both time and space (vps_id) for better performance.
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM timescaledb_information.hypertables WHERE hypertable_name = 'performance_metrics') THEN
        PERFORM create_hypertable('performance_metrics', 'time', 'vps_id', 4, if_not_exists => TRUE);
    END IF;
END $$;


-- Step 5: Create Continuous Aggregates for performance_metrics.
-- These views now include the consolidated disk space metrics.

-- 4.1: Minute-level aggregate
DROP MATERIALIZED VIEW IF EXISTS performance_metrics_summary_1m CASCADE;
CREATE MATERIALIZED VIEW performance_metrics_summary_1m
WITH (timescaledb.continuous) AS
SELECT
    vps_id,
    time_bucket('1 minute', time) AS bucket,
    -- CPU, Memory, Swap, Disk IO, Network, System Processes...
    AVG(cpu_usage_percent) AS avg_cpu_usage_percent, MAX(cpu_usage_percent) AS max_cpu_usage_percent, MIN(cpu_usage_percent) AS min_cpu_usage_percent,
    AVG(memory_usage_bytes) AS avg_memory_usage_bytes, MAX(memory_usage_bytes) AS max_memory_usage_bytes, MIN(memory_usage_bytes) AS min_memory_usage_bytes, MAX(memory_total_bytes) AS max_memory_total_bytes,
    AVG(swap_usage_bytes) AS avg_swap_usage_bytes, MAX(swap_usage_bytes) AS max_swap_usage_bytes, MIN(swap_usage_bytes) AS min_swap_usage_bytes, MAX(swap_total_bytes) AS max_swap_total_bytes,
    AVG(disk_io_read_bps) AS avg_disk_io_read_bps, MAX(disk_io_read_bps) AS max_disk_io_read_bps, MIN(disk_io_read_bps) AS min_disk_io_read_bps,
    AVG(disk_io_write_bps) AS avg_disk_io_write_bps, MAX(disk_io_write_bps) AS max_disk_io_write_bps, MIN(disk_io_write_bps) AS min_disk_io_write_bps,
    -- New consolidated disk space metrics
    AVG(total_disk_space_bytes) AS avg_total_disk_space_bytes, MAX(total_disk_space_bytes) AS max_total_disk_space_bytes, MIN(total_disk_space_bytes) AS min_total_disk_space_bytes,
    AVG(used_disk_space_bytes) AS avg_used_disk_space_bytes, MAX(used_disk_space_bytes) AS max_used_disk_space_bytes, MIN(used_disk_space_bytes) AS min_used_disk_space_bytes,
    -- Network and System
    AVG(network_rx_instant_bps) AS avg_network_rx_instant_bps, MAX(network_rx_instant_bps) AS max_network_rx_instant_bps, MIN(network_rx_instant_bps) AS min_network_rx_instant_bps,
    AVG(network_tx_instant_bps) AS avg_network_tx_instant_bps, MAX(network_tx_instant_bps) AS max_network_tx_instant_bps, MIN(network_tx_instant_bps) AS min_network_tx_instant_bps,
    LAST(network_rx_cumulative, time) as last_network_rx_cumulative, LAST(network_tx_cumulative, time) as last_network_tx_cumulative,
    MAX(uptime_seconds) AS max_uptime_seconds,
    AVG(total_processes_count) AS avg_total_processes_count, MAX(total_processes_count) AS max_total_processes_count,
    AVG(running_processes_count) AS avg_running_processes_count, MAX(running_processes_count) AS max_running_processes_count,
    AVG(tcp_established_connection_count) AS avg_tcp_established_connection_count, MAX(tcp_established_connection_count) AS max_tcp_established_connection_count
FROM public.performance_metrics GROUP BY vps_id, bucket;

-- 4.2: Hour-level aggregate
DROP MATERIALIZED VIEW IF EXISTS performance_metrics_summary_1h CASCADE;
CREATE MATERIALIZED VIEW performance_metrics_summary_1h WITH (timescaledb.continuous) AS
SELECT
    vps_id,
    time_bucket('1 hour', bucket) AS bucket,
    -- Aggregate all metrics from the 1-minute view
    AVG(avg_cpu_usage_percent) AS avg_cpu_usage_percent, MAX(max_cpu_usage_percent) AS max_cpu_usage_percent, MIN(min_cpu_usage_percent) AS min_cpu_usage_percent,
    AVG(avg_memory_usage_bytes) AS avg_memory_usage_bytes, MAX(max_memory_usage_bytes) AS max_memory_usage_bytes, MIN(min_memory_usage_bytes) AS min_memory_usage_bytes, MAX(max_memory_total_bytes) AS max_memory_total_bytes,
    AVG(avg_swap_usage_bytes) AS avg_swap_usage_bytes, MAX(max_swap_usage_bytes) AS max_swap_usage_bytes, MIN(min_swap_usage_bytes) AS min_swap_usage_bytes, MAX(max_swap_total_bytes) AS max_swap_total_bytes,
    AVG(avg_disk_io_read_bps) AS avg_disk_io_read_bps, MAX(max_disk_io_read_bps) AS max_disk_io_read_bps, MIN(min_disk_io_read_bps) AS min_disk_io_read_bps,
    AVG(avg_disk_io_write_bps) AS avg_disk_io_write_bps, MAX(max_disk_io_write_bps) AS max_disk_io_write_bps, MIN(min_disk_io_write_bps) AS min_disk_io_write_bps,
    AVG(avg_total_disk_space_bytes) AS avg_total_disk_space_bytes, MAX(max_total_disk_space_bytes) AS max_total_disk_space_bytes, MIN(min_total_disk_space_bytes) AS min_total_disk_space_bytes,
    AVG(avg_used_disk_space_bytes) AS avg_used_disk_space_bytes, MAX(max_used_disk_space_bytes) AS max_used_disk_space_bytes, MIN(min_used_disk_space_bytes) AS min_used_disk_space_bytes,
    AVG(avg_network_rx_instant_bps) AS avg_network_rx_instant_bps, MAX(max_network_rx_instant_bps) AS max_network_rx_instant_bps, MIN(min_network_rx_instant_bps) AS min_network_rx_instant_bps,
    AVG(avg_network_tx_instant_bps) AS avg_network_tx_instant_bps, MAX(max_network_tx_instant_bps) AS max_network_tx_instant_bps, MIN(min_network_tx_instant_bps) AS min_network_tx_instant_bps,
    LAST(last_network_rx_cumulative, bucket) as last_network_rx_cumulative, LAST(last_network_tx_cumulative, bucket) as last_network_tx_cumulative,
    MAX(max_uptime_seconds) AS max_uptime_seconds,
    AVG(avg_total_processes_count) AS avg_total_processes_count, MAX(max_total_processes_count) AS max_total_processes_count,
    AVG(avg_running_processes_count) AS avg_running_processes_count, MAX(max_running_processes_count) AS max_running_processes_count,
    AVG(avg_tcp_established_connection_count) AS avg_tcp_established_connection_count, MAX(max_tcp_established_connection_count) AS max_tcp_established_connection_count
FROM public.performance_metrics_summary_1m GROUP BY 1, 2;

-- 4.3: Day-level aggregate
DROP MATERIALIZED VIEW IF EXISTS performance_metrics_summary_1d CASCADE;
CREATE MATERIALIZED VIEW performance_metrics_summary_1d WITH (timescaledb.continuous) AS
SELECT
    vps_id,
    time_bucket('1 day', bucket) AS bucket,
    -- Aggregate all metrics from the 1-hour view
    AVG(avg_cpu_usage_percent) AS avg_cpu_usage_percent, MAX(max_cpu_usage_percent) AS max_cpu_usage_percent, MIN(min_cpu_usage_percent) AS min_cpu_usage_percent,
    AVG(avg_memory_usage_bytes) AS avg_memory_usage_bytes, MAX(max_memory_usage_bytes) AS max_memory_usage_bytes, MIN(min_memory_usage_bytes) AS min_memory_usage_bytes, MAX(max_memory_total_bytes) AS max_memory_total_bytes,
    AVG(avg_swap_usage_bytes) AS avg_swap_usage_bytes, MAX(max_swap_usage_bytes) AS max_swap_usage_bytes, MIN(min_swap_usage_bytes) AS min_swap_usage_bytes, MAX(max_swap_total_bytes) AS max_swap_total_bytes,
    AVG(avg_disk_io_read_bps) AS avg_disk_io_read_bps, MAX(max_disk_io_read_bps) AS max_disk_io_read_bps, MIN(min_disk_io_read_bps) AS min_disk_io_read_bps,
    AVG(avg_disk_io_write_bps) AS avg_disk_io_write_bps, MAX(max_disk_io_write_bps) AS max_disk_io_write_bps, MIN(min_disk_io_write_bps) AS min_disk_io_write_bps,
    AVG(avg_total_disk_space_bytes) AS avg_total_disk_space_bytes, MAX(max_total_disk_space_bytes) AS max_total_disk_space_bytes, MIN(min_total_disk_space_bytes) AS min_total_disk_space_bytes,
    AVG(avg_used_disk_space_bytes) AS avg_used_disk_space_bytes, MAX(max_used_disk_space_bytes) AS max_used_disk_space_bytes, MIN(min_used_disk_space_bytes) AS min_used_disk_space_bytes,
    AVG(avg_network_rx_instant_bps) AS avg_network_rx_instant_bps, MAX(max_network_rx_instant_bps) AS max_network_rx_instant_bps, MIN(min_network_rx_instant_bps) AS min_network_rx_instant_bps,
    AVG(avg_network_tx_instant_bps) AS avg_network_tx_instant_bps, MAX(max_network_tx_instant_bps) AS max_network_tx_instant_bps, MIN(min_network_tx_instant_bps) AS min_network_tx_instant_bps,
    LAST(last_network_rx_cumulative, bucket) as last_network_rx_cumulative, LAST(last_network_tx_cumulative, bucket) as last_network_tx_cumulative,
    MAX(max_uptime_seconds) AS max_uptime_seconds,
    AVG(avg_total_processes_count) AS avg_total_processes_count, MAX(max_total_processes_count) AS max_total_processes_count,
    AVG(avg_running_processes_count) AS avg_running_processes_count, MAX(max_running_processes_count) AS max_running_processes_count,
    AVG(avg_tcp_established_connection_count) AS avg_tcp_established_connection_count, MAX(max_tcp_established_connection_count) AS max_tcp_established_connection_count
FROM public.performance_metrics_summary_1h GROUP BY 1, 2;


-- Step 5: Apply refresh and retention policies.

-- 5.1: Raw data retention (24 hours)
SELECT add_retention_policy('performance_metrics', INTERVAL '24 hours', if_not_exists => TRUE);

-- 5.2: Minute-level policies (7-day retention)
SELECT add_continuous_aggregate_policy('performance_metrics_summary_1m', '1 hour', '10 minutes', '1 minute', if_not_exists => TRUE);
SELECT add_retention_policy('performance_metrics_summary_1m', INTERVAL '7 days', if_not_exists => TRUE);

-- 5.3: Hour-level policies (30-day retention)
SELECT add_continuous_aggregate_policy('performance_metrics_summary_1h', '1 day', '1 hour', '5 minute', if_not_exists => TRUE);
SELECT add_retention_policy('performance_metrics_summary_1h', INTERVAL '30 days', if_not_exists => TRUE);

-- 5.4: Day-level policies (365-day retention)
SELECT add_continuous_aggregate_policy('performance_metrics_summary_1d', '1 month', '1 day', '1 hour', if_not_exists => TRUE);
SELECT add_retention_policy('performance_metrics_summary_1d', INTERVAL '365 days', if_not_exists => TRUE);

-- Step 6: Clean up the now-unused performance_disk_usages table.
-- We drop it completely as it's being replaced by columns in performance_metrics.
DROP TABLE IF EXISTS public.performance_disk_usages CASCADE;

-- Step 7: Clean up the now-unused performance_network_interface_stats table.
DROP TABLE IF EXISTS public.performance_network_interface_stats CASCADE;