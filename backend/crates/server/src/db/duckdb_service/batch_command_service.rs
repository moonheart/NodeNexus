use crate::db::duckdb_service::DuckDbPool;
use chrono::Utc;
use duckdb::{params, types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef}, Result as DuckDbResult, Row};
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use tracing::error;
use uuid::Uuid;

use crate::db::entities::{batch_command_task, child_command_task};
use crate::db::enums::{BatchCommandStatus, ChildCommandStatus};
use crate::web::error::AppError;
use crate::web::models::batch_command_models::{
    BatchCommandTaskDetailResponse, ChildCommandTaskDetail, CreateBatchCommandRequest,
};
use nodenexus_common::agent_service::OutputType as GrpcOutputType;

// Wrapper for GrpcOutputType to implement Display
struct DisplayableGrpcOutputType(GrpcOutputType);

impl fmt::Display for DisplayableGrpcOutputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            GrpcOutputType::Stdout => write!(f, "stdout"),
            GrpcOutputType::Stderr => write!(f, "stderr"),
            GrpcOutputType::Unspecified => write!(f, "unspecified"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BatchCommandServiceError {
    #[error("Database error: {0}")]
    DbErr(#[from] duckdb::Error),
    #[error("Pool error: {0}")]
    PoolError(#[from] r2d2::Error),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Failed to create batch command: {0}")]
    CreationFailed(String),
    #[error("Batch command task not found: {0}")]
    NotFound(Uuid),
    #[error("Unauthorized to access this resource")]
    Unauthorized,
    #[error("Child task is not in an active state and cannot be terminated")]
    TaskNotTerminable,
    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Tokio join error: {0}")]
    JoinError(#[from] tokio::task::JoinError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<BatchCommandServiceError> for AppError {
    fn from(err: BatchCommandServiceError) -> Self {
        match err {
            BatchCommandServiceError::DbErr(e) => AppError::DatabaseError(e.to_string()),
            BatchCommandServiceError::PoolError(e) => AppError::DatabaseError(e.to_string()),
            BatchCommandServiceError::ValidationError(s) => AppError::InvalidInput(s),
            BatchCommandServiceError::CreationFailed(s) => AppError::InternalServerError(s),
            BatchCommandServiceError::NotFound(id) => AppError::NotFound(format!("Batch command {id} not found")),
            BatchCommandServiceError::Unauthorized => AppError::Unauthorized("Unauthorized".to_string()),
            BatchCommandServiceError::TaskNotTerminable => AppError::Conflict("Task not terminable".to_string()),
            BatchCommandServiceError::JsonError(e) => AppError::InternalServerError(e.to_string()),
            BatchCommandServiceError::JoinError(e) => AppError::InternalServerError(e.to_string()),
            BatchCommandServiceError::IoError(e) => AppError::InternalServerError(e.to_string()),
        }
    }
}

impl ToSql for BatchCommandStatus {
    fn to_sql(&self) -> DuckDbResult<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.to_string()))
    }
}

impl FromSql for BatchCommandStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let s = value.as_str()?;
        BatchCommandStatus::from_str(s).map_err(|_| FromSqlError::InvalidType)
    }
}

impl ToSql for ChildCommandStatus {
    fn to_sql(&self) -> DuckDbResult<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.to_string()))
    }
}

impl FromSql for ChildCommandStatus {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let s = value.as_str()?;
        ChildCommandStatus::from_str(s).map_err(|_| FromSqlError::InvalidType)
    }
}

fn row_to_batch_command_task(row: &Row) -> DuckDbResult<batch_command_task::Model> {
    let payload_str: String = row.get("original_request_payload")?;
    let original_request_payload = serde_json::from_str(&payload_str).map_err(|e| duckdb::Error::FromSqlConversionFailure(1, duckdb::types::Type::Text, Box::new(e)))?;

    Ok(batch_command_task::Model {
        batch_command_id: row.get("batch_command_id")?,
        original_request_payload,
        status: row.get("status")?,
        execution_alias: row.get("execution_alias")?,
        user_id: row.get("user_id")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        completed_at: row.get("completed_at")?,
    })
}

fn row_to_child_command_task(row: &Row) -> DuckDbResult<child_command_task::Model> {
    Ok(child_command_task::Model {
        child_command_id: row.get("child_command_id")?,
        batch_command_id: row.get("batch_command_id")?,
        vps_id: row.get("vps_id")?,
        status: row.get("status")?,
        exit_code: row.get("exit_code")?,
        error_message: row.get("error_message")?,
        stdout_log_path: row.get("stdout_log_path")?,
        stderr_log_path: row.get("stderr_log_path")?,
        last_output_at: row.get("last_output_at")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        agent_started_at: row.get("agent_started_at")?,
        agent_completed_at: row.get("agent_completed_at")?,
    })
}

pub async fn create_batch_command(
    db_pool: DuckDbPool,
    user_id: i32,
    request: CreateBatchCommandRequest,
) -> Result<(batch_command_task::Model, Vec<child_command_task::Model>), BatchCommandServiceError> {
    if request.command_content.is_none() && request.script_id.is_none() {
        return Err(BatchCommandServiceError::ValidationError("Either command_content or script_id must be provided.".to_string()));
    }
    if request.command_content.is_some() && request.script_id.is_some() {
        return Err(BatchCommandServiceError::ValidationError("Provide either command_content or script_id, not both.".to_string()));
    }
    if request.target_vps_ids.is_empty() {
        return Err(BatchCommandServiceError::ValidationError("At least one target_vps_id must be provided.".to_string()));
    }

    let db_pool_clone = db_pool.clone();
    let task = tokio::task::spawn_blocking(move || -> Result<_, BatchCommandServiceError> {
        let mut conn = db_pool_clone.get()?;
        let tx = conn.transaction()?;
        let batch_command_id = Uuid::new_v4();
        let now = Utc::now();
        let original_request_payload = serde_json::to_string(&request)?;

        tx.execute(
            "INSERT INTO batch_command_tasks (batch_command_id, original_request_payload, status, execution_alias, user_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                batch_command_id,
                original_request_payload,
                BatchCommandStatus::Pending,
                request.execution_alias,
                user_id,
                now,
                now,
            ],
        )?;

        let mut child_tasks_to_create = Vec::new();
        for vps_id in request.target_vps_ids {
            child_tasks_to_create.push((
                Uuid::new_v4(),
                batch_command_id,
                vps_id,
                ChildCommandStatus::Pending,
                now,
                now,
            ));
        }

        if !child_tasks_to_create.is_empty() {
            let mut stmt = tx.prepare(
                "INSERT INTO child_command_tasks (child_command_id, batch_command_id, vps_id, status, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?)",
            )?;
            for task in child_tasks_to_create {
                stmt.execute(params![task.0, task.1, task.2, task.3, task.4, task.5])?;
            }
        }

        tx.commit()?;
        Ok(batch_command_id)
    }).await??;

    let saved_batch_task = get_batch_task(db_pool.clone(), &task).await?;
    let created_child_tasks = get_child_tasks_for_batch(db_pool, &task).await?;

    Ok((saved_batch_task, created_child_tasks))
}

pub async fn get_batch_task(db_pool: DuckDbPool, batch_command_id: &Uuid) -> Result<batch_command_task::Model, BatchCommandServiceError> {
    let id = *batch_command_id;
    tokio::task::spawn_blocking(move || -> Result<_, BatchCommandServiceError> {
        let conn = db_pool.get()?;
        let task = conn.query_row(
            "SELECT * FROM batch_command_tasks WHERE batch_command_id = ?",
            params![id],
            row_to_batch_command_task,
        )?;
        Ok(task)
    }).await?
}

pub async fn get_child_tasks_for_batch(db_pool: DuckDbPool, batch_command_id: &Uuid) -> Result<Vec<child_command_task::Model>, BatchCommandServiceError> {
    let id = *batch_command_id;
    tokio::task::spawn_blocking(move || -> Result<_, BatchCommandServiceError> {
        let conn = db_pool.get()?;
        let mut stmt = conn.prepare("SELECT * FROM child_command_tasks WHERE batch_command_id = ?")?;
        let rows = stmt.query_map(params![id], row_to_child_command_task)?;
        let tasks = rows.collect::<DuckDbResult<Vec<_>>>()?;
        Ok(tasks)
    }).await?
}

pub async fn get_batch_command_detail_dto(
    db_pool: DuckDbPool,
    batch_command_id: Uuid,
    requesting_user_id: i32,
) -> Result<Option<BatchCommandTaskDetailResponse>, BatchCommandServiceError> {
    let batch_task_model = get_batch_task(db_pool.clone(), &batch_command_id).await?;

    if batch_task_model.user_id != requesting_user_id {
        return Err(BatchCommandServiceError::Unauthorized);
    }

    let child_tasks_models = get_child_tasks_for_batch(db_pool, &batch_command_id).await?;

    let child_task_details: Vec<ChildCommandTaskDetail> = child_tasks_models
        .into_iter()
        .map(|ct| ChildCommandTaskDetail {
            child_command_id: ct.child_command_id,
            vps_id: ct.vps_id,
            status: ct.status.to_string(),
            exit_code: ct.exit_code,
            error_message: ct.error_message,
            created_at: ct.created_at,
            updated_at: ct.updated_at,
            agent_started_at: ct.agent_started_at,
            agent_completed_at: ct.agent_completed_at,
            last_output_at: ct.last_output_at,
        })
        .collect();

    let response_dto = BatchCommandTaskDetailResponse {
        batch_command_id: batch_task_model.batch_command_id,
        overall_status: batch_task_model.status.to_string(),
        execution_alias: batch_task_model.execution_alias,
        user_id: batch_task_model.user_id.to_string(),
        original_request_payload: batch_task_model.original_request_payload,
        tasks: child_task_details,
        created_at: batch_task_model.created_at,
        updated_at: batch_task_model.updated_at,
        completed_at: batch_task_model.completed_at,
    };
    Ok(Some(response_dto))
}

pub async fn terminate_batch_command(db_pool: DuckDbPool, batch_command_id: Uuid, user_id: i32) -> Result<Vec<(Uuid, i32)>, BatchCommandServiceError> {
    tokio::task::spawn_blocking(move || -> Result<_, BatchCommandServiceError> {
        let mut conn = db_pool.get()?;
        let tx = conn.transaction()?;

        let batch_task: batch_command_task::Model = tx.query_row(
            "SELECT * FROM batch_command_tasks WHERE batch_command_id = ?",
            params![batch_command_id],
            row_to_batch_command_task,
        )
        .map_err(|_| BatchCommandServiceError::NotFound(batch_command_id))?;

        if batch_task.user_id != user_id {
            return Err(BatchCommandServiceError::Unauthorized);
        }

        tx.execute(
            "UPDATE batch_command_tasks SET status = ?, updated_at = ? WHERE batch_command_id = ?",
            params![BatchCommandStatus::Terminating, Utc::now(), batch_command_id],
        )?;

        let mut stmt = tx.prepare("SELECT * FROM child_command_tasks WHERE batch_command_id = ?")?;
        let child_tasks = stmt.query_map(params![batch_command_id], row_to_child_command_task)?
            .collect::<Result<Vec<_>, _>>()?;

        let active_child_tasks: Vec<(Uuid, i32)> = child_tasks.into_iter()
            .filter(|task| matches!(task.status, ChildCommandStatus::Pending | ChildCommandStatus::Executing | ChildCommandStatus::SentToAgent | ChildCommandStatus::AgentAccepted))
            .map(|task| (task.child_command_id, task.vps_id))
            .collect();

        if !active_child_tasks.is_empty() {
            let active_child_task_ids: Vec<Uuid> = active_child_tasks.iter().map(|(id, _)| *id).collect();
            let params_sql = active_child_task_ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
            let sql = format!("UPDATE child_command_tasks SET status = ?, updated_at = ? WHERE child_command_id IN ({params_sql})");
            
            let status = ChildCommandStatus::Terminating;
            let now = Utc::now();
            let mut params_vec: Vec<&dyn ToSql> = vec![&status, &now];
            for id in &active_child_task_ids {
                params_vec.push(id);
            }
            tx.execute(&sql, &params_vec[..])?;
        }

        tx.commit()?;
        Ok(active_child_tasks)
    }).await?
}

pub async fn terminate_single_child_task(db_pool: DuckDbPool, child_command_id: Uuid, user_id: i32) -> Result<Option<(Uuid, i32)>, BatchCommandServiceError> {
    tokio::task::spawn_blocking(move || -> Result<_, BatchCommandServiceError> {
        let mut conn = db_pool.get()?;
        let tx = conn.transaction()?;

        let child_task: child_command_task::Model = tx.query_row(
            "SELECT * FROM child_command_tasks WHERE child_command_id = ?",
            params![child_command_id],
            row_to_child_command_task,
        )
        .map_err(|_| BatchCommandServiceError::NotFound(child_command_id))?;

        let batch_task: batch_command_task::Model = tx.query_row(
            "SELECT * FROM batch_command_tasks WHERE batch_command_id = ?",
            params![child_task.batch_command_id],
            row_to_batch_command_task,
        )?;

        if batch_task.user_id != user_id {
            return Err(BatchCommandServiceError::Unauthorized);
        }

        if !matches!(child_task.status, ChildCommandStatus::Pending | ChildCommandStatus::Executing | ChildCommandStatus::SentToAgent | ChildCommandStatus::AgentAccepted) {
            return Err(BatchCommandServiceError::TaskNotTerminable);
        }

        tx.execute(
            "UPDATE child_command_tasks SET status = ?, updated_at = ? WHERE child_command_id = ?",
            params![ChildCommandStatus::Terminating, Utc::now(), child_command_id],
        )?;

        tx.commit()?;
        Ok(Some((child_task.child_command_id, child_task.vps_id)))
    }).await?
}

pub async fn update_child_task_status(db_pool: DuckDbPool, child_command_id: Uuid, status: ChildCommandStatus, error_message: Option<String>) -> Result<(), BatchCommandServiceError> {
    tokio::task::spawn_blocking(move || -> Result<_, BatchCommandServiceError> {
        let mut conn = db_pool.get()?;
        let tx = conn.transaction()?;

        let now = Utc::now();
        let mut params: Vec<&dyn ToSql> = vec![&status, &now];
        let mut set_clauses = vec!["status = ?", "updated_at = ?"];

        if let Some(ref msg) = error_message {
            set_clauses.push("error_message = ?");
            params.push(msg);
        }

        let sql = format!("UPDATE child_command_tasks SET {} WHERE child_command_id = ?", set_clauses.join(", "));
        params.push(&child_command_id);

        tx.execute(&sql, &params[..])?;
        tx.commit()?;
        Ok(())
    }).await?
}

pub async fn record_child_task_output(db_pool: DuckDbPool, child_command_id: Uuid, output: &str, output_type: GrpcOutputType) -> Result<(), BatchCommandServiceError> {
    let output = output.to_string();
    tokio::task::spawn_blocking(move || -> Result<_, BatchCommandServiceError> {
        let mut conn = db_pool.get()?;
        let tx = conn.transaction()?;

        let log_path_col = match output_type {
            GrpcOutputType::Stdout => "stdout_log_path",
            GrpcOutputType::Stderr => "stderr_log_path",
            _ => return Ok(()),
        };

        let log_path: Option<String> = tx.query_row(
            &format!("SELECT {log_path_col} FROM child_command_tasks WHERE child_command_id = ?"),
            params![child_command_id],
            |row| row.get(0),
        )?;

        let log_path = match log_path {
            Some(p) => PathBuf::from(p),
            None => {
                let new_path = PathBuf::from("logs").join(format!("{}_{}.log", child_command_id, DisplayableGrpcOutputType(output_type)));
                if let Some(parent) = new_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                tx.execute(
                    &format!("UPDATE child_command_tasks SET {log_path_col} = ? WHERE child_command_id = ?"),
                    params![new_path.to_str(), child_command_id],
                )?;
                new_path
            }
        };

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;
        file.write_all(output.as_bytes())?;

        tx.execute(
            "UPDATE child_command_tasks SET last_output_at = ? WHERE child_command_id = ?",
            params![Utc::now(), child_command_id],
        )?;

        tx.commit()?;
        Ok(())
    }).await?
}

pub async fn check_and_update_batch_task_status(db_pool: DuckDbPool, batch_command_id: Uuid) -> Result<(), BatchCommandServiceError> {
    tokio::task::spawn_blocking(move || -> Result<_, BatchCommandServiceError> {
        let mut conn = db_pool.get()?;
        let tx = conn.transaction()?;

        let mut stmt = tx.prepare("SELECT status FROM child_command_tasks WHERE batch_command_id = ?")?;
        let child_statuses = stmt.query_map(params![batch_command_id], |row| row.get(0))?
            .collect::<Result<Vec<ChildCommandStatus>, _>>()?;

        let total_tasks = child_statuses.len();
        let mut completed_successfully = 0;
        let mut completed_with_failure = 0;
        let mut terminated = 0;

        for status in child_statuses {
            match status {
                ChildCommandStatus::CompletedSuccessfully => completed_successfully += 1,
                ChildCommandStatus::CompletedWithFailure | ChildCommandStatus::AgentUnreachable | ChildCommandStatus::TimedOut | ChildCommandStatus::AgentError => completed_with_failure += 1,
                ChildCommandStatus::Terminated => terminated += 1,
                _ => (),
            }
        }

        let new_status = if completed_successfully + completed_with_failure + terminated == total_tasks {
            if completed_with_failure > 0 {
                Some(BatchCommandStatus::CompletedWithErrors)
            } else if terminated > 0 && completed_successfully + terminated == total_tasks {
                Some(BatchCommandStatus::Terminated)
            } else {
                Some(BatchCommandStatus::CompletedSuccessfully)
            }
        } else {
            None
        };

        if let Some(status) = new_status {
            tx.execute(
                "UPDATE batch_command_tasks SET status = ?, completed_at = ?, updated_at = ? WHERE batch_command_id = ?",
                params![status, Utc::now(), Utc::now(), batch_command_id],
            )?;
        }

        tx.commit()?;
        Ok(())
    }).await?
}