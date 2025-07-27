use super::json_from_row;
use crate::db::duckdb_service::vps_renewal_service::{
    create_or_update_vps_renewal_info, VpsRenewalDataInput,
};
use crate::db::entities::vps;
use crate::web::error::AppError;
use chrono::{DateTime, Utc};
use crate::db::duckdb_service::DuckDbPool;
use duckdb::{params, Row};
use nodenexus_common::agent_service::AgentHandshake;
use serde_json::json;
use uuid::Uuid;

fn row_to_vps_model(row: &Row) -> Result<vps::Model, duckdb::Error> {
    Ok(vps::Model {
        id: row.get("id")?,
        user_id: row.get("user_id")?,
        name: row.get("name")?,
        ip_address: row.get("ip_address")?,
        os_type: row.get("os_type")?,
        agent_secret: row.get("agent_secret")?,
        agent_version: row.get("agent_version")?,
        status: row.get("status")?,
        metadata: json_from_row(row, "metadata")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        group: row.get("group")?,
        agent_config_override: json_from_row(row, "agent_config_override")?,
        config_status: row.get("config_status")?,
        last_config_update_at: row.get("last_config_update_at")?,
        last_config_error: row.get("last_config_error")?,
        traffic_limit_bytes: row.get("traffic_limit_bytes")?,
        traffic_billing_rule: row.get("traffic_billing_rule")?,
        traffic_current_cycle_rx_bytes: row.get("traffic_current_cycle_rx_bytes")?,
        traffic_current_cycle_tx_bytes: row.get("traffic_current_cycle_tx_bytes")?,
        last_processed_cumulative_rx: row.get("last_processed_cumulative_rx")?,
        last_processed_cumulative_tx: row.get("last_processed_cumulative_tx")?,
        traffic_last_reset_at: row.get("traffic_last_reset_at")?,
        traffic_reset_config_type: row.get("traffic_reset_config_type")?,
        traffic_reset_config_value: row.get("traffic_reset_config_value")?,
        next_traffic_reset_at: row.get("next_traffic_reset_at")?,
    })
}

/// Creates a new VPS entry.
pub async fn create_vps(
    pool: DuckDbPool,
    user_id: i32,
    name: &str,
) -> Result<vps::Model, AppError> {
    let conn = pool.get()?;
    let now = Utc::now();
    let generated_agent_secret = Uuid::new_v4().to_string();

    let id: i32 = conn.query_row(
        "INSERT INTO vps (user_id, name, agent_secret, status, created_at, updated_at, config_status, traffic_current_cycle_rx_bytes, traffic_current_cycle_tx_bytes, last_processed_cumulative_rx, last_processed_cumulative_tx)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
        params![
            user_id,
            name,
            generated_agent_secret,
            "pending",
            now,
            now,
            "unknown",
            0,
            0,
            0,
            0,
        ],
        |row| row.get(0),
    )?;

    Ok(vps::Model {
        id,
        user_id,
        name: name.to_string(),
        ip_address: None,
        os_type: None,
        agent_secret: generated_agent_secret,
        agent_version: None,
        status: "pending".to_string(),
        metadata: None,
        created_at: now,
        updated_at: now,
        group: None,
        agent_config_override: None,
        config_status: "unknown".to_string(),
        last_config_update_at: None,
        last_config_error: None,
        traffic_limit_bytes: None,
        traffic_billing_rule: None,
        traffic_current_cycle_rx_bytes: Some(0),
        traffic_current_cycle_tx_bytes: Some(0),
        last_processed_cumulative_rx: Some(0),
        last_processed_cumulative_tx: Some(0),
        traffic_last_reset_at: None,
        traffic_reset_config_type: None,
        traffic_reset_config_value: None,
        next_traffic_reset_at: None,
    })
}

/// Retrieves a VPS by its ID.
pub async fn get_vps_by_id(
    pool: DuckDbPool,
    vps_id: i32,
) -> Result<Option<vps::Model>, AppError> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare("SELECT * FROM vps WHERE id = ?")?;
    let mut rows = stmt.query_map(params![vps_id], row_to_vps_model)?;
    Ok(rows.next().transpose()?)
}
/// Retrieves multiple VPS entries by their IDs.
pub async fn get_vps_by_ids(
    pool: DuckDbPool,
    vps_ids: Vec<i32>,
) -> Result<Vec<vps::Model>, AppError> {
    if vps_ids.is_empty() {
        return Ok(Vec::new());
    }

    let conn = pool.get()?;
    let params_sql = vps_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!("SELECT * FROM vps WHERE id IN ({params_sql})");

    let mut params_vec: Vec<&dyn duckdb::ToSql> = Vec::new();
    for id in &vps_ids {
        params_vec.push(id);
    }

    let mut stmt = conn.prepare(&sql)?;
    let vps_iter = stmt.query_map(&params_vec[..], row_to_vps_model)?;

    vps_iter.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

/// Retrieves all VPS entries for a given user.
pub async fn get_vps_by_user_id(
    pool: DuckDbPool,
    user_id: i32,
) -> Result<Vec<vps::Model>, AppError> {
    let conn = pool.get()?;
    let mut stmt =
        conn.prepare("SELECT * FROM vps WHERE user_id = ? ORDER BY created_at DESC")?;
    let vps_iter = stmt.query_map(params![user_id], row_to_vps_model)?;
    vps_iter.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

/// Updates a VPS's editable fields.
#[allow(clippy::too_many_arguments)]
pub async fn update_vps(
    pool: DuckDbPool,
    vps_id: i32,
    user_id: i32, // To ensure ownership
    name_opt: Option<String>,
    group_opt: Option<String>,
    tag_ids: Option<Vec<i32>>,
    traffic_limit_bytes_opt: Option<i64>,
    traffic_billing_rule_opt: Option<String>,
    traffic_reset_config_type_opt: Option<String>,
    traffic_reset_config_value_opt: Option<String>,
    next_traffic_reset_at_opt: Option<DateTime<Utc>>,
    renewal_info_input: Option<VpsRenewalDataInput>,
) -> Result<bool, AppError> {
    let mut conn = pool.get()?;
    let now = Utc::now();
    let mut vps_table_changed = false;
    let mut tags_changed = false;
    let mut renewal_info_changed = false;

    // Verify ownership first
    let vps_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM vps WHERE id = ? AND user_id = ?",
        params![vps_id, user_id],
        |row| row.get(0),
    )?;

    if vps_count == 0 {
        return Err(AppError::NotFound(
            "VPS not found or access denied".to_string(),
        ));
    }

    let tx = conn.transaction()?;

    // 1. Update the main VPS table
    let mut set_clauses = Vec::new();
    let mut params_vec: Vec<&dyn duckdb::ToSql> = Vec::new();

    if let Some(name) = &name_opt {
        set_clauses.push("name = ?");
        params_vec.push(name);
        vps_table_changed = true;
    }
    if let Some(group) = &group_opt {
        set_clauses.push("group = ?");
        params_vec.push(group);
        vps_table_changed = true;
    }
    if let Some(limit) = &traffic_limit_bytes_opt {
        set_clauses.push("traffic_limit_bytes = ?");
        params_vec.push(limit);
        vps_table_changed = true;
    }
    if let Some(rule) = &traffic_billing_rule_opt {
        set_clauses.push("traffic_billing_rule = ?");
        params_vec.push(rule);
        vps_table_changed = true;
    }
    if let Some(reset_type) = &traffic_reset_config_type_opt {
        set_clauses.push("traffic_reset_config_type = ?");
        params_vec.push(reset_type);
        vps_table_changed = true;
    }
    if let Some(reset_value) = &traffic_reset_config_value_opt {
        set_clauses.push("traffic_reset_config_value = ?");
        params_vec.push(reset_value);
        vps_table_changed = true;
    }
    if let Some(reset_at) = &next_traffic_reset_at_opt {
        set_clauses.push("next_traffic_reset_at = ?");
        params_vec.push(reset_at);
        vps_table_changed = true;
    }

    if vps_table_changed {
        set_clauses.push("updated_at = ?");
        params_vec.push(&now);

        let sql = format!("UPDATE vps SET {} WHERE id = ?", set_clauses.join(", "));
        params_vec.push(&vps_id);

        tx.execute(&sql, &params_vec[..])?;
    }

    // 2. If tag_ids is provided, update the associations.
    if let Some(ids) = tag_ids {
        tags_changed = true;
        tx.execute("DELETE FROM vps_tags WHERE vps_id = ?", params![vps_id])?;

        if !ids.is_empty() {
            let mut stmt = tx.prepare("INSERT INTO vps_tags (vps_id, tag_id) VALUES (?, ?)")?;
            for tag_id in ids {
                stmt.execute(params![vps_id, tag_id])?;
            }
        }
    }

    // 3. If renewal_info_input is provided, update renewal info
    if let Some(renewal_input) = renewal_info_input {
        create_or_update_vps_renewal_info(&tx, vps_id, &renewal_input)?;
        renewal_info_changed = true;
    }

    tx.commit()?;

    Ok(vps_table_changed || tags_changed || renewal_info_changed)
}
/// Updates the status of a VPS.
pub async fn update_vps_status(
    pool: DuckDbPool,
    vps_id: i32,
    status: &str,
) -> Result<u64, AppError> {
    let conn = pool.get()?;
    let now = Utc::now();
    let rows_affected = conn.execute(
        "UPDATE vps SET status = ?, updated_at = ? WHERE id = ?",
        params![status, now, vps_id],
    )?;
    Ok(rows_affected as u64)
}

/// Updates VPS information based on AgentHandshake data.
pub async fn update_vps_info_on_handshake(
    pool: DuckDbPool,
    vps_id: i32,
    handshake_info: &AgentHandshake,
) -> Result<u64, AppError> {
    let conn = pool.get()?;
    let now = Utc::now();

    let first_ipv4 = handshake_info
        .public_ip_addresses
        .iter()
        .find_map(|ip_str| {
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

    let os_type_str = nodenexus_common::agent_service::OsType::try_from(handshake_info.os_type)
        .map(|os_enum| format!("{os_enum:?}"))
        .unwrap_or_else(|_| "Unknown".to_string());

    let mut agent_info_metadata_map = serde_json::Map::new();
    agent_info_metadata_map.insert("os_name".to_string(), json!(handshake_info.os_name));
    agent_info_metadata_map.insert("arch".to_string(), json!(handshake_info.arch));
    agent_info_metadata_map.insert("hostname".to_string(), json!(handshake_info.hostname));
    agent_info_metadata_map.insert(
        "public_ip_addresses".to_string(),
        json!(handshake_info.public_ip_addresses),
    );
    agent_info_metadata_map.insert(
        "kernel_version".to_string(),
        json!(handshake_info.kernel_version),
    );
    agent_info_metadata_map.insert(
        "os_version_detail".to_string(),
        json!(handshake_info.os_version_detail),
    );
    agent_info_metadata_map.insert(
        "long_os_version".to_string(),
        json!(handshake_info.long_os_version),
    );
    agent_info_metadata_map.insert(
        "distribution_id".to_string(),
        json!(handshake_info.distribution_id),
    );
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
        agent_info_metadata_map.insert(
            "cpu_static_info".to_string(),
            json!({
                "name": cpu_info.name, "frequency": cpu_info.frequency,
                "vendor_id": cpu_info.vendor_id, "brand": cpu_info.brand,
            }),
        );
    }
    if let Some(cc) = &handshake_info.country_code {
        if !cc.is_empty() {
            agent_info_metadata_map.insert("country_code".to_string(), json!(cc));
        }
    }
    let agent_info_metadata = serde_json::Value::Object(agent_info_metadata_map);

    // Fetch current metadata to merge
    let mut stmt = conn.prepare("SELECT metadata FROM vps WHERE id = ?")?;
    let current_metadata_str: Option<String> =
        stmt.query_row(params![vps_id], |row| row.get(0))?;

    let current_metadata: serde_json::Value = current_metadata_str
        .and_then(|s| serde_json::from_str(s.as_str()).ok())
        .unwrap_or_else(|| json!({}));

    let merged_metadata = match current_metadata {
        serde_json::Value::Object(mut current_map) => {
            if let serde_json::Value::Object(new_map) = agent_info_metadata {
                current_map.extend(new_map);
            }
            serde_json::Value::Object(current_map)
        }
        _ => agent_info_metadata, // If current is not an object, overwrite
    };
    let merged_metadata_str = serde_json::to_string(&merged_metadata).unwrap();

    let rows_affected = conn.execute(
        "UPDATE vps SET os_type = ?, ip_address = ?, agent_version = ?, metadata = ?, status = ?, updated_at = ? WHERE id = ?",
        params![
            os_type_str,
            first_ipv4,
            handshake_info.agent_version,
            merged_metadata_str,
            "online",
            now,
            vps_id,
        ],
    )?;

    Ok(rows_affected as u64)
}
/// Retrieves a list of VPS models that are owned by the specified user from a given list of IDs.
pub async fn get_owned_vps_from_ids(
    pool: DuckDbPool,
    user_id: i32,
    vps_ids: &[i32],
) -> Result<Vec<vps::Model>, AppError> {
    if vps_ids.is_empty() {
        return Ok(Vec::new());
    }

    let conn = pool.get()?;
    let params_sql = vps_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT * FROM vps WHERE user_id = ? AND id IN ({params_sql})"
    );

    let mut params_vec: Vec<&dyn duckdb::ToSql> = Vec::new();
    params_vec.push(&user_id);
    for id in vps_ids {
        params_vec.push(id);
    }

    let mut stmt = conn.prepare(&sql)?;
    let vps_iter = stmt.query_map(&params_vec[..], row_to_vps_model)?;

    vps_iter.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

/// Deletes a VPS by its ID.
pub async fn delete_vps(pool: DuckDbPool, vps_id: i32) -> Result<u64, AppError> {
    let conn = pool.get()?;
    let rows_affected = conn.execute("DELETE FROM vps WHERE id = ?", params![vps_id])?;
    Ok(rows_affected as u64)
}