use tokio::task;
use duckdb::{params, Result as DuckDbResult};
use uuid::Uuid;

use crate::db::duckdb_service::DuckDbPool;
use crate::db::entities::theme;
use crate::web::error::AppError;

fn row_to_theme_model(row: &duckdb::Row<'_>) -> DuckDbResult<theme::Model> {
    Ok(theme::Model {
        id: row.get("id")?,
        user_id: row.get("user_id")?,
        name: row.get("name")?,
        is_official: row.get("is_official")?,
        css: row.get("css")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub async fn get_themes_for_user(pool: DuckDbPool, user_id: i32) -> Result<Vec<theme::Model>, AppError> {
    task::spawn_blocking(move || {
        let conn = pool.get()?;
        let mut stmt = conn.prepare("SELECT * FROM themes WHERE user_id = ? OR is_official = TRUE")?;
        let themes = stmt.query_map(params![user_id], row_to_theme_model)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(themes)
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
}

pub async fn create_theme(pool: DuckDbPool, user_id: i32, name: String, css: String) -> Result<theme::Model, AppError> {
    task::spawn_blocking(move || {
        let conn = pool.get()?;
        let id = Uuid::new_v4();
        let now = chrono::Utc::now();
        let theme = conn.query_row(
            "INSERT INTO themes (id, user_id, name, is_official, css, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING *",
            params![id, user_id, name, false, css, now, now],
            row_to_theme_model,
        )?;
        Ok(theme)
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
}

pub async fn get_theme_by_id(pool: DuckDbPool, theme_id: Uuid, user_id: i32) -> Result<Option<theme::Model>, AppError> {
    task::spawn_blocking(move || {
        let conn = pool.get()?;
        let mut stmt = conn.prepare("SELECT * FROM themes WHERE id = ? AND (user_id = ? OR is_official = TRUE)")?;
        let mut rows = stmt.query_map(params![theme_id, user_id], row_to_theme_model)?;
        
        match rows.next() {
            Some(Ok(theme)) => Ok(Some(theme)),
            Some(Err(e)) => Err(AppError::from(e)),
            None => Ok(None),
        }
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
}

pub async fn update_theme(pool: DuckDbPool, theme_id: Uuid, user_id: i32, name: Option<String>, css: Option<String>) -> Result<theme::Model, AppError> {
    task::spawn_blocking(move || {
        let conn = pool.get()?;
        
        // First, verify the user can edit this theme
        let theme: theme::Model = conn.query_row(
            "SELECT * FROM themes WHERE id = ? AND user_id = ? AND is_official = FALSE",
            params![theme_id, user_id],
            row_to_theme_model,
        ).map_err(|_| AppError::NotFound("Theme not found or you don't have permission to edit it.".to_string()))?;

        let name = name.unwrap_or(theme.name);
        let css = css.unwrap_or(theme.css);
        let now = chrono::Utc::now();

        let updated_theme = conn.query_row(
            "UPDATE themes SET name = ?, css = ?, updated_at = ? WHERE id = ? RETURNING *",
            params![name, css, now, theme_id],
            row_to_theme_model,
        )?;

        Ok(updated_theme)
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
}

pub async fn delete_theme(pool: DuckDbPool, theme_id: Uuid, user_id: i32) -> Result<(), AppError> {
    task::spawn_blocking(move || {
        let conn = pool.get()?;
        let rows_affected = conn.execute(
            "DELETE FROM themes WHERE id = ? AND user_id = ? AND is_official = FALSE",
            params![theme_id, user_id],
        )?;

        if rows_affected == 0 {
            Err(AppError::NotFound("Theme not found or you don't have permission to delete it.".to_string()))
        } else {
            Ok(())
        }
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
}