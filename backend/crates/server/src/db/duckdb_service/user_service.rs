use super::Error;
use crate::db::{self, entities::user};
use crate::web::error::AppError;
use db::duckdb_service::DuckDbPool;
use chrono::Utc;
use duckdb::{params, Result as DuckDbResult};
use tokio::task;

// Helper function to map a DuckDB row to our user model
fn row_to_user_model(row: &duckdb::Row<'_>) -> DuckDbResult<user::Model> {
    Ok(user::Model {
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
    })
}

pub async fn get_user_by_username(
    pool: DuckDbPool,
    username: String,
) -> Result<Option<user::Model>, AppError> {
    task::spawn_blocking(move || {
        let conn = pool.get()?;
        let mut stmt = conn.prepare("SELECT * FROM users WHERE username = ?")?;
        
        let mut user_iter = stmt.query_map(params![username], row_to_user_model)?;

        match user_iter.next() {
            Some(Ok(user)) => Ok(Some(user)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
}

pub async fn get_user_by_id(
    pool: DuckDbPool,
    user_id: i32,
) -> Result<Option<user::Model>, Error> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare("SELECT * FROM users WHERE id = ?")?;
    let mut rows = stmt.query(params![user_id])?;

    if let Some(row) = rows.next()? {
        Ok(Some(row_to_user_model(row)?))
    } else {
        Ok(None)
    }
}

pub async fn create_user(
    pool: DuckDbPool,
    username: String,
    password_hash: String,
) -> Result<user::Model, AppError> {
    task::spawn_blocking(move || {
        let conn = pool.get()?;
        let now = Utc::now();
        let role = "user";
        let password_login_disabled = false;
        let theme_mode = "system";
        let language = "auto";

        let user_model = conn.query_row(
            "INSERT INTO users (username, password_hash, role, password_login_disabled, created_at, updated_at, theme_mode, language) 
             VALUES (?, ?, ?, ?, ?, ?, ?, ?) 
             RETURNING *",
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
            row_to_user_model,
        )?;
        Ok(user_model)
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
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