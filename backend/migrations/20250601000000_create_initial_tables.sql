create table public.users
(
    id                      serial
        primary key,
    username                varchar(255)                                                 not null
        unique,
    password_hash           varchar(255),
    created_at              timestamp with time zone default now()                       not null,
    updated_at              timestamp with time zone default now()                       not null,
    role                    varchar                  default 'user'::character varying   not null,
    password_login_disabled boolean                  default false                       not null,
    theme_mode              varchar(50)              default 'system'::character varying not null
        constraint users_theme_mode_check
            check ((theme_mode)::text = ANY
                   ((ARRAY ['light'::character varying, 'dark'::character varying, 'system'::character varying])::text[])),
    language                varchar(20)              default 'auto'::character varying   not null,
    active_theme_id         uuid
);

alter table public.users
    owner to postgres;

create table public.vps
(
    id                             serial
        primary key,
    user_id                        integer                                                       not null
        references public.users
            on delete cascade,
    name                           varchar(255)                                                  not null,
    ip_address                     varchar(255),
    os_type                        varchar(100),
    agent_secret                   varchar(255)                                                  not null
        unique,
    status                         varchar(50)                                                   not null,
    metadata                       jsonb,
    created_at                     timestamp with time zone default now()                        not null,
    updated_at                     timestamp with time zone default now()                        not null,
    "group"                        varchar(255),
    agent_config_override          jsonb,
    config_status                  varchar(50)              default 'unknown'::character varying not null,
    last_config_update_at          timestamp with time zone,
    last_config_error              text,
    traffic_limit_bytes            bigint,
    traffic_billing_rule           varchar(20),
    traffic_current_cycle_rx_bytes bigint                   default 0                            not null,
    traffic_current_cycle_tx_bytes bigint                   default 0                            not null,
    last_processed_cumulative_rx   bigint                   default 0                            not null,
    last_processed_cumulative_tx   bigint                   default 0                            not null,
    traffic_last_reset_at          timestamp with time zone,
    traffic_reset_config_type      varchar(50),
    traffic_reset_config_value     varchar(100),
    next_traffic_reset_at          timestamp with time zone,
    agent_version                  varchar(255)
);

alter table public.vps
    owner to postgres;

create index idx_vps_user_id
    on public.vps (user_id);

create index idx_vps_ip_address
    on public.vps (ip_address);

create index idx_vps_config_status
    on public.vps (config_status);

create table public.performance_metrics
(
    time                             timestamp with time zone not null,
    vps_id                           integer                  not null
        references public.vps
            on delete cascade,
    cpu_usage_percent                double precision         not null,
    memory_usage_bytes               bigint                   not null,
    memory_total_bytes               bigint                   not null,
    disk_io_read_bps                 bigint                   not null,
    disk_io_write_bps                bigint                   not null,
    network_rx_cumulative            bigint                   not null,
    network_tx_cumulative            bigint                   not null,
    id                               serial
        primary key,
    swap_usage_bytes                 bigint  default 0        not null,
    swap_total_bytes                 bigint  default 0        not null,
    uptime_seconds                   bigint  default 0        not null,
    total_processes_count            integer default 0        not null,
    running_processes_count          integer default 0        not null,
    tcp_established_connection_count integer default 0        not null,
    network_rx_instant_bps           bigint  default 0        not null,
    network_tx_instant_bps           bigint  default 0        not null
);

comment on column public.performance_metrics.network_rx_cumulative is 'Cumulative network received bytes for the default interface since agent start or counter reset. Stored here for historical reasons/potential future use.';

comment on column public.performance_metrics.network_tx_cumulative is 'Cumulative network transmitted bytes for the default interface since agent start or counter reset. Stored here for historical reasons/potential future use.';

comment on column public.performance_metrics.network_rx_instant_bps is 'Instantaneous network receive speed in Bytes Per Second, calculated by the agent based on the default interface.';

comment on column public.performance_metrics.network_tx_instant_bps is 'Instantaneous network transmit speed in Bytes Per Second, calculated by the agent based on the default interface.';

alter table public.performance_metrics
    owner to postgres;

create index idx_performance_metrics_vps_id_time_desc
    on public.performance_metrics (vps_id asc, time desc);

create table public.performance_disk_usages
(
    id                    serial
        primary key,
    performance_metric_id integer          not null
        references public.performance_metrics
            on delete cascade,
    mount_point           text             not null,
    used_bytes            bigint           not null,
    total_bytes           bigint           not null,
    fstype                text,
    usage_percent         double precision not null,
    constraint uq_disk_usage_metric_mount
        unique (performance_metric_id, mount_point)
);

alter table public.performance_disk_usages
    owner to postgres;

create index idx_performance_disk_usages_metric_id
    on public.performance_disk_usages (performance_metric_id);

create table public.performance_network_interface_stats
(
    id                         serial
        primary key,
    performance_metric_id      integer not null
        references public.performance_metrics
            on delete cascade,
    interface_name             text    not null,
    rx_bytes_per_sec           bigint  not null,
    tx_bytes_per_sec           bigint  not null,
    rx_packets_per_sec         bigint  not null,
    tx_packets_per_sec         bigint  not null,
    rx_errors_total_cumulative bigint  not null,
    tx_errors_total_cumulative bigint  not null,
    constraint uq_network_stats_metric_interface
        unique (performance_metric_id, interface_name)
);

alter table public.performance_network_interface_stats
    owner to postgres;

create index idx_performance_network_interface_stats_metric_id
    on public.performance_network_interface_stats (performance_metric_id);

create table public.settings
(
    key        varchar(255)                           not null
        primary key,
    value      jsonb                                  not null,
    updated_at timestamp with time zone default now() not null
);

alter table public.settings
    owner to postgres;

create table public.tags
(
    id         serial
        primary key,
    user_id    integer                                                       not null
        references public.users
            on delete cascade,
    name       varchar(255)                                                  not null,
    color      varchar(7)               default '#ffffff'::character varying not null,
    icon       varchar(255),
    url        varchar(2048),
    is_visible boolean                  default true                         not null,
    created_at timestamp with time zone default now()                        not null,
    updated_at timestamp with time zone default now()                        not null,
    unique (user_id, name)
);

alter table public.tags
    owner to postgres;

create table public.vps_tags
(
    vps_id integer not null
        references public.vps
            on delete cascade,
    tag_id integer not null
        references public.tags
            on delete cascade,
    primary key (vps_id, tag_id)
);

alter table public.vps_tags
    owner to postgres;

create table public.notification_channels
(
    id           serial
        primary key,
    user_id      integer                                not null
        references public.users
            on delete cascade,
    name         varchar(255)                           not null,
    channel_type varchar(50)                            not null,
    config       bytea                                  not null,
    created_at   timestamp with time zone default now() not null,
    updated_at   timestamp with time zone default now() not null
);

comment on table public.notification_channels is '存储用户配置的推送渠道';

comment on column public.notification_channels.config is '加密后的渠道配置信息，以JSON格式存储';

alter table public.notification_channels
    owner to postgres;

create index idx_notification_channels_user_id
    on public.notification_channels (user_id);

create table public.alert_rules
(
    id                  serial
        primary key,
    user_id             integer                                not null
        references public.users
            on delete cascade,
    name                varchar(255)                           not null,
    vps_id              integer
                                                               references public.vps
                                                                   on delete set null,
    metric_type         varchar(100)                           not null,
    threshold           double precision                       not null,
    comparison_operator varchar(10)                            not null,
    duration_seconds    integer                                not null,
    created_at          timestamp with time zone default now() not null,
    updated_at          timestamp with time zone default now() not null,
    is_active           boolean                  default true  not null,
    last_triggered_at   timestamp with time zone,
    cooldown_seconds    integer                  default 300   not null
);

alter table public.alert_rules
    owner to postgres;

create index idx_alert_rules_user_id
    on public.alert_rules (user_id);

create index idx_alert_rules_vps_id
    on public.alert_rules (vps_id);

create table public.alert_rule_channels
(
    alert_rule_id integer not null
        references public.alert_rules
            on delete cascade,
    channel_id    integer not null
        references public.notification_channels
            on delete cascade,
    primary key (alert_rule_id, channel_id)
);

alter table public.alert_rule_channels
    owner to postgres;

create index idx_alert_rule_channels_alert_rule_id
    on public.alert_rule_channels (alert_rule_id);

create index idx_alert_rule_channels_channel_id
    on public.alert_rule_channels (channel_id);

create table public.vps_renewal_info
(
    vps_id                     integer                                            not null
        primary key
        references public.vps
            on delete cascade,
    renewal_cycle              text,
    renewal_cycle_custom_days  integer,
    renewal_price              double precision,
    renewal_currency           text,
    next_renewal_date          timestamp with time zone,
    last_renewal_date          timestamp with time zone,
    service_start_date         timestamp with time zone,
    payment_method             text,
    auto_renew_enabled         boolean                  default false,
    renewal_notes              text,
    reminder_active            boolean                  default false,
    last_reminder_generated_at timestamp with time zone,
    created_at                 timestamp with time zone default CURRENT_TIMESTAMP not null,
    updated_at                 timestamp with time zone default CURRENT_TIMESTAMP not null
);

alter table public.vps_renewal_info
    owner to postgres;

create table public.batch_command_tasks
(
    batch_command_id         uuid                                   not null
        primary key,
    original_request_payload jsonb                                  not null,
    status                   varchar(50)                            not null,
    execution_alias          varchar(255),
    user_id                  integer                                not null
        references public.users
            on delete cascade,
    created_at               timestamp with time zone default now() not null,
    updated_at               timestamp with time zone default now() not null,
    completed_at             timestamp with time zone
);

comment on column public.batch_command_tasks.status is 'Possible values: PENDING, IN_PROGRESS, COMPLETED_SUCCESSFULLY, COMPLETED_WITH_ERRORS, TERMINATED, FAILED_TO_START';

alter table public.batch_command_tasks
    owner to postgres;

create index idx_batch_command_tasks_status
    on public.batch_command_tasks (status);

create index idx_batch_command_tasks_user_id
    on public.batch_command_tasks (user_id);

create index idx_batch_command_tasks_created_at
    on public.batch_command_tasks (created_at desc);

create table public.child_command_tasks
(
    child_command_id   uuid                                   not null
        primary key,
    batch_command_id   uuid                                   not null
        references public.batch_command_tasks
            on delete cascade,
    vps_id             integer                                not null
        references public.vps
            on delete cascade,
    status             varchar(50)                            not null,
    exit_code          integer,
    error_message      text,
    stdout_log_path    varchar(1024),
    stderr_log_path    varchar(1024),
    last_output_at     timestamp with time zone,
    created_at         timestamp with time zone default now() not null,
    updated_at         timestamp with time zone default now() not null,
    agent_started_at   timestamp with time zone,
    agent_completed_at timestamp with time zone
);

comment on column public.child_command_tasks.status is 'Possible values: PENDING, SENT_TO_AGENT, AGENT_ACCEPTED, EXECUTING, SUCCESS, FAILURE, TERMINATED, AGENT_UNREACHABLE, AGENT_REJECTED';

alter table public.child_command_tasks
    owner to postgres;

create index idx_child_command_tasks_batch_command_id
    on public.child_command_tasks (batch_command_id);

create index idx_child_command_tasks_vps_id
    on public.child_command_tasks (vps_id);

create index idx_child_command_tasks_status
    on public.child_command_tasks (status);

create index idx_child_command_tasks_created_at
    on public.child_command_tasks (created_at desc);

create table public.service_monitors
(
    id                serial
        primary key,
    user_id           integer                                                         not null
        references public.users
            on delete cascade,
    name              varchar                                                         not null,
    monitor_type      varchar                                                         not null,
    target            varchar                                                         not null,
    frequency_seconds integer                  default 60                             not null,
    timeout_seconds   integer                  default 10                             not null,
    is_active         boolean                  default true                           not null,
    monitor_config    jsonb,
    created_at        timestamp with time zone default now()                          not null,
    updated_at        timestamp with time zone default now()                          not null,
    assignment_type   varchar(255)             default 'INCLUSIVE'::character varying not null
);

alter table public.service_monitors
    owner to postgres;

create table public.service_monitor_agents
(
    monitor_id integer not null
        references public.service_monitors
            on delete cascade,
    vps_id     integer not null
        references public.vps
            on delete cascade,
    primary key (monitor_id, vps_id)
);

alter table public.service_monitor_agents
    owner to postgres;

create table public.service_monitor_tags
(
    monitor_id integer not null
        references public.service_monitors
            on delete cascade,
    tag_id     integer not null
        references public.tags
            on delete cascade,
    primary key (monitor_id, tag_id)
);

alter table public.service_monitor_tags
    owner to postgres;

create table public.service_monitor_results
(
    time       timestamp with time zone not null,
    monitor_id integer                  not null
        references public.service_monitors
            on delete cascade,
    agent_id   integer                  not null
        references public.vps
            on delete cascade,
    is_up      boolean                  not null,
    latency_ms integer,
    details    jsonb
);

alter table public.service_monitor_results
    owner to postgres;

create index service_monitor_results_time_idx
    on public.service_monitor_results (time desc);

create index service_monitor_results_monitor_id_time_idx
    on public.service_monitor_results (monitor_id asc, time desc);

create index service_monitor_results_agent_id_time_idx
    on public.service_monitor_results (agent_id asc, time desc);

create trigger ts_insert_blocker
    before insert
    on public.service_monitor_results
    for each row
execute procedure ???();

create table public.command_scripts
(
    id                serial
        primary key,
    user_id           integer                                        not null
        references public.users
            on delete cascade,
    name              varchar(255)                                   not null,
    description       text,
    script_content    text                                           not null,
    working_directory varchar(255)                                   not null,
    created_at        timestamp with time zone default now()         not null,
    updated_at        timestamp with time zone default now()         not null,
    language          text                     default 'shell'::text not null,
    constraint uq_user_script_name
        unique (user_id, name)
);

alter table public.command_scripts
    owner to postgres;

create index idx_command_scripts_user_id
    on public.command_scripts (user_id);

create table public.oauth2_providers
(
    id                serial
        primary key,
    provider_name     varchar(255)                           not null
        unique,
    client_id         varchar(255)                           not null,
    client_secret     text                                   not null,
    auth_url          varchar(255)                           not null,
    token_url         varchar(255)                           not null,
    user_info_url     varchar(255)                           not null,
    scopes            text,
    user_info_mapping jsonb,
    enabled           boolean                  default true  not null,
    created_at        timestamp with time zone default now() not null,
    updated_at        timestamp with time zone default now() not null,
    icon_url          text
);

alter table public.oauth2_providers
    owner to postgres;

create index idx_oauth2_providers_provider_name
    on public.oauth2_providers (provider_name);

create table public.user_identity_providers
(
    id               serial
        primary key,
    user_id          integer                                not null
        references public.users
            on delete cascade,
    provider_name    varchar(255)                           not null,
    provider_user_id varchar(255)                           not null,
    created_at       timestamp with time zone default now() not null,
    updated_at       timestamp with time zone default now() not null,
    unique (provider_name, provider_user_id)
);

alter table public.user_identity_providers
    owner to postgres;

create index idx_user_identity_providers_user_id
    on public.user_identity_providers (user_id);

create table public.themes
(
    id          uuid                     default gen_random_uuid() not null
        primary key,
    user_id     integer                                            not null
        references public.users
            on delete cascade,
    name        varchar(255)                                       not null,
    is_official boolean                  default false             not null,
    css         text                                               not null,
    created_at  timestamp with time zone default now()             not null,
    updated_at  timestamp with time zone default now()             not null
);

alter table public.themes
    owner to postgres;

create index idx_themes_user_id
    on public.themes (user_id);

create index idx_themes_is_official
    on public.themes (is_official);

