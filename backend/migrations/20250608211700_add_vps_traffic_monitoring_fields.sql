-- Add new columns to the vps table for traffic monitoring
ALTER TABLE vps
ADD COLUMN traffic_limit_bytes BIGINT,
ADD COLUMN traffic_billing_rule VARCHAR(20),
ADD COLUMN traffic_current_cycle_rx_bytes BIGINT NOT NULL DEFAULT 0,
ADD COLUMN traffic_current_cycle_tx_bytes BIGINT NOT NULL DEFAULT 0,
ADD COLUMN last_processed_cumulative_rx BIGINT NOT NULL DEFAULT 0,
ADD COLUMN last_processed_cumulative_tx BIGINT NOT NULL DEFAULT 0,
ADD COLUMN traffic_last_reset_at TIMESTAMPTZ,
ADD COLUMN traffic_reset_config_type VARCHAR(50),
ADD COLUMN traffic_reset_config_value VARCHAR(100),
ADD COLUMN next_traffic_reset_at TIMESTAMPTZ;

-- It might be beneficial to add indexes on columns that will be frequently queried,
-- for example, next_traffic_reset_at if the cron job queries it often.
-- CREATE INDEX IF NOT EXISTS idx_vps_next_traffic_reset_at ON vps (next_traffic_reset_at);
-- This can be decided based on query patterns later.