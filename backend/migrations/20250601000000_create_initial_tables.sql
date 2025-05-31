-- Create users table
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    email VARCHAR(255) UNIQUE NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create vps table
CREATE TABLE vps (
    id SERIAL PRIMARY KEY,
    user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    ip_address VARCHAR(255) NOT NULL,
    os_type VARCHAR(100), -- e.g., "Ubuntu 22.04 LTS", "CentOS 7"
    agent_secret VARCHAR(255) NOT NULL UNIQUE, -- Secret for agent authentication, should be unique
    status VARCHAR(50) NOT NULL,       -- e.g., "online", "offline", "pending_install", "error"
    metadata JSONB, -- For storing additional info like vendor details, notes, etc.
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Optional: Add indexes for frequently queried columns
CREATE INDEX idx_vps_user_id ON vps(user_id);
CREATE INDEX idx_vps_ip_address ON vps(ip_address);

-- Placeholder for other tables to be created in subsequent migrations:
-- performance_metrics (hypertable)
-- docker_containers
-- docker_metrics (hypertable)
-- tasks
-- task_runs
-- alert_rules
-- alert_events
-- vps_monthly_traffic