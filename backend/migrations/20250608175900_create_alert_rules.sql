-- Create the alert_rules table
CREATE TABLE alert_rules (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    vps_id INTEGER REFERENCES vps(id) ON DELETE SET NULL, -- Optional, for global rules
    metric_type VARCHAR(100) NOT NULL, -- e.g., 'cpu_usage_percent', 'memory_usage_percent'
    threshold DOUBLE PRECISION NOT NULL,
    comparison_operator VARCHAR(10) NOT NULL, -- e.g., '>', '<', '='
    duration_seconds INTEGER NOT NULL, -- How long the condition must persist
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    last_triggered_at TIMESTAMPTZ NULL,
    cooldown_seconds INTEGER NOT NULL DEFAULT 300, -- Default to 300 seconds (5 minutes)
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW() -- App layer will handle updates
);

-- Create the alert_rule_channels join table for M:N relationship
CREATE TABLE alert_rule_channels (
    alert_rule_id INTEGER NOT NULL REFERENCES alert_rules(id) ON DELETE CASCADE,
    channel_id INTEGER NOT NULL REFERENCES notification_channels(id) ON DELETE CASCADE,
    PRIMARY KEY (alert_rule_id, channel_id)
);

-- Indexes for foreign keys for performance
CREATE INDEX idx_alert_rules_user_id ON alert_rules(user_id);
CREATE INDEX idx_alert_rules_vps_id ON alert_rules(vps_id);
CREATE INDEX idx_alert_rule_channels_alert_rule_id ON alert_rule_channels(alert_rule_id);
CREATE INDEX idx_alert_rule_channels_channel_id ON alert_rule_channels(channel_id);