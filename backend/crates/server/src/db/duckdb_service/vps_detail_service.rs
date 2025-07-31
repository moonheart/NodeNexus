use std::collections::HashMap;
use duckdb::{params, Connection};
use crate::db::duckdb_service::{json_from_row, DuckDbPool};
use crate::db::entities::{vps, vps_renewal_info};
use crate::web::error::AppError;
use crate::web::models::websocket_models::{ServerBasicInfo, ServerWithDetails, Tag as WebsocketTag};

// Helper function to map a DuckDB row to a vps::Model
fn row_to_vps_model(row: &duckdb::Row<'_>) -> Result<vps::Model, duckdb::Error> {
    Ok(vps::Model {
        id: row.get("vps_id")?,
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

// Helper function to map a DuckDB row to an optional vps_renewal_info::Model
fn row_to_renewal_info(row: &duckdb::Row<'_>) -> Result<Option<vps_renewal_info::Model>, duckdb::Error> {
    // If the primary key is null, the whole record is considered null
    if row.get::<_, Option<i32>>("ri_vps_id")?.is_none() {
        return Ok(None);
    }
    Ok(Some(vps_renewal_info::Model {
        vps_id: row.get("ri_vps_id")?,
        renewal_cycle: row.get("renewal_cycle")?,
        renewal_cycle_custom_days: row.get("renewal_cycle_custom_days")?,
        renewal_price: row.get("renewal_price")?,
        renewal_currency: row.get("renewal_currency")?,
        next_renewal_date: row.get("next_renewal_date")?,
        last_renewal_date: row.get("last_renewal_date")?,
        service_start_date: row.get("service_start_date")?,
        payment_method: row.get("payment_method")?,
        auto_renew_enabled: row.get("auto_renew_enabled")?,
        renewal_notes: row.get("renewal_notes")?,
        reminder_active: row.get("reminder_active")?,
        last_reminder_generated_at: row.get("last_reminder_generated_at")?,
        created_at: row.get("ri_created_at")?,
        updated_at: row.get("ri_updated_at")?,
    }))
}

// Helper function to map a DuckDB row to an optional WebsocketTag
fn row_to_tag(row: &duckdb::Row<'_>) -> Result<Option<WebsocketTag>, duckdb::Error> {
    // If the primary key is null, the whole record is considered null
    if row.get::<_, Option<i32>>("tag_id")?.is_none() {
        return Ok(None);
    }
    Ok(Some(WebsocketTag {
        id: row.get("tag_id")?,
        name: row.get("tag_name")?,
        color: row.get("tag_color")?,
        icon: row.get("tag_icon")?,
        url: row.get("tag_url")?,
        is_visible: row.get("tag_is_visible")?,
    }))
}

fn build_server_with_details(
    vps_model: vps::Model,
    renewal_info_opt: Option<vps_renewal_info::Model>,
    tags: Option<Vec<WebsocketTag>>,
) -> ServerWithDetails {
    let basic_info = ServerBasicInfo {
        id: vps_model.id,
        user_id: vps_model.user_id,
        name: vps_model.name,
        ip_address: vps_model.ip_address,
        status: vps_model.status,
        agent_version: vps_model.agent_version,
        group: vps_model.group,
        tags,
        config_status: vps_model.config_status,
        last_config_update_at: vps_model.last_config_update_at,
        last_config_error: vps_model.last_config_error,
        traffic_limit_bytes: vps_model.traffic_limit_bytes,
        traffic_billing_rule: vps_model.traffic_billing_rule,
        traffic_current_cycle_rx_bytes: vps_model.traffic_current_cycle_rx_bytes,
        traffic_current_cycle_tx_bytes: vps_model.traffic_current_cycle_tx_bytes,
        traffic_last_reset_at: vps_model.traffic_last_reset_at,
        traffic_reset_config_type: vps_model.traffic_reset_config_type,
        traffic_reset_config_value: vps_model.traffic_reset_config_value,
        next_traffic_reset_at: vps_model.next_traffic_reset_at,
    };

    ServerWithDetails {
        basic_info,
        os_type: vps_model.os_type,
        created_at: vps_model.created_at,
        metadata: vps_model.metadata,
        renewal_cycle: renewal_info_opt.as_ref().and_then(|ri| ri.renewal_cycle.clone()),
        renewal_cycle_custom_days: renewal_info_opt.as_ref().and_then(|ri| ri.renewal_cycle_custom_days),
        renewal_price: renewal_info_opt.as_ref().and_then(|ri| ri.renewal_price),
        renewal_currency: renewal_info_opt.as_ref().and_then(|ri| ri.renewal_currency.clone()),
        next_renewal_date: renewal_info_opt.as_ref().and_then(|ri| ri.next_renewal_date),
        last_renewal_date: renewal_info_opt.as_ref().and_then(|ri| ri.last_renewal_date),
        service_start_date: renewal_info_opt.as_ref().and_then(|ri| ri.service_start_date),
        payment_method: renewal_info_opt.as_ref().and_then(|ri| ri.payment_method.clone()),
        auto_renew_enabled: renewal_info_opt.as_ref().and_then(|ri| ri.auto_renew_enabled),
        renewal_notes: renewal_info_opt.as_ref().and_then(|ri| ri.renewal_notes.clone()),
        reminder_active: renewal_info_opt.as_ref().and_then(|ri| ri.reminder_active),
    }
}

const SELECT_VPS_WITH_DETAILS_SQL: &str = "
    SELECT
        v.id as vps_id, v.user_id, v.name, v.ip_address, v.os_type, v.agent_secret, v.agent_version, v.status, v.metadata, v.created_at, v.updated_at, v.group, v.agent_config_override, v.config_status, v.last_config_update_at, v.last_config_error, v.traffic_limit_bytes, v.traffic_billing_rule, v.traffic_current_cycle_rx_bytes, v.traffic_current_cycle_tx_bytes, v.last_processed_cumulative_rx, v.last_processed_cumulative_tx, v.traffic_last_reset_at, v.traffic_reset_config_type, v.traffic_reset_config_value, v.next_traffic_reset_at,
        ri.vps_id as ri_vps_id, ri.renewal_cycle, ri.renewal_cycle_custom_days, ri.renewal_price, ri.renewal_currency, ri.next_renewal_date, ri.last_renewal_date, ri.service_start_date, ri.payment_method, ri.auto_renew_enabled, ri.renewal_notes, ri.reminder_active, ri.last_reminder_generated_at, ri.created_at as ri_created_at, ri.updated_at as ri_updated_at,
        t.id as tag_id, t.name as tag_name, t.color as tag_color, t.icon as tag_icon, t.url as tag_url, t.is_visible as tag_is_visible
    FROM vps v
    LEFT JOIN vps_renewal_info ri ON v.id = ri.vps_id
    LEFT JOIN vps_tags vt ON v.id = vt.vps_id
    LEFT JOIN tags t ON vt.tag_id = t.id
";

fn process_query_results(
    conn: &mut Connection,
    query: &str,
    params: &[&dyn duckdb::ToSql],
) -> Result<Vec<ServerWithDetails>, AppError> {
    let mut stmt = conn.prepare(query).map_err(|e| AppError::DatabaseError(e.to_string()))?;
    let mut rows = stmt.query(params).map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let mut vps_map: HashMap<i32, (vps::Model, Option<vps_renewal_info::Model>, Vec<WebsocketTag>)> = HashMap::new();

    while let Some(row) = rows.next().map_err(|e| AppError::DatabaseError(e.to_string()))? {
        let vps_id = row.get("vps_id").map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let entry = vps_map.entry(vps_id).or_insert_with_key(|_| {
            let vps_model = row_to_vps_model(row).unwrap();
            let renewal_info = row_to_renewal_info(row).unwrap();
            (vps_model, renewal_info, Vec::new())
        });

        if let Some(tag) = row_to_tag(row).map_err(|e| AppError::DatabaseError(e.to_string()))? {
            entry.2.push(tag);
        }
    }

    let mut servers_with_details = vps_map
        .into_values()
        .map(|(vps_model, renewal_info, tags)| {
            let tags_opt = if tags.is_empty() { None } else { Some(tags) };
            build_server_with_details(vps_model, renewal_info, tags_opt)
        })
        .collect::<Vec<_>>();
    
    servers_with_details.sort_by(|a, b| a.basic_info.id.cmp(&b.basic_info.id));

    Ok(servers_with_details)
}

pub async fn get_all_vps_with_details_for_cache(pool: DuckDbPool) -> Result<Vec<ServerWithDetails>, AppError> {
    let mut conn = pool.get().map_err(|e| AppError::DatabaseError(e.to_string()))?;
    let query = format!("{SELECT_VPS_WITH_DETAILS_SQL} ORDER BY v.id ASC");
    process_query_results(&mut conn, &query, &[])
}

pub async fn get_all_vps_with_details_for_user(pool: DuckDbPool, user_id: i32) -> Result<Vec<ServerWithDetails>, AppError> {
    let mut conn = pool.get().map_err(|e| AppError::DatabaseError(e.to_string()))?;
    let query = format!("{SELECT_VPS_WITH_DETAILS_SQL} WHERE v.user_id = ? ORDER BY v.id ASC");
    process_query_results(&mut conn, &query, params![user_id])}

pub async fn get_vps_with_details_for_cache_by_id(pool: DuckDbPool, vps_id: i32) -> Result<Option<ServerWithDetails>, AppError> {
    let mut conn = pool.get().map_err(|e| AppError::DatabaseError(e.to_string()))?;
    let query = format!("{SELECT_VPS_WITH_DETAILS_SQL} WHERE v.id = ? LIMIT 1");
    let mut results = process_query_results(&mut conn, &query, params![vps_id])?;
    Ok(results.pop())
}