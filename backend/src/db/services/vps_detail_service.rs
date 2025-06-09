use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Result};

use crate::websocket_models::{ServerBasicInfo, ServerMetricsSnapshot, ServerWithDetails};

// --- Vps Detail Service Functions ---

// Helper struct for the get_all_vps_with_details_for_cache query result
#[derive(FromRow, Debug)]
struct VpsDetailQueryResult {
    vps_id: i32,
    vps_user_id: i32,
    vps_name: String,
    vps_ip_address: Option<String>,
    vps_status: String,
    vps_os_type: Option<String>,
    vps_group: Option<String>,
    vps_tags_json: Option<serde_json::Value>,
    vps_created_at: chrono::DateTime<Utc>,
    vps_metadata: Option<serde_json::Value>,
    vps_config_status: String,
    vps_last_config_update_at: Option<chrono::DateTime<Utc>>,
    vps_last_config_error: Option<String>,
    vps_traffic_limit_bytes: Option<i64>,
    vps_traffic_billing_rule: Option<String>,
    vps_traffic_current_cycle_rx_bytes: Option<i64>,
    vps_traffic_current_cycle_tx_bytes: Option<i64>,
    vps_last_processed_cumulative_rx: Option<i64>,
    vps_last_processed_cumulative_tx: Option<i64>,
    vps_traffic_last_reset_at: Option<chrono::DateTime<Utc>>,
    vps_traffic_reset_config_type: Option<String>,
    vps_traffic_reset_config_value: Option<String>,
    vps_next_traffic_reset_at: Option<chrono::DateTime<Utc>>,
    cpu_usage_percent: Option<f64>,
    memory_usage_bytes: Option<i64>,
    memory_total_bytes: Option<i64>,
    network_rx_instant_bps: Option<i64>,
    network_tx_instant_bps: Option<i64>,
    uptime_seconds: Option<i64>,
    total_disk_used_bytes: Option<i64>,
    total_disk_total_bytes: Option<i64>,
    metric_time: Option<chrono::DateTime<Utc>>,
    renewal_cycle: Option<String>,
    renewal_cycle_custom_days: Option<i32>,
    renewal_price: Option<f64>,
    renewal_currency: Option<String>,
    next_renewal_date: Option<DateTime<Utc>>,
    last_renewal_date: Option<DateTime<Utc>>,
    service_start_date: Option<DateTime<Utc>>,
    payment_method: Option<String>,
    auto_renew_enabled: Option<bool>,
    renewal_notes: Option<String>,
    reminder_active: Option<bool>,
}

/// Retrieves all VPS along with their latest metrics and disk usage for cache initialization.
pub async fn get_all_vps_with_details_for_cache(
    pool: &PgPool,
) -> Result<Vec<ServerWithDetails>> {
    let query_results = sqlx::query_as::<_, VpsDetailQueryResult>(
        r#"
        WITH RankedMetrics AS (
            SELECT *, ROW_NUMBER() OVER (PARTITION BY vps_id ORDER BY time DESC) as rn
            FROM performance_metrics
        ),
        LatestVpsMetrics AS (
            SELECT * FROM RankedMetrics WHERE rn = 1
        ),
        LatestVpsDiskUsage AS (
            SELECT
                lvm.vps_id,
                SUM(pdu.used_bytes)::BIGINT as total_disk_used_bytes,
                SUM(pdu.total_bytes)::BIGINT as total_disk_total_bytes
            FROM performance_disk_usages pdu
            JOIN LatestVpsMetrics lvm ON pdu.performance_metric_id = lvm.id
            GROUP BY lvm.vps_id
        ),
        VpsTagsAggregated AS (
            SELECT
                vt.vps_id,
                json_agg(json_build_object(
                    'id', t.id,
                    'name', t.name,
                    'color', t.color,
                    'icon', t.icon,
                    'url', t.url,
                    'isVisible', t.is_visible
                )) as tags_json
            FROM vps_tags vt
            JOIN tags t ON vt.tag_id = t.id
            GROUP BY vt.vps_id
        )
        SELECT
            v.id as vps_id,
            v.user_id as vps_user_id,
            v.name as vps_name,
            v.ip_address as vps_ip_address,
            v.status as vps_status,
            v.os_type as vps_os_type,
            v."group" as vps_group,
            vta.tags_json as vps_tags_json,
            v.created_at as vps_created_at,
            v.metadata as vps_metadata,
            v.config_status as vps_config_status,
            v.last_config_update_at as vps_last_config_update_at,
            v.last_config_error as vps_last_config_error,
            v.traffic_limit_bytes as vps_traffic_limit_bytes,
            v.traffic_billing_rule as vps_traffic_billing_rule,
            v.traffic_current_cycle_rx_bytes as vps_traffic_current_cycle_rx_bytes,
            v.traffic_current_cycle_tx_bytes as vps_traffic_current_cycle_tx_bytes,
            v.last_processed_cumulative_rx as vps_last_processed_cumulative_rx,
            v.last_processed_cumulative_tx as vps_last_processed_cumulative_tx,
            v.traffic_last_reset_at as vps_traffic_last_reset_at,
            v.traffic_reset_config_type as vps_traffic_reset_config_type,
            v.traffic_reset_config_value as vps_traffic_reset_config_value,
            v.next_traffic_reset_at as vps_next_traffic_reset_at,
            lvm.cpu_usage_percent,
            lvm.memory_usage_bytes,
            lvm.memory_total_bytes,
            lvm.network_rx_instant_bps,
            lvm.network_tx_instant_bps,
            lvm.uptime_seconds,
            lvdu.total_disk_used_bytes,
            lvdu.total_disk_total_bytes,
            lvm.time as metric_time,
            vri.renewal_cycle,
            vri.renewal_cycle_custom_days,
            vri.renewal_price,
            vri.renewal_currency,
            vri.next_renewal_date,
            vri.last_renewal_date,
            vri.service_start_date,
            vri.payment_method,
            vri.auto_renew_enabled,
            vri.renewal_notes,
            vri.reminder_active
        FROM vps v
        LEFT JOIN LatestVpsMetrics lvm ON v.id = lvm.vps_id
        LEFT JOIN LatestVpsDiskUsage lvdu ON v.id = lvdu.vps_id
        LEFT JOIN VpsTagsAggregated vta ON v.id = vta.vps_id
        LEFT JOIN vps_renewal_info vri ON v.id = vri.vps_id
        ORDER BY v.id;
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut servers_with_details = Vec::new();
    for row in query_results {
        let tags: Option<Vec<crate::websocket_models::Tag>> =
            row.vps_tags_json
                .and_then(|json_value| serde_json::from_value(json_value).ok());

        let basic_info = ServerBasicInfo {
            id: row.vps_id,
            user_id: row.vps_user_id,
            name: row.vps_name,
            ip_address: row.vps_ip_address,
            status: row.vps_status,
            group: row.vps_group,
            tags,
            config_status: row.vps_config_status,
            last_config_update_at: row.vps_last_config_update_at,
            last_config_error: row.vps_last_config_error,
            traffic_limit_bytes: row.vps_traffic_limit_bytes,
            traffic_billing_rule: row.vps_traffic_billing_rule,
            traffic_current_cycle_rx_bytes: row.vps_traffic_current_cycle_rx_bytes,
            traffic_current_cycle_tx_bytes: row.vps_traffic_current_cycle_tx_bytes,
            traffic_last_reset_at: row.vps_traffic_last_reset_at,
            traffic_reset_config_type: row.vps_traffic_reset_config_type,
            traffic_reset_config_value: row.vps_traffic_reset_config_value,
            next_traffic_reset_at: row.vps_next_traffic_reset_at,
        };

        let latest_metrics = if row.cpu_usage_percent.is_some() && row.metric_time.is_some() {
            Some(ServerMetricsSnapshot {
                time: row.metric_time.unwrap(),
                cpu_usage_percent: row.cpu_usage_percent.unwrap_or(0.0) as f32,
                memory_usage_bytes: row.memory_usage_bytes.unwrap_or(0) as u64,
                memory_total_bytes: row.memory_total_bytes.unwrap_or(0) as u64,
                network_rx_instant_bps: row.network_rx_instant_bps.map(|val| val as u64),
                network_tx_instant_bps: row.network_tx_instant_bps.map(|val| val as u64),
                uptime_seconds: row.uptime_seconds.map(|val| val as u64),
                disk_used_bytes: row.total_disk_used_bytes.map(|val| val as u64),
                disk_total_bytes: row.total_disk_total_bytes.map(|val| val as u64),
            })
        } else {
            None
        };

        servers_with_details.push(ServerWithDetails {
            basic_info,
            latest_metrics,
            os_type: row.vps_os_type,
            created_at: row.vps_created_at,
            metadata: row.vps_metadata,
            renewal_cycle: row.renewal_cycle,
            renewal_cycle_custom_days: row.renewal_cycle_custom_days,
            renewal_price: row.renewal_price,
            renewal_currency: row.renewal_currency,
            next_renewal_date: row.next_renewal_date,
            last_renewal_date: row.last_renewal_date,
            service_start_date: row.service_start_date,
            payment_method: row.payment_method,
            auto_renew_enabled: row.auto_renew_enabled,
            renewal_notes: row.renewal_notes,
            reminder_active: row.reminder_active,
        });
    }

    Ok(servers_with_details)
}

/// Retrieves all VPS for a specific user along with their latest metrics and disk usage.
pub async fn get_all_vps_with_details_for_user(
    pool: &PgPool,
    user_id: i32,
) -> Result<Vec<ServerWithDetails>> {
    let query_results = sqlx::query_as::<_, VpsDetailQueryResult>(
        r#"
        WITH RankedMetrics AS (
            SELECT *, ROW_NUMBER() OVER (PARTITION BY vps_id ORDER BY time DESC) as rn
            FROM performance_metrics
            WHERE vps_id IN (SELECT id FROM vps WHERE user_id = $1)
        ),
        LatestVpsMetrics AS (
            SELECT * FROM RankedMetrics WHERE rn = 1
        ),
        LatestVpsDiskUsage AS (
            SELECT
                lvm.vps_id,
                SUM(pdu.used_bytes)::BIGINT as total_disk_used_bytes,
                SUM(pdu.total_bytes)::BIGINT as total_disk_total_bytes
            FROM performance_disk_usages pdu
            JOIN LatestVpsMetrics lvm ON pdu.performance_metric_id = lvm.id
            GROUP BY lvm.vps_id
        ),
        VpsTagsAggregated AS (
            SELECT
                vt.vps_id,
                json_agg(json_build_object(
                    'id', t.id,
                    'name', t.name,
                    'color', t.color,
                    'icon', t.icon,
                    'url', t.url,
                    'isVisible', t.is_visible
                )) as tags_json
            FROM vps_tags vt
            JOIN tags t ON vt.tag_id = t.id
            WHERE vt.vps_id IN (SELECT id FROM vps WHERE user_id = $1)
            GROUP BY vt.vps_id
        )
        SELECT
            v.id as vps_id,
            v.user_id as vps_user_id,
            v.name as vps_name,
            v.ip_address as vps_ip_address,
            v.status as vps_status,
            v.os_type as vps_os_type,
            v."group" as vps_group,
            vta.tags_json as vps_tags_json,
            v.created_at as vps_created_at,
            v.metadata as vps_metadata,
            v.config_status as vps_config_status,
            v.last_config_update_at as vps_last_config_update_at,
            v.last_config_error as vps_last_config_error,
            v.traffic_limit_bytes as vps_traffic_limit_bytes,
            v.traffic_billing_rule as vps_traffic_billing_rule,
            v.traffic_current_cycle_rx_bytes as vps_traffic_current_cycle_rx_bytes,
            v.traffic_current_cycle_tx_bytes as vps_traffic_current_cycle_tx_bytes,
            v.last_processed_cumulative_rx as vps_last_processed_cumulative_rx,
            v.last_processed_cumulative_tx as vps_last_processed_cumulative_tx,
            v.traffic_last_reset_at as vps_traffic_last_reset_at,
            v.traffic_reset_config_type as vps_traffic_reset_config_type,
            v.traffic_reset_config_value as vps_traffic_reset_config_value,
            v.next_traffic_reset_at as vps_next_traffic_reset_at,
            lvm.cpu_usage_percent,
            lvm.memory_usage_bytes,
            lvm.memory_total_bytes,
            lvm.network_rx_instant_bps,
            lvm.network_tx_instant_bps,
            lvm.uptime_seconds,
            lvdu.total_disk_used_bytes,
            lvdu.total_disk_total_bytes,
            lvm.time as metric_time,
            vri.renewal_cycle,
            vri.renewal_cycle_custom_days,
            vri.renewal_price,
            vri.renewal_currency,
            vri.next_renewal_date,
            vri.last_renewal_date,
            vri.service_start_date,
            vri.payment_method,
            vri.auto_renew_enabled,
            vri.renewal_notes,
            vri.reminder_active
        FROM vps v
        LEFT JOIN LatestVpsMetrics lvm ON v.id = lvm.vps_id
        LEFT JOIN LatestVpsDiskUsage lvdu ON v.id = lvdu.vps_id
        LEFT JOIN VpsTagsAggregated vta ON v.id = vta.vps_id
        LEFT JOIN vps_renewal_info vri ON v.id = vri.vps_id
        WHERE v.user_id = $1
        ORDER BY v.id;
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    let mut servers_with_details = Vec::new();
    for row in query_results {
        let tags: Option<Vec<crate::websocket_models::Tag>> =
            row.vps_tags_json
                .and_then(|json_value| serde_json::from_value(json_value).ok());

        let basic_info = ServerBasicInfo {
            id: row.vps_id,
            user_id: row.vps_user_id,
            name: row.vps_name,
            ip_address: row.vps_ip_address,
            status: row.vps_status,
            group: row.vps_group,
            tags,
            config_status: row.vps_config_status,
            last_config_update_at: row.vps_last_config_update_at,
            last_config_error: row.vps_last_config_error,
            traffic_limit_bytes: row.vps_traffic_limit_bytes,
            traffic_billing_rule: row.vps_traffic_billing_rule,
            traffic_current_cycle_rx_bytes: row.vps_traffic_current_cycle_rx_bytes,
            traffic_current_cycle_tx_bytes: row.vps_traffic_current_cycle_tx_bytes,
            traffic_last_reset_at: row.vps_traffic_last_reset_at,
            traffic_reset_config_type: row.vps_traffic_reset_config_type,
            traffic_reset_config_value: row.vps_traffic_reset_config_value,
            next_traffic_reset_at: row.vps_next_traffic_reset_at,
        };

        let latest_metrics = if row.cpu_usage_percent.is_some() && row.metric_time.is_some() {
            Some(ServerMetricsSnapshot {
                time: row.metric_time.unwrap(),
                cpu_usage_percent: row.cpu_usage_percent.unwrap_or(0.0) as f32,
                memory_usage_bytes: row.memory_usage_bytes.unwrap_or(0) as u64,
                memory_total_bytes: row.memory_total_bytes.unwrap_or(0) as u64,
                network_rx_instant_bps: row.network_rx_instant_bps.map(|val| val as u64),
                network_tx_instant_bps: row.network_tx_instant_bps.map(|val| val as u64),
                uptime_seconds: row.uptime_seconds.map(|val| val as u64),
                disk_used_bytes: row.total_disk_used_bytes.map(|val| val as u64),
                disk_total_bytes: row.total_disk_total_bytes.map(|val| val as u64),
            })
        } else {
            None
        };

        servers_with_details.push(ServerWithDetails {
            basic_info,
            latest_metrics,
            os_type: row.vps_os_type,
            created_at: row.vps_created_at,
            metadata: row.vps_metadata,
            renewal_cycle: row.renewal_cycle,
            renewal_cycle_custom_days: row.renewal_cycle_custom_days,
            renewal_price: row.renewal_price,
            renewal_currency: row.renewal_currency,
            next_renewal_date: row.next_renewal_date,
            last_renewal_date: row.last_renewal_date,
            service_start_date: row.service_start_date,
            payment_method: row.payment_method,
            auto_renew_enabled: row.auto_renew_enabled,
            renewal_notes: row.renewal_notes,
            reminder_active: row.reminder_active,
        });
    }

    Ok(servers_with_details)
}

/// Retrieves a single VPS with its full details for cache updates.
pub async fn get_vps_with_details_for_cache_by_id(
    pool: &PgPool,
    vps_id: i32,
) -> Result<Option<ServerWithDetails>> {
    let query_result = sqlx::query_as::<_, VpsDetailQueryResult>(
        r#"
        WITH RankedMetrics AS (
            SELECT *, ROW_NUMBER() OVER (PARTITION BY vps_id ORDER BY time DESC) as rn
            FROM performance_metrics
        ),
        LatestVpsMetrics AS (
            SELECT * FROM RankedMetrics WHERE rn = 1
        ),
        LatestVpsDiskUsage AS (
            SELECT
                lvm.vps_id,
                SUM(pdu.used_bytes)::BIGINT as total_disk_used_bytes,
                SUM(pdu.total_bytes)::BIGINT as total_disk_total_bytes
            FROM performance_disk_usages pdu
            JOIN LatestVpsMetrics lvm ON pdu.performance_metric_id = lvm.id
            GROUP BY lvm.vps_id
        ),
        VpsTagsAggregated AS (
            SELECT
                vt.vps_id,
                json_agg(json_build_object(
                    'id', t.id,
                    'name', t.name,
                    'color', t.color,
                    'icon', t.icon,
                    'url', t.url,
                    'isVisible', t.is_visible
                )) as tags_json
            FROM vps_tags vt
            JOIN tags t ON vt.tag_id = t.id
            GROUP BY vt.vps_id
        )
        SELECT
            v.id as vps_id,
            v.user_id as vps_user_id,
            v.name as vps_name,
            v.ip_address as vps_ip_address,
            v.status as vps_status,
            v.os_type as vps_os_type,
            v."group" as vps_group,
            vta.tags_json as vps_tags_json,
            v.created_at as vps_created_at,
            v.metadata as vps_metadata,
            v.config_status as vps_config_status,
            v.last_config_update_at as vps_last_config_update_at,
            v.last_config_error as vps_last_config_error,
            v.traffic_limit_bytes as vps_traffic_limit_bytes,
            v.traffic_billing_rule as vps_traffic_billing_rule,
            v.traffic_current_cycle_rx_bytes as vps_traffic_current_cycle_rx_bytes,
            v.traffic_current_cycle_tx_bytes as vps_traffic_current_cycle_tx_bytes,
            v.last_processed_cumulative_rx as vps_last_processed_cumulative_rx,
            v.last_processed_cumulative_tx as vps_last_processed_cumulative_tx,
            v.traffic_last_reset_at as vps_traffic_last_reset_at,
            v.traffic_reset_config_type as vps_traffic_reset_config_type,
            v.traffic_reset_config_value as vps_traffic_reset_config_value,
            v.next_traffic_reset_at as vps_next_traffic_reset_at,
            lvm.cpu_usage_percent,
            lvm.memory_usage_bytes,
            lvm.memory_total_bytes,
            lvm.network_rx_instant_bps,
            lvm.network_tx_instant_bps,
            lvm.uptime_seconds,
            lvdu.total_disk_used_bytes,
            lvdu.total_disk_total_bytes,
            lvm.time as metric_time,
            vri.renewal_cycle,
            vri.renewal_cycle_custom_days,
            vri.renewal_price,
            vri.renewal_currency,
            vri.next_renewal_date,
            vri.last_renewal_date,
            vri.service_start_date,
            vri.payment_method,
            vri.auto_renew_enabled,
            vri.renewal_notes,
            vri.reminder_active
        FROM vps v
        LEFT JOIN LatestVpsMetrics lvm ON v.id = lvm.vps_id
        LEFT JOIN LatestVpsDiskUsage lvdu ON v.id = lvdu.vps_id
        LEFT JOIN VpsTagsAggregated vta ON v.id = vta.vps_id
        LEFT JOIN vps_renewal_info vri ON v.id = vri.vps_id
        WHERE v.id = $1
        "#,
    )
    .bind(vps_id)
    .fetch_optional(pool)
    .await?;

    if let Some(row) = query_result {
        let tags: Option<Vec<crate::websocket_models::Tag>> =
            row.vps_tags_json
                .and_then(|json_value| serde_json::from_value(json_value).ok());

        let basic_info = ServerBasicInfo {
            id: row.vps_id,
            user_id: row.vps_user_id,
            name: row.vps_name,
            ip_address: row.vps_ip_address,
            status: row.vps_status,
            group: row.vps_group,
            tags,
            config_status: row.vps_config_status,
            last_config_update_at: row.vps_last_config_update_at,
            last_config_error: row.vps_last_config_error,
            traffic_limit_bytes: row.vps_traffic_limit_bytes,
            traffic_billing_rule: row.vps_traffic_billing_rule,
            traffic_current_cycle_rx_bytes: row.vps_traffic_current_cycle_rx_bytes,
            traffic_current_cycle_tx_bytes: row.vps_traffic_current_cycle_tx_bytes,
            traffic_last_reset_at: row.vps_traffic_last_reset_at,
            traffic_reset_config_type: row.vps_traffic_reset_config_type,
            traffic_reset_config_value: row.vps_traffic_reset_config_value,
            next_traffic_reset_at: row.vps_next_traffic_reset_at,
        };

        let latest_metrics = if row.cpu_usage_percent.is_some() && row.metric_time.is_some() {
            Some(ServerMetricsSnapshot {
                time: row.metric_time.unwrap(),
                cpu_usage_percent: row.cpu_usage_percent.unwrap_or(0.0) as f32,
                memory_usage_bytes: row.memory_usage_bytes.unwrap_or(0) as u64,
                memory_total_bytes: row.memory_total_bytes.unwrap_or(0) as u64,
                network_rx_instant_bps: row.network_rx_instant_bps.map(|val| val as u64),
                network_tx_instant_bps: row.network_tx_instant_bps.map(|val| val as u64),
                uptime_seconds: row.uptime_seconds.map(|val| val as u64),
                disk_used_bytes: row.total_disk_used_bytes.map(|val| val as u64),
                disk_total_bytes: row.total_disk_total_bytes.map(|val| val as u64),
            })
        } else {
            None
        };

        Ok(Some(ServerWithDetails {
            basic_info,
            latest_metrics,
            os_type: row.vps_os_type,
            created_at: row.vps_created_at,
            metadata: row.vps_metadata,
            renewal_cycle: row.renewal_cycle,
            renewal_cycle_custom_days: row.renewal_cycle_custom_days,
            renewal_price: row.renewal_price,
            renewal_currency: row.renewal_currency,
            next_renewal_date: row.next_renewal_date,
            last_renewal_date: row.last_renewal_date,
            service_start_date: row.service_start_date,
            payment_method: row.payment_method,
            auto_renew_enabled: row.auto_renew_enabled,
            renewal_notes: row.renewal_notes,
            reminder_active: row.reminder_active,
        }))
    } else {
        Ok(None)
    }
}