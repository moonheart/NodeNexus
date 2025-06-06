-- Add settings table for global configurations
CREATE TABLE settings (
    key VARCHAR(255) PRIMARY KEY,
    value JSONB NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Add a default global agent config.
-- These values are placeholders and can be adjusted in the settings UI.
INSERT INTO settings (key, value) VALUES
('global_agent_config', '{
    "metrics_collect_interval_seconds": 60,
    "metrics_upload_batch_max_size": 100,
    "metrics_upload_interval_seconds": 300,
    "docker_info_collect_interval_seconds": 600,
    "docker_info_upload_interval_seconds": 900,
    "generic_metrics_upload_batch_max_size": 100,
    "generic_metrics_upload_interval_seconds": 300,
    "feature_flags": {},
    "log_level": "info",
    "heartbeat_interval_seconds": 30
}');

-- Modify the vps table to support per-vps config overrides and status tracking
ALTER TABLE vps
ADD COLUMN agent_config_override JSONB,
ADD COLUMN config_status VARCHAR(50) NOT NULL DEFAULT 'unknown',
ADD COLUMN last_config_update_at TIMESTAMPTZ,
ADD COLUMN last_config_error TEXT;

-- Add an index on the new status column for faster lookups
CREATE INDEX idx_vps_config_status ON vps(config_status);
