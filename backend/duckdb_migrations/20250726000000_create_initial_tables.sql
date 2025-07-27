-- DuckDB compatible schema based on the original PostgreSQL migration (v2)

CREATE TABLE IF NOT EXISTS users (
    id                      INTEGER PRIMARY KEY,
    username                VARCHAR(255) NOT NULL UNIQUE,
    password_hash           VARCHAR(255),
    created_at              TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    role                    VARCHAR NOT NULL DEFAULT 'user',
    password_login_disabled BOOLEAN NOT NULL DEFAULT false,
    theme_mode              VARCHAR(50) NOT NULL DEFAULT 'system' CHECK(theme_mode IN ('light', 'dark', 'system')),
    language                VARCHAR(20) NOT NULL DEFAULT 'auto',
    active_theme_id         INTEGER -- Changed from UUID to INTEGER for simplicity
);

CREATE TABLE IF NOT EXISTS vps (
    id                             INTEGER PRIMARY KEY,
    user_id                        INTEGER NOT NULL,
    name                           VARCHAR(255) NOT NULL,
    ip_address                     VARCHAR(255),
    os_type                        VARCHAR(100),
    agent_secret                   VARCHAR(255) NOT NULL UNIQUE,
    status                         VARCHAR(50) NOT NULL,
    metadata                       JSON,
    created_at                     TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    updated_at                     TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    "group"                        VARCHAR(255),
    agent_config_override          JSON,
    config_status                  VARCHAR(50) NOT NULL DEFAULT 'unknown',
    last_config_update_at          TIMESTAMPTZ,
    last_config_error              TEXT,
    traffic_limit_bytes            BIGINT,
    traffic_billing_rule           VARCHAR(20),
    traffic_current_cycle_rx_bytes BIGINT NOT NULL DEFAULT 0,
    traffic_current_cycle_tx_bytes BIGINT NOT NULL DEFAULT 0,
    last_processed_cumulative_rx   BIGINT NOT NULL DEFAULT 0,
    last_processed_cumulative_tx   BIGINT NOT NULL DEFAULT 0,
    traffic_last_reset_at          TIMESTAMPTZ,
    traffic_reset_config_type      VARCHAR(50),
    traffic_reset_config_value     VARCHAR(100),
    next_traffic_reset_at          TIMESTAMPTZ,
    agent_version                  VARCHAR(255)
);

CREATE INDEX IF NOT EXISTS idx_vps_user_id ON vps (user_id);
CREATE INDEX IF NOT EXISTS idx_vps_ip_address ON vps (ip_address);
CREATE INDEX IF NOT EXISTS idx_vps_config_status ON vps (config_status);

-- This table will be our "hot" storage for recent metrics.
CREATE TABLE IF NOT EXISTS performance_metrics (
    time                             TIMESTAMPTZ NOT NULL,
    vps_id                           INTEGER NOT NULL,
    cpu_usage_percent                DOUBLE NOT NULL,
    memory_usage_bytes               BIGINT NOT NULL,
    memory_total_bytes               BIGINT NOT NULL,
    disk_io_read_bps                 BIGINT NOT NULL,
    disk_io_write_bps                BIGINT NOT NULL,
    network_rx_cumulative            BIGINT NOT NULL,
    network_tx_cumulative            BIGINT NOT NULL,
    swap_usage_bytes                 BIGINT NOT NULL DEFAULT 0,
    swap_total_bytes                 BIGINT NOT NULL DEFAULT 0,
    uptime_seconds                   BIGINT NOT NULL DEFAULT 0,
    total_processes_count            INTEGER NOT NULL DEFAULT 0,
    running_processes_count          INTEGER NOT NULL DEFAULT 0,
    tcp_established_connection_count INTEGER NOT NULL DEFAULT 0,
    network_rx_instant_bps           BIGINT NOT NULL DEFAULT 0,
    network_tx_instant_bps           BIGINT NOT NULL DEFAULT 0,
    total_disk_space_bytes           BIGINT NOT NULL DEFAULT 0,
    used_disk_space_bytes            BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (vps_id, time)
);

CREATE INDEX IF NOT EXISTS idx_performance_metrics_vps_id_time_desc ON performance_metrics (vps_id ASC, time DESC);

-- Summary tables for aggregated performance metrics, mirroring TimescaleDB's continuous aggregates.

CREATE TABLE IF NOT EXISTS performance_metrics_summary_1m (
    vps_id                                 INTEGER NOT NULL,
    time                                   TIMESTAMPTZ NOT NULL,
    avg_cpu_usage_percent                  DOUBLE,
    max_cpu_usage_percent                  DOUBLE,
    min_cpu_usage_percent                  DOUBLE,
    avg_memory_usage_bytes                 DOUBLE,
    max_memory_usage_bytes                 BIGINT,
    min_memory_usage_bytes                 BIGINT,
    max_memory_total_bytes                 BIGINT,
    avg_swap_usage_bytes                   DOUBLE,
    max_swap_usage_bytes                   BIGINT,
    min_swap_usage_bytes                   BIGINT,
    max_swap_total_bytes                   BIGINT,
    avg_disk_io_read_bps                   DOUBLE,
    max_disk_io_read_bps                   BIGINT,
    min_disk_io_read_bps                   BIGINT,
    avg_disk_io_write_bps                  DOUBLE,
    max_disk_io_write_bps                  BIGINT,
    min_disk_io_write_bps                  BIGINT,
    avg_total_disk_space_bytes             DOUBLE,
    max_total_disk_space_bytes             BIGINT,
    min_total_disk_space_bytes             BIGINT,
    avg_used_disk_space_bytes              DOUBLE,
    max_used_disk_space_bytes              BIGINT,
    min_used_disk_space_bytes              BIGINT,
    avg_network_rx_instant_bps             DOUBLE,
    max_network_rx_instant_bps             BIGINT,
    min_network_rx_instant_bps             BIGINT,
    avg_network_tx_instant_bps             DOUBLE,
    max_network_tx_instant_bps             BIGINT,
    min_network_tx_instant_bps             BIGINT,
    last_network_rx_cumulative           BIGINT,
    last_network_tx_cumulative           BIGINT,
    max_uptime_seconds                     BIGINT,
    avg_total_processes_count              DOUBLE,
    max_total_processes_count              INTEGER,
    avg_running_processes_count            DOUBLE,
    max_running_processes_count            INTEGER,
    avg_tcp_established_connection_count   DOUBLE,
    max_tcp_established_connection_count   INTEGER,
    PRIMARY KEY (vps_id, time)
);

CREATE TABLE IF NOT EXISTS performance_metrics_summary_1h (
    vps_id                                 INTEGER NOT NULL,
    time                                   TIMESTAMPTZ NOT NULL,
    avg_cpu_usage_percent                  DOUBLE,
    max_cpu_usage_percent                  DOUBLE,
    min_cpu_usage_percent                  DOUBLE,
    avg_memory_usage_bytes                 DOUBLE,
    max_memory_usage_bytes                 BIGINT,
    min_memory_usage_bytes                 BIGINT,
    max_memory_total_bytes                 BIGINT,
    avg_swap_usage_bytes                   DOUBLE,
    max_swap_usage_bytes                   BIGINT,
    min_swap_usage_bytes                   BIGINT,
    max_swap_total_bytes                   BIGINT,
    avg_disk_io_read_bps                   DOUBLE,
    max_disk_io_read_bps                   BIGINT,
    min_disk_io_read_bps                   BIGINT,
    avg_disk_io_write_bps                  DOUBLE,
    max_disk_io_write_bps                  BIGINT,
    min_disk_io_write_bps                  BIGINT,
    avg_total_disk_space_bytes             DOUBLE,
    max_total_disk_space_bytes             BIGINT,
    min_total_disk_space_bytes             BIGINT,
    avg_used_disk_space_bytes              DOUBLE,
    max_used_disk_space_bytes              BIGINT,
    min_used_disk_space_bytes              BIGINT,
    avg_network_rx_instant_bps             DOUBLE,
    max_network_rx_instant_bps             BIGINT,
    min_network_rx_instant_bps             BIGINT,
    avg_network_tx_instant_bps             DOUBLE,
    max_network_tx_instant_bps             BIGINT,
    min_network_tx_instant_bps             BIGINT,
    last_network_rx_cumulative           BIGINT,
    last_network_tx_cumulative           BIGINT,
    max_uptime_seconds                     BIGINT,
    avg_total_processes_count              DOUBLE,
    max_total_processes_count              INTEGER,
    avg_running_processes_count            DOUBLE,
    max_running_processes_count            INTEGER,
    avg_tcp_established_connection_count   DOUBLE,
    max_tcp_established_connection_count   INTEGER,
    PRIMARY KEY (vps_id, time)
);

CREATE TABLE IF NOT EXISTS performance_metrics_summary_1d (
    vps_id                                 INTEGER NOT NULL,
    time                                   TIMESTAMPTZ NOT NULL,
    avg_cpu_usage_percent                  DOUBLE,
    max_cpu_usage_percent                  DOUBLE,
    min_cpu_usage_percent                  DOUBLE,
    avg_memory_usage_bytes                 DOUBLE,
    max_memory_usage_bytes                 BIGINT,
    min_memory_usage_bytes                 BIGINT,
    max_memory_total_bytes                 BIGINT,
    avg_swap_usage_bytes                   DOUBLE,
    max_swap_usage_bytes                   BIGINT,
    min_swap_usage_bytes                   BIGINT,
    max_swap_total_bytes                   BIGINT,
    avg_disk_io_read_bps                   DOUBLE,
    max_disk_io_read_bps                   BIGINT,
    min_disk_io_read_bps                   BIGINT,
    avg_disk_io_write_bps                  DOUBLE,
    max_disk_io_write_bps                  BIGINT,
    min_disk_io_write_bps                  BIGINT,
    avg_total_disk_space_bytes             DOUBLE,
    max_total_disk_space_bytes             BIGINT,
    min_total_disk_space_bytes             BIGINT,
    avg_used_disk_space_bytes              DOUBLE,
    max_used_disk_space_bytes              BIGINT,
    min_used_disk_space_bytes              BIGINT,
    avg_network_rx_instant_bps             DOUBLE,
    max_network_rx_instant_bps             BIGINT,
    min_network_rx_instant_bps             BIGINT,
    avg_network_tx_instant_bps             DOUBLE,
    max_network_tx_instant_bps             BIGINT,
    min_network_tx_instant_bps             BIGINT,
    last_network_rx_cumulative           BIGINT,
    last_network_tx_cumulative           BIGINT,
    max_uptime_seconds                     BIGINT,
    avg_total_processes_count              DOUBLE,
    max_total_processes_count              INTEGER,
    avg_running_processes_count            DOUBLE,
    max_running_processes_count            INTEGER,
    avg_tcp_established_connection_count   DOUBLE,
    max_tcp_established_connection_count   INTEGER,
    PRIMARY KEY (vps_id, time)
);

CREATE TABLE IF NOT EXISTS settings (
    key        VARCHAR(255) NOT NULL PRIMARY KEY,
    value      JSON NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp
);

CREATE TABLE IF NOT EXISTS tags (
    id         INTEGER PRIMARY KEY,
    user_id    INTEGER NOT NULL,
    name       VARCHAR(255) NOT NULL,
    color      VARCHAR(7) NOT NULL DEFAULT '#ffffff',
    icon       VARCHAR(255),
    url        VARCHAR(2048),
    is_visible BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    UNIQUE (user_id, name)
);

CREATE TABLE IF NOT EXISTS vps_tags (
    vps_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    PRIMARY KEY (vps_id, tag_id)
);

CREATE TABLE IF NOT EXISTS notification_channels (
    id           INTEGER PRIMARY KEY,
    user_id      INTEGER NOT NULL,
    name         VARCHAR(255) NOT NULL,
    channel_type VARCHAR(50) NOT NULL,
    config       BLOB NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT current_timestamp
);

CREATE INDEX IF NOT EXISTS idx_notification_channels_user_id ON notification_channels (user_id);

CREATE TABLE IF NOT EXISTS alert_rules (
    id                  INTEGER PRIMARY KEY,
    user_id             INTEGER NOT NULL,
    name                VARCHAR(255) NOT NULL,
    vps_id              INTEGER,
    metric_type         VARCHAR(100) NOT NULL,
    threshold           DOUBLE NOT NULL,
    comparison_operator VARCHAR(10) NOT NULL,
    duration_seconds    INTEGER NOT NULL,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    is_active           BOOLEAN NOT NULL DEFAULT true,
    last_triggered_at   TIMESTAMPTZ,
    cooldown_seconds    INTEGER NOT NULL DEFAULT 300
);

CREATE INDEX IF NOT EXISTS idx_alert_rules_user_id ON alert_rules (user_id);
CREATE INDEX IF NOT EXISTS idx_alert_rules_vps_id ON alert_rules (vps_id);

CREATE TABLE IF NOT EXISTS alert_rule_channels (
    alert_rule_id INTEGER NOT NULL,
    channel_id    INTEGER NOT NULL,
    PRIMARY KEY (alert_rule_id, channel_id)
);

CREATE TABLE IF NOT EXISTS vps_renewal_info (
    vps_id                     INTEGER NOT NULL PRIMARY KEY,
    renewal_cycle              TEXT,
    renewal_cycle_custom_days  INTEGER,
    renewal_price              DOUBLE,
    renewal_currency           TEXT,
    next_renewal_date          TIMESTAMPTZ,
    last_renewal_date          TIMESTAMPTZ,
    service_start_date         TIMESTAMPTZ,
    payment_method             TEXT,
    auto_renew_enabled         BOOLEAN DEFAULT false,
    renewal_notes              TEXT,
    reminder_active            BOOLEAN DEFAULT false,
    last_reminder_generated_at TIMESTAMPTZ,
    created_at                 TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    updated_at                 TIMESTAMPTZ NOT NULL DEFAULT current_timestamp
);

CREATE TABLE IF NOT EXISTS batch_command_tasks (
    batch_command_id         UUID NOT NULL PRIMARY KEY,
    original_request_payload JSON NOT NULL,
    status                   VARCHAR(50) NOT NULL,
    execution_alias          VARCHAR(255),
    user_id                  INTEGER NOT NULL,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    completed_at             TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_batch_command_tasks_status ON batch_command_tasks (status);
CREATE INDEX IF NOT EXISTS idx_batch_command_tasks_user_id ON batch_command_tasks (user_id);
CREATE INDEX IF NOT EXISTS idx_batch_command_tasks_created_at ON batch_command_tasks (created_at DESC);

CREATE TABLE IF NOT EXISTS child_command_tasks (
    child_command_id   UUID NOT NULL PRIMARY KEY,
    batch_command_id   UUID NOT NULL,
    vps_id             INTEGER NOT NULL,
    status             VARCHAR(50) NOT NULL,
    exit_code          INTEGER,
    error_message      TEXT,
    stdout_log_path    VARCHAR(1024),
    stderr_log_path    VARCHAR(1024),
    last_output_at     TIMESTAMPTZ,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    agent_started_at   TIMESTAMPTZ,
    agent_completed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_child_command_tasks_batch_command_id ON child_command_tasks (batch_command_id);
CREATE INDEX IF NOT EXISTS idx_child_command_tasks_vps_id ON child_command_tasks (vps_id);
CREATE INDEX IF NOT EXISTS idx_child_command_tasks_status ON child_command_tasks (status);
CREATE INDEX IF NOT EXISTS idx_child_command_tasks_created_at ON child_command_tasks (created_at DESC);

CREATE TABLE IF NOT EXISTS service_monitors (
    id                INTEGER PRIMARY KEY,
    user_id           INTEGER NOT NULL,
    name              VARCHAR NOT NULL,
    monitor_type      VARCHAR NOT NULL,
    target            VARCHAR NOT NULL,
    frequency_seconds INTEGER NOT NULL DEFAULT 60,
    timeout_seconds   INTEGER NOT NULL DEFAULT 10,
    is_active         BOOLEAN NOT NULL DEFAULT true,
    monitor_config    JSON,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    assignment_type   VARCHAR(255) NOT NULL DEFAULT 'INCLUSIVE'
);

CREATE TABLE IF NOT EXISTS service_monitor_agents (
    monitor_id INTEGER NOT NULL,
    vps_id     INTEGER NOT NULL,
    PRIMARY KEY (monitor_id, vps_id)
);

CREATE TABLE IF NOT EXISTS service_monitor_tags (
    monitor_id INTEGER NOT NULL,
    tag_id     INTEGER NOT NULL,
    PRIMARY KEY (monitor_id, tag_id)
);

CREATE TABLE IF NOT EXISTS service_monitor_results (
    time       TIMESTAMPTZ NOT NULL,
    monitor_id INTEGER NOT NULL,
    agent_id   INTEGER NOT NULL,
    is_up      BOOLEAN NOT NULL,
    latency_ms INTEGER,
    details    JSON
);

CREATE INDEX IF NOT EXISTS service_monitor_results_monitor_id_time_idx ON service_monitor_results (monitor_id ASC, time DESC);
CREATE INDEX IF NOT EXISTS service_monitor_results_agent_id_time_idx ON service_monitor_results (agent_id ASC, time DESC);

CREATE TABLE IF NOT EXISTS command_scripts (
    id                INTEGER PRIMARY KEY,
    user_id           INTEGER NOT NULL,
    name              VARCHAR(255) NOT NULL,
    description       TEXT,
    script_content    TEXT NOT NULL,
    working_directory VARCHAR(255) NOT NULL,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    language          TEXT NOT NULL DEFAULT 'shell',
    UNIQUE (user_id, name)
);

CREATE INDEX IF NOT EXISTS idx_command_scripts_user_id ON command_scripts (user_id);

CREATE TABLE IF NOT EXISTS oauth2_providers (
    id                INTEGER PRIMARY KEY,
    provider_name     VARCHAR(255) NOT NULL UNIQUE,
    client_id         VARCHAR(255) NOT NULL,
    client_secret     TEXT NOT NULL,
    auth_url          VARCHAR(255) NOT NULL,
    token_url         VARCHAR(255) NOT NULL,
    user_info_url     VARCHAR(255) NOT NULL,
    scopes            TEXT,
    user_info_mapping JSON,
    enabled           BOOLEAN NOT NULL DEFAULT true,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    icon_url          TEXT
);

CREATE INDEX IF NOT EXISTS idx_oauth2_providers_provider_name ON oauth2_providers (provider_name);

CREATE TABLE IF NOT EXISTS user_identity_providers (
    id               INTEGER PRIMARY KEY,
    user_id          INTEGER NOT NULL,
    provider_name    VARCHAR(255) NOT NULL,
    provider_user_id VARCHAR(255) NOT NULL,
    created_at       TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    updated_at       TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    UNIQUE (provider_name, provider_user_id)
);

CREATE INDEX IF NOT EXISTS idx_user_identity_providers_user_id ON user_identity_providers (user_id);

CREATE TABLE IF NOT EXISTS themes (
    id          INTEGER PRIMARY KEY,
    user_id     INTEGER NOT NULL,
    name        VARCHAR(255) NOT NULL,
    is_official BOOLEAN NOT NULL DEFAULT false,
    css         TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT current_timestamp
);

CREATE INDEX IF NOT EXISTS idx_themes_user_id ON themes (user_id);
CREATE INDEX IF NOT EXISTS idx_themes_is_official ON themes (is_official);

