use chrono::{DateTime, Utc};
use serde_json::json;
use sqlx::{PgPool, Result};
use uuid::Uuid;

use crate::db::models::Vps;
use crate::db::services::vps_renewal_service::{create_or_update_vps_renewal_info, VpsRenewalDataInput};

// --- Vps Core Service Functions ---

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
    // Renewal Information
    renewal_info_input: Option<VpsRenewalDataInput>,
) -> Result<bool> {
    let mut tx = pool.begin().await?;
    let now = Utc::now();
    let mut vps_table_changed = false;
    let mut renewal_info_changed = false;

    // 1. Update the main VPS table
    if name.is_some() || group.is_some() || traffic_limit_bytes.is_some() ||
       traffic_billing_rule.is_some() || traffic_reset_config_type.is_some() ||
       traffic_reset_config_value.is_some() || next_traffic_reset_at.is_some() {
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
        if rows_affected > 0 {
            vps_table_changed = true;
        }
    }

    // 2. If tag_ids is provided, update the associations.
    let mut tags_changed = false;
    if let Some(ids) = tag_ids {
        tags_changed = true; 
        sqlx::query!("DELETE FROM vps_tags WHERE vps_id = $1", vps_id)
            .execute(&mut *tx)
            .await?;

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

    // 3. If renewal_info_input is provided, update renewal info
    if let Some(renewal_input) = renewal_info_input {
        create_or_update_vps_renewal_info(&mut tx, vps_id, &renewal_input).await?;
        renewal_info_changed = true;
    }

    tx.commit().await?;

    Ok(vps_table_changed || tags_changed || renewal_info_changed)
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
pub async fn update_vps_info_on_handshake(
    pool: &PgPool,
    vps_id: i32,
    handshake_info: &crate::agent_service::AgentHandshake,
) -> Result<u64> {
    let now = Utc::now();

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
        SET os_type = $1,
            ip_address = $2,
            metadata = COALESCE(metadata, '{}'::jsonb) || $3::jsonb,
            status = $4,
            updated_at = $5
        WHERE id = $6
        "#,
        os_type_str,
        first_ipv4,
        agent_info_metadata,
        "online",
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