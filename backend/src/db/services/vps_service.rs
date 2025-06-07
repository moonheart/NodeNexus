use chrono::Utc;
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
            agent_config_override, config_status, last_config_update_at, last_config_error
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        RETURNING
            id, user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at, "group",
            agent_config_override, config_status, last_config_update_at, last_config_error
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
        None::<String>             // last_config_error
    )
    .fetch_one(pool)
    .await?;
    Ok(vps)
}

/// Retrieves a VPS by its ID.
pub async fn get_vps_by_id(pool: &PgPool, vps_id: i32) -> Result<Option<Vps>> {
    sqlx::query_as!(Vps, r#"SELECT id, user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at, "group", agent_config_override, config_status, last_config_update_at, last_config_error FROM vps WHERE id = $1"#, vps_id)
        .fetch_optional(pool)
        .await
}

/// Retrieves all VPS entries for a given user.
pub async fn get_vps_by_user_id(pool: &PgPool, user_id: i32) -> Result<Vec<Vps>> {
    sqlx::query_as!(Vps, r#"SELECT id, user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at, "group", agent_config_override, config_status, last_config_update_at, last_config_error FROM vps WHERE user_id = $1 ORDER BY created_at DESC"#, user_id)
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
            updated_at = $3
        WHERE id = $4 AND user_id = $5
        "#,
        name,
        group,
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
    os_type_str: &str,
    os_name: &str,
    arch: &str,
    hostname: &str,
    public_ip_list: &[String], // Changed from CSV to slice of Strings
) -> Result<u64> {
    let now = Utc::now();

    // Find the first IPv4 address from the list
    let first_ipv4 = public_ip_list.iter().find_map(|ip_str| {
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

    // Construct the metadata JSON to be updated
    let agent_info_metadata = json!({
        "os_name": os_name,
        "arch": arch,
        "hostname": hostname,
        "public_ip_addresses": public_ip_list // Store the full list here
    });

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
        first_ipv4, // This will be Option<String>, SQL NULL if None. Max length 45 for ip_address column.
        agent_info_metadata, // Pass the serde_json::Value directly
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
    // New config fields
    vps_config_status: String,
    vps_last_config_update_at: Option<chrono::DateTime<Utc>>,
    vps_last_config_error: Option<String>,
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
            v.config_status as vps_config_status,
            v.last_config_update_at as vps_last_config_update_at,
            v.last_config_error as vps_last_config_error,
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
            v.config_status as vps_config_status,
            v.last_config_update_at as vps_last_config_update_at,
            v.last_config_error as vps_last_config_error,
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
            v.config_status as vps_config_status,
            v.last_config_update_at as vps_last_config_update_at,
            v.last_config_error as vps_last_config_error,
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
        }))
    } else {
        Ok(None)
    }
}