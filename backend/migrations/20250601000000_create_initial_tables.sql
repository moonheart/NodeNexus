create table public.users
(
    id            serial
        primary key,
    username      varchar(255)                           not null
        unique,
    password_hash varchar(255)                           not null,
    email         varchar(255)                           not null
        unique,
    created_at    timestamp with time zone default now() not null,
    updated_at    timestamp with time zone default now() not null
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
    next_traffic_reset_at          timestamp with time zone
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
    network_rx_bps                   bigint                   not null,
    network_tx_bps                   bigint                   not null,
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

comment on column public.performance_metrics.network_rx_bps is 'Cumulative network received bytes for the default interface since agent start or counter reset. Stored here for historical reasons/potential future use.';

comment on column public.performance_metrics.network_tx_bps is 'Cumulative network transmitted bytes for the default interface since agent start or counter reset. Stored here for historical reasons/potential future use.';

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

