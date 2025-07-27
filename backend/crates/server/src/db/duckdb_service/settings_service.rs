use crate::db::duckdb_service::{json_from_row, DuckDbPool};
use crate::db::entities::setting;
use crate::web::error::AppError;
use chrono::Utc;
use duckdb::{params, Row, Result as DuckDbResult};

fn row_to_setting_model(row: &Row) -> DuckDbResult<setting::Model> {
    let value: Option<serde_json::Value> = json_from_row(row, "value")?;
    Ok(setting::Model {
        key: row.get("key")?,
        value: value.unwrap_or(serde_json::Value::Null),
        updated_at: row.get("updated_at")?,
    })
}

pub async fn get_setting(
    pool: DuckDbPool,
    key: &str,
) -> Result<Option<setting::Model>, AppError> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare("SELECT * FROM settings WHERE key = ?")?;
    let mut rows = stmt.query_map(params![key], row_to_setting_model)?;
    
    match rows.next() {
        Some(res) => Ok(Some(res?)),
        None => Ok(None),
    }
}

pub async fn update_setting(
    pool: DuckDbPool,
    key: &str,
    value: &serde_json::Value,
) -> Result<setting::Model, AppError> {
    let conn = pool.get()?;
    let now = Utc::now();
    let value_str = serde_json::to_string(value)?;

    let setting = conn.query_row(
        "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at
         RETURNING *",
        params![key, value_str, now],
        row_to_setting_model,
    )?;
    Ok(setting)
}

pub async fn update_vps_config_override(
    pool: DuckDbPool,
    vps_id: i32,
    user_id: i32,
    config_override: &serde_json::Value,
) -> Result<u64, AppError> {
    let conn = pool.get()?;
    let now = Utc::now();
    let config_str = serde_json::to_string(config_override)?;

    let rows_affected = conn.execute(
        "UPDATE vps SET agent_config_override = ?, updated_at = ? WHERE id = ? AND user_id = ?",
        params![config_str, now, vps_id, user_id],
    )?;
    Ok(rows_affected as u64)
}

pub async fn update_vps_config_status(
    pool: DuckDbPool,
    vps_id: i32,
    status: &str,
    error: Option<&str>,
) -> Result<u64, AppError> {
    let conn = pool.get()?;
    let now = Utc::now();
    let rows_affected = conn.execute(
        "UPDATE vps SET config_status = ?, last_config_error = ?, last_config_update_at = ?, updated_at = ? WHERE id = ?",
        params![status, error, Some(now), now, vps_id],
    )?;
    Ok(rows_affected as u64)
}