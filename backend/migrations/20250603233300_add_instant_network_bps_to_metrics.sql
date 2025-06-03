-- Add columns for instantaneous network speed (Bytes Per Second) calculated by the agent
ALTER TABLE performance_metrics
ADD COLUMN network_rx_instant_bps BIGINT NOT NULL DEFAULT 0,
ADD COLUMN network_tx_instant_bps BIGINT NOT NULL DEFAULT 0;

-- Add comments to clarify the purpose of the columns
COMMENT ON COLUMN performance_metrics.network_rx_instant_bps IS 'Instantaneous network receive speed in Bytes Per Second, calculated by the agent based on the default interface.';
COMMENT ON COLUMN performance_metrics.network_tx_instant_bps IS 'Instantaneous network transmit speed in Bytes Per Second, calculated by the agent based on the default interface.';
-- Also clarify the existing columns now store cumulative data
COMMENT ON COLUMN performance_metrics.network_rx_bps IS 'Cumulative network received bytes for the default interface since agent start or counter reset. Stored here for historical reasons/potential future use.';
COMMENT ON COLUMN performance_metrics.network_tx_bps IS 'Cumulative network transmitted bytes for the default interface since agent start or counter reset. Stored here for historical reasons/potential future use.';