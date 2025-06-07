use chrono::Utc;
use sqlx::{PgPool, Result};

use crate::db::models::Setting;

// --- Settings Service Functions ---

/// Retrieves a setting by its key.
pub async fn get_setting(pool: &PgPool, key: &str) -> Result<Option<Setting>> {
    sqlx::query_as!(
        Setting,
        r#"
        SELECT key, value, updated_at
        FROM settings
        WHERE key = $1
        "#,
        key
    )
    .fetch_optional(pool)
    .await
}

/// Creates or updates a setting.
pub async fn update_setting(pool: &PgPool, key: &str, value: &serde_json::Value) -> Result<u64> {
    let now = Utc::now();
    let rows_affected = sqlx::query!(
        r#"
        INSERT INTO settings (key, value, updated_at)
        VALUES ($1, $2, $3)
        ON CONFLICT (key) DO UPDATE SET
            value = EXCLUDED.value,
            updated_at = EXCLUDED.updated_at
        "#,
        key,
        value,
        now
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows_affected)
}

/// Updates a VPS's config override field.
pub async fn update_vps_config_override(
    pool: &PgPool,
    vps_id: i32,
    user_id: i32, // For authorization
    config_override: &serde_json::Value,
) -> Result<u64> {
    let now = Utc::now();
    let rows_affected = sqlx::query!(
        r#"
        UPDATE vps
        SET
            agent_config_override = $1,
            updated_at = $2
        WHERE id = $3 AND user_id = $4
        "#,
        config_override,
        now,
        vps_id,
        user_id
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows_affected)
}

/// Updates the config status of a VPS.
pub async fn update_vps_config_status(
    pool: &PgPool,
    vps_id: i32,
    status: &str,
    error: Option<&str>,
) -> Result<u64> {
    let now = Utc::now();
    let rows_affected = sqlx::query!(
        r#"
        UPDATE vps
        SET
            config_status = $1,
            last_config_error = $2,
            last_config_update_at = $3,
            updated_at = $4
        WHERE id = $5
        "#,
        status,
        error,
        now,
        now,
        vps_id
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows_affected)
}