use super::Error;
use crate::db::{self, entities::user};
use db::duckdb_service::DuckDbPool;
use chrono::Utc;
use duckdb::params;

pub async fn get_user_by_username(
    pool: DuckDbPool,
    username: &str,
) -> Result<Option<user::Model>, Error> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare("SELECT * FROM users WHERE username = ?")?;
    let mut rows = stmt.query(params![username])?;

    if let Some(row) = rows.next()? {
        let user_model = user::Model {
            id: row.get("id")?,
            username: row.get("username")?,
            password_hash: row.get("password_hash")?,
            role: row.get("role")?,
            password_login_disabled: row.get("password_login_disabled")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            theme_mode: row.get("theme_mode")?,
            active_theme_id: row.get("active_theme_id")?,
            language: row.get("language")?,
        };
        Ok(Some(user_model))
    } else {
        Ok(None)
    }
}

pub async fn get_user_by_id(
    pool: DuckDbPool,
    user_id: i32,
) -> Result<Option<user::Model>, Error> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare("SELECT * FROM users WHERE id = ?")?;
    let mut rows = stmt.query(params![user_id])?;

    if let Some(row) = rows.next()? {
        let user_model = user::Model {
            id: row.get("id")?,
            username: row.get("username")?,
            password_hash: row.get("password_hash")?,
            role: row.get("role")?,
            password_login_disabled: row.get("password_login_disabled")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
            theme_mode: row.get("theme_mode")?,
            active_theme_id: row.get("active_theme_id")?,
            language: row.get("language")?,
        };
        Ok(Some(user_model))
    } else {
        Ok(None)
    }
}

pub async fn create_user(
    pool: DuckDbPool,
    username: &str,
    password_hash: &str,
) -> Result<user::Model, Error> {
    let conn = pool.get()?;
    let now = Utc::now();
    let role = "user";
    let password_login_disabled = false;
    let theme_mode = "system";
    let language = "auto";

    let id: i32 = conn.query_row(
        "INSERT INTO users (username, password_hash, role, password_login_disabled, created_at, updated_at, theme_mode, language) VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
        params![
            username,
            password_hash,
            role,
            password_login_disabled,
            now,
            now,
            theme_mode,
            language,
        ],
        |row| row.get(0),
    )?;

    Ok(user::Model {
        id,
        username: username.to_string(),
        password_hash: Some(password_hash.to_string()),
        role: role.to_string(),
        password_login_disabled,
        created_at: now,
        updated_at: now,
        theme_mode: theme_mode.to_string(),
        active_theme_id: None,
        language: language.to_string(),
    })
}

pub async fn update_preference(
    pool: DuckDbPool,
    user_id: i32,
    language: &str,
) -> Result<(), Error> {
    let conn = pool.get()?;
    conn.execute(
        "UPDATE users SET language = ?, updated_at = ? WHERE id = ?",
        params![language, Utc::now(), user_id],
    )?;
    Ok(())
}

pub async fn update_username(
    pool: DuckDbPool,
    user_id: i32,
    username: &str,
) -> Result<user::Model, Error> {
    let conn = pool.get()?;
    let now = Utc::now();
    conn.execute(
        "UPDATE users SET username = ?, updated_at = ? WHERE id = ?",
        params![username, now, user_id],
    )?;
    // After updating, we need to return the updated user model.
    // We could query it again, or we can assume the update was successful
    // and construct the model from the input, but that's not ideal
    // if other fields could have been updated by triggers, etc.
    // For now, let's re-fetch.
    get_user_by_id(pool, user_id)
        .await?
        .ok_or_else(|| Error::DuckDB(duckdb::Error::QueryReturnedNoRows))
}

pub async fn update_password(
    pool: DuckDbPool,
    user_id: i32,
    new_password_hash: &str,
) -> Result<(), Error> {
    let conn = pool.get()?;
    let now = Utc::now();
    conn.execute(
        "UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?",
        params![new_password_hash, now, user_id],
    )?;
    Ok(())
}
use crate::db::entities::user_identity_provider;

pub async fn get_connected_accounts(
    pool: DuckDbPool,
    user_id: i32,
) -> Result<Vec<user_identity_provider::Model>, Error> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare("SELECT * FROM user_identity_providers WHERE user_id = ?")?;
    let identities = stmt
        .query_map(params![user_id], |row| {
            Ok(user_identity_provider::Model {
                id: row.get("id")?,
                user_id: row.get("user_id")?,
                provider_name: row.get("provider_name")?,
                provider_user_id: row.get("provider_user_id")?,
                created_at: row.get("created_at")?,
                updated_at: row.get("updated_at")?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(identities)
}

pub async fn unlink_provider(
    pool: DuckDbPool,
    user_id: i32,
    provider: &str,
) -> Result<(), Error> {
    let conn = pool.get()?;
    conn.execute(
        "DELETE FROM user_identity_providers WHERE user_id = ? AND provider_name = ?",
        params![user_id, provider],
    )?;
    Ok(())
}