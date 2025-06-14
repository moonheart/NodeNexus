-- backend/migrations/{timestamp}_create_service_monitor_tables.sql

-- Table to store the configuration for each monitoring task
CREATE TABLE service_monitors (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR NOT NULL,
    monitor_type VARCHAR NOT NULL, -- 'http', 'ping', 'tcp'
    target VARCHAR NOT NULL,
    frequency_seconds INTEGER NOT NULL DEFAULT 60,
    timeout_seconds INTEGER NOT NULL DEFAULT 10,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    monitor_config JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Junction table for direct monitor-to-agent assignments (many-to-many)
CREATE TABLE service_monitor_agents (
    monitor_id INTEGER NOT NULL REFERENCES service_monitors(id) ON DELETE CASCADE,
    vps_id INTEGER NOT NULL REFERENCES vps(id) ON DELETE CASCADE,
    PRIMARY KEY (monitor_id, vps_id)
);

-- Junction table for tag-based monitor assignments (many-to-many)
CREATE TABLE service_monitor_tags (
    monitor_id INTEGER NOT NULL REFERENCES service_monitors(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (monitor_id, tag_id)
);

-- Table to store the results of each monitor check (TimescaleDB Hypertable)
CREATE TABLE service_monitor_results (
    "time" TIMESTAMPTZ NOT NULL,
    monitor_id INTEGER NOT NULL REFERENCES service_monitors(id) ON DELETE CASCADE,
    agent_id INTEGER NOT NULL REFERENCES vps(id) ON DELETE CASCADE,
    is_up BOOLEAN NOT NULL,
    latency_ms INTEGER NOT NULL,
    details JSONB
);

-- Convert the results table to a TimescaleDB hypertable
SELECT create_hypertable('service_monitor_results', 'time');

-- Add indexes for efficient querying
CREATE INDEX ON service_monitor_results (monitor_id, "time" DESC);
CREATE INDEX ON service_monitor_results (agent_id, "time" DESC);