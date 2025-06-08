use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::{FromRow, PgPool, Result};
use uuid::Uuid;

use crate::db::models::Vps;
use crate::websocket_models::{ServerBasicInfo, ServerMetricsSnapshot, ServerWithDetails};

// --- Vps Service Functions ---

/// Creates a new VPS entry.
pub async fn create_vps(pool: &PgPool, user_id: i32, name: &str) -> Result<Vps> {
    let now = Utc::now();
    let generated_agent_secret = Uuid::new_v4().to_string();
    let initial_status = "pending";
    let initial_ip_address: Option<String> = None;
    let initial_os_type: Option<String> = None;
    let initial_metadata: Option<serde_json::Value> = None;
    let initial_group: Option<String> = None;

    let vps = sqlx::query_as!(
        Vps,
        r#"
        INSERT INTO vps (
            user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at, "group",
            agent_config_override, config_status, last_config_update_at, last_config_error,
            traffic_limit_bytes, traffic_billing_rule, traffic_current_cycle_rx_bytes, traffic_current_cycle_tx_bytes,
            last_processed_cumulative_rx, last_processed_cumulative_tx, traffic_last_reset_at,
            traffic_reset_config_type, traffic_reset_config_value, next_traffic_reset_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)
        RETURNING
            id, user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at, "group",
            agent_config_override, config_status, last_config_update_at, last_config_error,
            traffic_limit_bytes, traffic_billing_rule, traffic_current_cycle_rx_bytes, traffic_current_cycle_tx_bytes,
            last_processed_cumulative_rx, last_processed_cumulative_tx, traffic_last_reset_at,
            traffic_reset_config_type, traffic_reset_config_value, next_traffic_reset_at
        "#,
        user_id,
        name,
        initial_ip_address,
        initial_os_type,
        generated_agent_secret,
        initial_status,
        initial_metadata,
        now,
        now,
        initial_group,
        None::<serde_json::Value>, // agent_config_override
        "unknown",                 // config_status
        None::<chrono::DateTime<Utc>>, // last_config_update_at
        None::<String>,             // last_config_error
        None::<i64>, None::<String>, Some(0i64), Some(0i64), Some(0i64), Some(0i64), None::<DateTime<Utc>>, None::<String>, None::<String>, None::<DateTime<Utc>>
)
.fetch_one(pool)
    .await?;
    Ok(vps)
}

/// Retrieves a VPS by its ID.
pub async fn get_vps_by_id(pool: &PgPool, vps_id: i32) -> Result<Option<Vps>> {
    sqlx::query_as!(Vps, r#"SELECT id, user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at, "group", agent_config_override, config_status, last_config_update_at, last_config_error, traffic_limit_bytes, traffic_billing_rule, traffic_current_cycle_rx_bytes, traffic_current_cycle_tx_bytes, last_processed_cumulative_rx, last_processed_cumulative_tx, traffic_last_reset_at, traffic_reset_config_type, traffic_reset_config_value, next_traffic_reset_at FROM vps WHERE id = $1"#, vps_id)
        .fetch_optional(pool)
        .await
}

/// Retrieves all VPS entries for a given user.
pub async fn get_vps_by_user_id(pool: &PgPool, user_id: i32) -> Result<Vec<Vps>> {
    sqlx::query_as!(Vps, r#"SELECT id, user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at, "group", agent_config_override, config_status, last_config_update_at, last_config_error, traffic_limit_bytes, traffic_billing_rule, traffic_current_cycle_rx_bytes, traffic_current_cycle_tx_bytes, last_processed_cumulative_rx, last_processed_cumulative_tx, traffic_last_reset_at, traffic_reset_config_type, traffic_reset_config_value, next_traffic_reset_at FROM vps WHERE user_id = $1 ORDER BY created_at DESC"#, user_id)
        .fetch_all(pool)
        .await
}

/// Retrieves all VPS entries for a given user.
/// This is an alias for get_vps_by_user_id, but could be different in the future if needed.
pub async fn get_all_vps_for_user(pool: &PgPool, user_id: i32) -> Result<Vec<Vps>> {
    get_vps_by_user_id(pool, user_id).await
}

/// Updates a VPS's editable fields.
pub async fn update_vps(
    pool: &PgPool,
    vps_id: i32,
    user_id: i32, // To ensure ownership
    name: Option<String>,
    group: Option<String>,
    tag_ids: Option<Vec<i32>>,
    // Traffic monitoring config fields
    traffic_limit_bytes: Option<i64>,
    traffic_billing_rule: Option<String>,
    traffic_reset_config_type: Option<String>,
    traffic_reset_config_value: Option<String>,
    next_traffic_reset_at: Option<DateTime<Utc>>,
) -> Result<bool> {
    // Return bool indicating if a change was made
    let mut tx = pool.begin().await?;
    let now = Utc::now();

    // 1. Update the main VPS table
    let rows_affected = sqlx::query!(
        r#"
        UPDATE vps
        SET
            name = COALESCE($1, name),
            "group" = $2,
            traffic_limit_bytes = $3,
            traffic_billing_rule = $4,
            traffic_reset_config_type = $5,
            traffic_reset_config_value = $6,
            next_traffic_reset_at = $7,
            updated_at = $8
        WHERE id = $9 AND user_id = $10
        "#,
        name,
        group,
        traffic_limit_bytes,
        traffic_billing_rule,
        traffic_reset_config_type,
        traffic_reset_config_value,
        next_traffic_reset_at,
        now,
        vps_id,
        user_id
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    // 2. If tag_ids is provided, update the associations.
    let mut tags_changed = false;
    if let Some(ids) = tag_ids {
        tags_changed = true; // The presence of the key indicates an intent to update.
                             // 2a. Delete all existing tags for this VPS
        sqlx::query!("DELETE FROM vps_tags WHERE vps_id = $1", vps_id)
            .execute(&mut *tx)
            .await?;

        // 2b. Insert the new tags if the list is not empty
        if !ids.is_empty() {
            sqlx::query!(
                r#"
                INSERT INTO vps_tags (vps_id, tag_id)
                SELECT $1, tag_id
                FROM UNNEST($2::int[]) as tag_id
                ON CONFLICT (vps_id, tag_id) DO NOTHING
                "#,
                vps_id,
                &ids
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;

    Ok(rows_affected > 0 || tags_changed)
}

/// Updates the status of a VPS.
pub async fn update_vps_status(pool: &PgPool, vps_id: i32, status: &str) -> Result<u64> {
    let now = Utc::now();
    let rows_affected = sqlx::query!(
        "UPDATE vps SET status = $1, updated_at = $2 WHERE id = $3",
        status,
        now,
        vps_id
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows_affected)
}

/// Updates VPS information based on AgentHandshake data.
/// This includes OS details, hostname, and public IP addresses.
/// Also sets status to "online".
pub async fn update_vps_info_on_handshake(
    pool: &PgPool,
    vps_id: i32,
    handshake_info: &crate::agent_service::AgentHandshake, // Pass the full handshake message
) -> Result<u64> {
    let now = Utc::now();

    // Find the first IPv4 address from the list
    let first_ipv4 = handshake_info.public_ip_addresses.iter().find_map(|ip_str| {
        ip_str
            .parse::<std::net::IpAddr>()
            .ok()
            .and_then(|ip_addr| {
                if ip_addr.is_ipv4() {
                    Some(ip_str.clone())
                } else {
                    None
                }
            })
    });

    let os_type_str = crate::agent_service::OsType::try_from(handshake_info.os_type)
        .map(|os_enum| format!("{:?}", os_enum))
        .unwrap_or_else(|_| "Unknown".to_string());

    // Construct the metadata JSON to be updated
    let mut agent_info_metadata_map = serde_json::Map::new();
    agent_info_metadata_map.insert("os_name".to_string(), json!(handshake_info.os_name));
    agent_info_metadata_map.insert("arch".to_string(), json!(handshake_info.arch));
    agent_info_metadata_map.insert("hostname".to_string(), json!(handshake_info.hostname));
    agent_info_metadata_map.insert("public_ip_addresses".to_string(), json!(handshake_info.public_ip_addresses));
    agent_info_metadata_map.insert("kernel_version".to_string(), json!(handshake_info.kernel_version));
    agent_info_metadata_map.insert("os_version_detail".to_string(), json!(handshake_info.os_version_detail));
    agent_info_metadata_map.insert("long_os_version".to_string(), json!(handshake_info.long_os_version));
    agent_info_metadata_map.insert("distribution_id".to_string(), json!(handshake_info.distribution_id));
    if let Some(p_cores) = handshake_info.physical_core_count {
        agent_info_metadata_map.insert("physical_core_count".to_string(), json!(p_cores));
    }
    if let Some(total_mem) = handshake_info.total_memory_bytes {
        agent_info_metadata_map.insert("total_memory_bytes".to_string(), json!(total_mem));
    }
    if let Some(total_swap) = handshake_info.total_swap_bytes {
        agent_info_metadata_map.insert("total_swap_bytes".to_string(), json!(total_swap));
    }

    if let Some(cpu_info) = &handshake_info.cpu_static_info {
        agent_info_metadata_map.insert("cpu_static_info".to_string(), json!({
            "name": cpu_info.name,
            "frequency": cpu_info.frequency,
            "vendor_id": cpu_info.vendor_id,
            "brand": cpu_info.brand,
        }));
    }
    if let Some(cc) = &handshake_info.country_code {
        if !cc.is_empty() {
            agent_info_metadata_map.insert("country_code".to_string(), json!(cc));
        }
    }
    
    let agent_info_metadata = serde_json::Value::Object(agent_info_metadata_map);

    let rows_affected = sqlx::query!(
        r#"
        UPDATE vps
        SET os_type = $1,                               -- Direct column
            ip_address = $2,                            -- Direct column, stores first IPv4 (VARCHAR(45) for now)
            metadata = COALESCE(metadata, '{}'::jsonb) || $3::jsonb, -- Merge new info into metadata
            status = $4,                                -- Direct column
            updated_at = $5                             -- Direct column
        WHERE id = $6
        "#,
        os_type_str,
        first_ipv4,
        agent_info_metadata,
        "online",   // Set status to online on successful handshake
        now,
        vps_id
    )
    .execute(pool)
    .await?
    .rows_affected();

    if rows_affected == 0 {
        eprintln!("VPS info update on handshake for vps_id {} affected 0 rows. This might indicate the VPS ID was not found for update.", vps_id);
    }
    Ok(rows_affected)
}

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
    vps_metadata: Option<serde_json::Value>, // Added for VPS metadata
    // New config fields
    vps_config_status: String,
    vps_last_config_update_at: Option<chrono::DateTime<Utc>>,
    vps_last_config_error: Option<String>,
    // Traffic Monitoring Fields from vps table
    vps_traffic_limit_bytes: Option<i64>,
    vps_traffic_billing_rule: Option<String>,
    vps_traffic_current_cycle_rx_bytes: Option<i64>, // In DB: NOT NULL DEFAULT 0
    vps_traffic_current_cycle_tx_bytes: Option<i64>, // In DB: NOT NULL DEFAULT 0
    vps_last_processed_cumulative_rx: Option<i64>, // In DB: NOT NULL DEFAULT 0
    vps_last_processed_cumulative_tx: Option<i64>, // In DB: NOT NULL DEFAULT 0
    vps_traffic_last_reset_at: Option<chrono::DateTime<Utc>>,
    vps_traffic_reset_config_type: Option<String>,
    vps_traffic_reset_config_value: Option<String>,
    vps_next_traffic_reset_at: Option<chrono::DateTime<Utc>>,
    // Metrics fields (all optional because of LEFT JOIN)
    cpu_usage_percent: Option<f64>,
    memory_usage_bytes: Option<i64>,
    memory_total_bytes: Option<i64>,
    network_rx_instant_bps: Option<i64>,
    network_tx_instant_bps: Option<i64>,
    uptime_seconds: Option<i64>,
    total_disk_used_bytes: Option<i64>,
    total_disk_total_bytes: Option<i64>,
    metric_time: Option<chrono::DateTime<Utc>>, // Added to store the time of the metric
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
            v.metadata as vps_metadata, -- Added
            v.config_status as vps_config_status,
            v.last_config_update_at as vps_last_config_update_at,
            v.last_config_error as vps_last_config_error,
            -- Select new traffic fields
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
            lvm.time as metric_time
        FROM vps v
        LEFT JOIN LatestVpsMetrics lvm ON v.id = lvm.vps_id
        LEFT JOIN LatestVpsDiskUsage lvdu ON v.id = lvdu.vps_id
        LEFT JOIN VpsTagsAggregated vta ON v.id = vta.vps_id
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
            // Map new traffic fields
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
            metadata: row.vps_metadata, // Added
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
            WHERE vps_id IN (SELECT id FROM vps WHERE user_id = $1) -- Pre-filter metrics for performance
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
            WHERE vt.vps_id IN (SELECT id FROM vps WHERE user_id = $1) -- Pre-filter tags for performance
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
            v.metadata as vps_metadata, -- Added
            v.config_status as vps_config_status,
            v.last_config_update_at as vps_last_config_update_at,
            v.last_config_error as vps_last_config_error,
            -- Select new traffic fields
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
            lvm.time as metric_time
        FROM vps v
        LEFT JOIN LatestVpsMetrics lvm ON v.id = lvm.vps_id
        LEFT JOIN LatestVpsDiskUsage lvdu ON v.id = lvdu.vps_id
        LEFT JOIN VpsTagsAggregated vta ON v.id = vta.vps_id
        WHERE v.user_id = $1 -- Filter for the specific user
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
            // Map new traffic fields
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
            metadata: row.vps_metadata, // Added
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
            v.metadata as vps_metadata, -- Added
            v.config_status as vps_config_status,
            v.last_config_update_at as vps_last_config_update_at,
            v.last_config_error as vps_last_config_error,
            -- Select new traffic fields
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
            lvm.time as metric_time
        FROM vps v
        LEFT JOIN LatestVpsMetrics lvm ON v.id = lvm.vps_id
        LEFT JOIN LatestVpsDiskUsage lvdu ON v.id = lvdu.vps_id
        LEFT JOIN VpsTagsAggregated vta ON v.id = vta.vps_id
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
            // Map new traffic fields
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
            metadata: row.vps_metadata, // Added
        }))
    } else {
        Ok(None)
    }
}
/// Updates VPS traffic statistics after a new performance metric is recorded.
/// This function should be called within the same transaction as saving the performance metric.
pub async fn update_vps_traffic_stats_after_metric(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, // Changed to accept a transaction
    vps_id: i32,
    new_cumulative_rx: i64,
    new_cumulative_tx: i64,
) -> Result<()> {
    // 1. Get the current Vps traffic stats
    let vps = sqlx::query_as!(
        Vps,
        r#"
        SELECT 
            id, user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at, "group",
            agent_config_override, config_status, last_config_update_at, last_config_error,
            traffic_limit_bytes, traffic_billing_rule, traffic_current_cycle_rx_bytes, traffic_current_cycle_tx_bytes,
            last_processed_cumulative_rx, last_processed_cumulative_tx, traffic_last_reset_at,
            traffic_reset_config_type, traffic_reset_config_value, next_traffic_reset_at
        FROM vps WHERE id = $1 FOR UPDATE
        "#, // Added FOR UPDATE to lock the row
        vps_id
    )
    .fetch_one(&mut **tx) // Use the dereferenced transaction
    .await?;

    let last_rx = vps.last_processed_cumulative_rx.unwrap_or(0);
    let last_tx = vps.last_processed_cumulative_tx.unwrap_or(0);
    let mut current_cycle_rx = vps.traffic_current_cycle_rx_bytes.unwrap_or(0);
    let mut current_cycle_tx = vps.traffic_current_cycle_tx_bytes.unwrap_or(0);

    // 2. Calculate delta, handling counter resets
    let delta_rx = if new_cumulative_rx >= last_rx {
        new_cumulative_rx - last_rx
    } else {
        // Counter reset detected (or very first data point if last_rx was 0 and new_cumulative_rx is also 0)
        // If last_rx was non-zero, this means a reset. We take the new_cumulative_rx as the delta since last point.
        // If last_rx was 0, and new_cumulative_rx is also 0, delta is 0.
        // If last_rx was 0, and new_cumulative_rx is >0, this case is not hit.
        new_cumulative_rx 
    };

    let delta_tx = if new_cumulative_tx >= last_tx {
        new_cumulative_tx - last_tx
    } else {
        new_cumulative_tx
    };

    // 3. Update cycle usage
    current_cycle_rx += delta_rx;
    current_cycle_tx += delta_tx;

    // 4. Update Vps table
    sqlx::query!(
        r#"
        UPDATE vps
        SET
            traffic_current_cycle_rx_bytes = $1,
            traffic_current_cycle_tx_bytes = $2,
            last_processed_cumulative_rx = $3,
            last_processed_cumulative_tx = $4,
            updated_at = $5
        WHERE id = $6
        "#,
        current_cycle_rx,
        current_cycle_tx,
        new_cumulative_rx,
        new_cumulative_tx,
        Utc::now(),
        vps_id
    )
    .execute(&mut **tx) // Use the dereferenced transaction
    .await?;

    Ok(())
}
use chrono::{NaiveDate, Datelike, Duration}; // Added for date calculations

// ... (other existing code) ...

/// Processes traffic reset for a single VPS if due.
/// Resets current cycle usage, updates last reset time, and calculates the next reset time.
/// Returns Ok(true) if a reset was performed, Ok(false) otherwise.
pub async fn process_vps_traffic_reset(pool: &PgPool, vps_id: i32) -> Result<bool> {
    let mut tx = pool.begin().await?;
    let now = Utc::now();

    // Fetch the VPS with FOR UPDATE to lock the row during the transaction
    let vps = sqlx::query_as!(
        Vps,
        r#"
        SELECT 
            id, user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at, "group",
            agent_config_override, config_status, last_config_update_at, last_config_error,
            traffic_limit_bytes, traffic_billing_rule, traffic_current_cycle_rx_bytes, traffic_current_cycle_tx_bytes,
            last_processed_cumulative_rx, last_processed_cumulative_tx, traffic_last_reset_at,
            traffic_reset_config_type, traffic_reset_config_value, next_traffic_reset_at
        FROM vps WHERE id = $1 FOR UPDATE
        "#,
        vps_id
    )
    .fetch_optional(&mut *tx) // Use the dereferenced transaction
    .await?;

    if vps.is_none() {
        tx.commit().await?; // VPS not found, commit to release lock if any was taken by begin
        return Ok(false); 
    }
    let mut vps_data = vps.unwrap();

    // Check if reset is due
    if vps_data.next_traffic_reset_at.is_none() || vps_data.next_traffic_reset_at.unwrap() > now {
        tx.commit().await?; // Commit transaction as no reset is needed
        return Ok(false); // Not due for reset
    }

    // --- Perform Reset ---
    let last_reset_time = vps_data.next_traffic_reset_at.unwrap(); // This was the scheduled reset time

    // Calculate new next_traffic_reset_at
    let new_next_reset_at: Option<DateTime<Utc>>;
    match (vps_data.traffic_reset_config_type.as_deref(), vps_data.traffic_reset_config_value.as_deref()) {
        (Some("monthly_day_of_month"), Some(value_str)) => {
            // Example value_str: "day:15,time_offset_seconds:28800"
            let mut day_of_month: Option<u32> = None;
            let mut time_offset_seconds: i64 = 0;

            for part in value_str.split(',') {
                let kv: Vec<&str> = part.split(':').collect();
                if kv.len() == 2 {
                    match kv[0] {
                        "day" => day_of_month = kv[1].parse().ok(),
                        "time_offset_seconds" => time_offset_seconds = kv[1].parse().unwrap_or(0),
                        _ => {}
                    }
                }
            }

            if let Some(day) = day_of_month {
                let current_reset_naive_date = last_reset_time.date_naive();
                let mut next_month_year = current_reset_naive_date.year();
                let mut next_month_month = current_reset_naive_date.month() + 1;
                
                if next_month_month > 12 {
                    next_month_month = 1;
                    next_month_year += 1;
                }
                
                let first_day_of_next_month = NaiveDate::from_ymd_opt(next_month_year, next_month_month, 1).unwrap();
                let days_in_next_month = if next_month_month == 12 {
                    NaiveDate::from_ymd_opt(next_month_year + 1, 1, 1).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(next_month_year, next_month_month + 1, 1).unwrap()
                }.signed_duration_since(first_day_of_next_month).num_days() as u32;
                
                let actual_day = std::cmp::min(day, days_in_next_month);

                if let Some(naive_date_next) = NaiveDate::from_ymd_opt(next_month_year, next_month_month, actual_day) {
                     let naive_datetime_next = naive_date_next.and_hms_opt(0,0,0).unwrap_or(naive_date_next.and_hms_opt(0,0,0).expect("Should be valid time")) + Duration::seconds(time_offset_seconds);
                    new_next_reset_at = Some(DateTime::<Utc>::from_naive_utc_and_offset(naive_datetime_next, Utc));
                } else {
                    new_next_reset_at = None; 
                    eprintln!("Error calculating next reset date for monthly_day_of_month for VPS ID {}: Could not form NaiveDate from y/m/d: {}/{}/{}", vps_id, next_month_year, next_month_month, actual_day);
                }
            } else {
                new_next_reset_at = None; // Invalid config
                eprintln!("Invalid traffic_reset_config_value (missing day) for monthly_day_of_month for VPS ID {}", vps_id);
            }
        }
        (Some("fixed_days"), Some(value_str)) => {
            if let Ok(days) = value_str.parse::<i64>() {
                if days > 0 {
                    new_next_reset_at = Some(last_reset_time + Duration::days(days));
                } else {
                    new_next_reset_at = None;
                    eprintln!("Invalid traffic_reset_config_value (days <= 0) for fixed_days for VPS ID {}", vps_id);
                }
            } else {
                new_next_reset_at = None; // Invalid config
                eprintln!("Invalid traffic_reset_config_value (not a number) for fixed_days for VPS ID {}", vps_id);
            }
        }
        _ => {
            new_next_reset_at = None; // Unknown or missing config type/value
            eprintln!("Missing or unknown traffic_reset_config_type or _value for VPS ID {}. Cannot calculate next reset.", vps_id);
        }
    }

    // Update VPS record
    sqlx::query!(
        r#"
        UPDATE vps
        SET
            traffic_current_cycle_rx_bytes = 0,
            traffic_current_cycle_tx_bytes = 0,
            traffic_last_reset_at = $1,
            next_traffic_reset_at = $2,
            updated_at = $3
        WHERE id = $4
        "#,
        last_reset_time, // This is the time when the reset actually occurred
        new_next_reset_at,
        Utc::now(),
        vps_id
    )
    .execute(&mut *tx) // Use the dereferenced transaction
    .await?;

    tx.commit().await?;
    Ok(true)
}
/// Retrieves IDs of VPS that are due for a traffic reset check.
pub async fn get_vps_due_for_traffic_reset(pool: &PgPool) -> Result<Vec<i32>> {
    let now = Utc::now();
    let vps_ids = sqlx::query!(
        r#"
        SELECT id
        FROM vps
        WHERE traffic_reset_config_type IS NOT NULL 
          AND traffic_reset_config_value IS NOT NULL
          AND next_traffic_reset_at IS NOT NULL
          AND next_traffic_reset_at <= $1
        "#,
        now
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| row.id)
    .collect();
    Ok(vps_ids)
}