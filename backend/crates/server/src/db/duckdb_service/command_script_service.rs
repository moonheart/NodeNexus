use crate::db::duckdb_service::DuckDbPool;
use crate::db::entities::command_script::ScriptLanguage;
use crate::web::error::AppError;
use chrono::{DateTime, Utc};
use duckdb::{params, types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef}, Result as DuckDbResult, Row};
use serde::{Deserialize, Serialize};
use std::fmt;
use tokio::task::JoinError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandScript {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub language: ScriptLanguage,
    pub script_content: String,
    pub working_directory: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum CommandScriptServiceError {
    #[error("Database error: {0}")]
    DbErr(#[from] duckdb::Error),
    #[error("Pool error: {0}")]
    PoolError(#[from] r2d2::Error),
    #[error("Script not found: {0}")]
    NotFound(i32),
    #[error("Unauthorized operation")]
    Unauthorized,
    #[error("A script with the name '{0}' already exists.")]
    DuplicateName(String),
    #[error("Tokio join error: {0}")]
    JoinError(#[from] JoinError),
}

impl From<CommandScriptServiceError> for AppError {
    fn from(err: CommandScriptServiceError) -> Self {
        match err {
            CommandScriptServiceError::DbErr(e) => AppError::DatabaseError(e.to_string()),
            CommandScriptServiceError::PoolError(e) => AppError::DatabaseError(e.to_string()),
            CommandScriptServiceError::NotFound(id) => AppError::NotFound(format!("Script with ID {id} not found")),
            CommandScriptServiceError::Unauthorized => AppError::Unauthorized("You are not authorized to perform this action.".to_string()),
            CommandScriptServiceError::DuplicateName(name) => AppError::Conflict(format!("A script with the name '{name}' already exists.")),
            CommandScriptServiceError::JoinError(e) => AppError::InternalServerError(e.to_string()),
        }
    }
}

impl fmt::Display for ScriptLanguage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptLanguage::Shell => write!(f, "shell"),
            ScriptLanguage::PowerShell => write!(f, "powershell"),
        }
    }
}

impl ToSql for ScriptLanguage {
    fn to_sql(&self) -> DuckDbResult<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.to_string()))
    }
}

impl FromSql for ScriptLanguage {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let s = value.as_str()?;
        match s {
            "shell" => Ok(ScriptLanguage::Shell),
            "powershell" => Ok(ScriptLanguage::PowerShell),
            _ => Err(FromSqlError::InvalidType),
        }
    }
}

fn row_to_command_script(row: &Row) -> DuckDbResult<CommandScript> {
    Ok(CommandScript {
        id: row.get("id")?,
        user_id: row.get("user_id")?,
        name: row.get("name")?,
        description: row.get("description")?,
        language: row.get("language")?,
        script_content: row.get("script_content")?,
        working_directory: row.get("working_directory")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub async fn create_script(
    db_pool: DuckDbPool,
    user_id: i32,
    name: String,
    description: Option<String>,
    language: ScriptLanguage,
    script_content: String,
    working_directory: String,
) -> Result<CommandScript, CommandScriptServiceError> {
    let pool = db_pool.clone();
    let name_clone = name.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM command_scripts WHERE user_id = ? AND name = ?",
            params![user_id, name_clone],
            |row| row.get(0),
        )?;
        if count > 0 {
            return Err(CommandScriptServiceError::DuplicateName(name_clone));
        }
        Ok(())
    }).await??;

    let pool = db_pool.clone();
    let name_clone_2 = name.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let now = Utc::now();
        let mut stmt = conn.prepare(
            "INSERT INTO command_scripts (user_id, name, description, language, script_content, working_directory, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING *",
        )?;
        let script = stmt.query_row(
            params![
                user_id,
                name_clone_2,
                description,
                language,
                script_content,
                working_directory,
                now,
                now,
            ],
            row_to_command_script,
        )?;
        Ok(script)
    }).await?
}

pub async fn get_scripts_by_user(
    db_pool: DuckDbPool,
    user_id: i32,
) -> Result<Vec<CommandScript>, CommandScriptServiceError> {
    tokio::task::spawn_blocking(move || {
        let conn = db_pool.get()?;
        let mut stmt = conn.prepare("SELECT * FROM command_scripts WHERE user_id = ?")?;
        let rows = stmt.query_map(params![user_id], row_to_command_script)?;
        let scripts = rows.collect::<DuckDbResult<Vec<_>>>()?;
        Ok(scripts)
    }).await?
}

pub async fn get_script_by_id(
    db_pool: DuckDbPool,
    script_id: i32,
    user_id: i32,
) -> Result<CommandScript, CommandScriptServiceError> {
    tokio::task::spawn_blocking(move || {
        let conn = db_pool.get()?;
        let script = conn.query_row(
            "SELECT * FROM command_scripts WHERE id = ? AND user_id = ?",
            params![script_id, user_id],
            row_to_command_script,
        ).map_err(|_| CommandScriptServiceError::NotFound(script_id))?;
        Ok(script)
    }).await?
}

pub async fn update_script(
    db_pool: DuckDbPool,
    script_id: i32,
    user_id: i32,
    name: String,
    description: Option<String>,
    language: ScriptLanguage,
    script_content: String,
    working_directory: String,
) -> Result<CommandScript, CommandScriptServiceError> {
    let pool = db_pool.clone();
    tokio::task::spawn_blocking(move || {
        let conn = pool.get()?;
        let now = Utc::now();
        let mut stmt = conn.prepare(
            "UPDATE command_scripts SET name = ?, description = ?, language = ?, script_content = ?, working_directory = ?, updated_at = ?
             WHERE id = ? AND user_id = ? RETURNING *",
        )?;
        let script = stmt.query_row(
            params![
                name,
                description,
                language,
                script_content,
                working_directory,
                now,
                script_id,
                user_id,
            ],
            row_to_command_script,
        ).map_err(|_| CommandScriptServiceError::NotFound(script_id))?;
        Ok(script)
    }).await?
}

pub async fn delete_script(
    db_pool: DuckDbPool,
    script_id: i32,
    user_id: i32,
) -> Result<(), CommandScriptServiceError> {
    tokio::task::spawn_blocking(move || {
        let conn = db_pool.get()?;
        let changes = conn.execute(
            "DELETE FROM command_scripts WHERE id = ? AND user_id = ?",
            params![script_id, user_id],
        )?;
        if changes == 0 {
            Err(CommandScriptServiceError::NotFound(script_id))
        } else {
            Ok(())
        }
    }).await?
}