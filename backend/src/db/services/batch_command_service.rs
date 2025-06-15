use sea_orm::{DatabaseConnection, Set, ActiveModelTrait, TransactionTrait, DbErr, ColumnTrait}; // Ensure ColumnTrait is here
use std::sync::Arc;
use crate::server::result_broadcaster::ResultBroadcaster; // Added import
use std::fmt; // Added for Display trait
use std::path::PathBuf; // For constructing paths
use tokio::fs; // For async file operations (creating directory)
use uuid::Uuid;
use chrono::Utc;
use tracing::error;

use crate::db::entities::{
    batch_command_task,
    child_command_task,
    batch_command_task::Entity as BatchTaskEntity, // Alias for clarity
    child_command_task::Entity as ChildTaskEntity, // Alias for clarity
};
use crate::db::enums::{BatchCommandStatus, ChildCommandStatus}; // Added import
use crate::http_server::models::{CreateBatchCommandRequest, BatchCommandTaskDetailResponse, ChildCommandTaskDetail}; // API DTOs
use sea_orm::{EntityTrait, QueryFilter, ModelTrait}; // Removed ColumnTrait from here
use crate::agent_service::OutputType as GrpcOutputType; // For record_child_task_output

// Wrapper for GrpcOutputType to implement Display
struct DisplayableGrpcOutputType(GrpcOutputType);

impl fmt::Display for DisplayableGrpcOutputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            GrpcOutputType::Stdout => write!(f, "stdout"),
            GrpcOutputType::Stderr => write!(f, "stderr"),
            GrpcOutputType::Unspecified => write!(f, "unspecified"),
            // If GrpcOutputType is non-exhaustive or has other variants,
            // you might need a wildcard arm:
            // _ => write!(f, "unknown"),
        }
    }
}

// Placeholder for a more comprehensive error type for this service
#[derive(Debug, thiserror::Error)]
pub enum BatchCommandServiceError {
    #[error("Database error: {0}")]
    DbErr(#[from] DbErr),
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
}

#[derive(Clone, Debug)] // Added Debug
pub struct BatchCommandManager {
    db: Arc<DatabaseConnection>,
    result_broadcaster: Arc<ResultBroadcaster>,
}

impl BatchCommandManager {
    pub fn new(db: Arc<DatabaseConnection>, result_broadcaster: Arc<ResultBroadcaster>) -> Self {
        Self { db, result_broadcaster }
    }

    pub async fn create_batch_command(
        &self,
        user_id: i32, // Change to i32
        request: CreateBatchCommandRequest,
    ) -> Result<(batch_command_task::Model, Vec<child_command_task::Model>), BatchCommandServiceError> {
        // Validate request
        if request.command_content.is_none() && request.script_id.is_none() {
            return Err(BatchCommandServiceError::ValidationError(
                "Either command_content or script_id must be provided.".to_string(),
            ));
        }
        if request.command_content.is_some() && request.script_id.is_some() {
            return Err(BatchCommandServiceError::ValidationError(
                "Provide either command_content or script_id, not both.".to_string(),
            ));
        }
        if request.target_vps_ids.is_empty() {
            return Err(BatchCommandServiceError::ValidationError(
                "At least one target_vps_id must be provided.".to_string(),
            ));
        }

        let batch_command_id = Uuid::new_v4();
        let now = Utc::now();

        let txn = self.db.begin().await?;

        let batch_task = batch_command_task::ActiveModel {
            batch_command_id: Set(batch_command_id),
            original_request_payload: Set(serde_json::to_value(&request)
                .map_err(|e| BatchCommandServiceError::CreationFailed(format!("Failed to serialize request: {}", e)))?),
            status: Set(BatchCommandStatus::Pending), // Initial status
            execution_alias: Set(request.execution_alias.clone()),
            user_id: Set(user_id),
            created_at: Set(now.clone()),
            updated_at: Set(now.clone()),
            completed_at: Set(None),
        };

        let saved_batch_task = batch_task.insert(&txn).await?;

        let mut child_tasks_to_create = Vec::new();
        for vps_id in request.target_vps_ids {
            let child_task = child_command_task::ActiveModel {
                child_command_id: Set(Uuid::new_v4()),
                batch_command_id: Set(batch_command_id),
                vps_id: Set(vps_id), // Assuming vps_id is String
                status: Set(ChildCommandStatus::Pending),
                exit_code: Set(None),
                error_message: Set(None),
                stdout_log_path: Set(None),
                stderr_log_path: Set(None),
                last_output_at: Set(None),
                created_at: Set(now.clone()),
                updated_at: Set(now.clone()),
                agent_started_at: Set(None),
                agent_completed_at: Set(None),
            };
            child_tasks_to_create.push(child_task);
        }

        if !child_tasks_to_create.is_empty() {
            // Corrected: Call insert_many on the Entity, not the alias if it's for the Model
            // ChildTaskEntity is an alias for child_command_task::Entity
            ChildTaskEntity::insert_many(child_tasks_to_create)
                .exec(&txn)
                .await?;
        }

        txn.commit().await?;

        // Fetch the child tasks that were just inserted to return them
        let created_child_tasks = ChildTaskEntity::find()
            .filter(child_command_task::Column::BatchCommandId.eq(batch_command_id))
            .all(self.db.as_ref()) // Use self.db directly as it's Arc<DatabaseConnection>
            .await?;
 
        Ok((saved_batch_task, created_child_tasks))
    }
 
    pub async fn get_batch_command_detail_dto(
        &self,
        batch_command_id: Uuid,
        requesting_user_id: i32, // Change to i32
    ) -> Result<Option<BatchCommandTaskDetailResponse>, BatchCommandServiceError> {
        let batch_task_model = BatchTaskEntity::find_by_id(batch_command_id)
            .one(self.db.as_ref())
            .await?;

        match batch_task_model {
            Some(task) => {
                if task.user_id != requesting_user_id {
                    return Err(BatchCommandServiceError::Unauthorized);
                }

                let child_tasks_models = task
                    .find_related(ChildTaskEntity)
                    .all(self.db.as_ref())
                    .await?;

                let child_task_details: Vec<ChildCommandTaskDetail> = child_tasks_models
                    .into_iter()
                    .map(|ct| ChildCommandTaskDetail {
                        child_command_id: ct.child_command_id,
                        vps_id: ct.vps_id,
                        status: ct.status.to_string(), // Now Display is implemented
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
                    batch_command_id: task.batch_command_id,
                    overall_status: task.status.to_string(), // Now Display is implemented
                    execution_alias: task.execution_alias,
                    user_id: task.user_id.to_string(),
                    original_request_payload: task.original_request_payload,
                    tasks: child_task_details,
                    created_at: task.created_at,
                    updated_at: task.updated_at,
                    completed_at: task.completed_at,
                };
                Ok(Some(response_dto))
            }
            None => Ok(None),
        }
    }

    pub async fn terminate_batch_command(
        &self,
        batch_command_id: Uuid,
        requesting_user_id: i32,
    ) -> Result<Vec<child_command_task::Model>, BatchCommandServiceError> {
        let txn = self.db.begin().await?;

        let batch_task_model = BatchTaskEntity::find_by_id(batch_command_id)
            .one(&txn)
            .await?;

        let task_to_terminate = match batch_task_model {
            Some(task) => {
                if task.user_id != requesting_user_id {
                    return Err(BatchCommandServiceError::Unauthorized);
                }
                // Check if already completed or terminated
                if task.status == BatchCommandStatus::CompletedSuccessfully || task.status == BatchCommandStatus::CompletedWithErrors || task.status == BatchCommandStatus::Terminated {
                    // Optionally return Ok if already in a final state, or an error/specific status
                    // If already terminated, we might still want to return the child tasks if the caller expects them.
                    // For now, if parent is already terminated, return empty list, assuming no further action needed.
                    // Or, we could fetch child tasks regardless and let caller decide.
                    // Let's return empty Vec if already in a final state.
                    return Ok(Vec::new());
                }
                task
            }
            None => return Err(BatchCommandServiceError::NotFound(batch_command_id)),
        };

        let mut active_batch_task: batch_command_task::ActiveModel = task_to_terminate.into();
        active_batch_task.status = Set(BatchCommandStatus::Terminating); // Or "TERMINATED" if no agent interaction confirmation is needed yet
        active_batch_task.updated_at = Set(Utc::now());
        active_batch_task.update(&txn).await?;

        // Find active child tasks and update their status
        let active_child_statuses = vec![
            ChildCommandStatus::Pending,
            ChildCommandStatus::SentToAgent,
            ChildCommandStatus::AgentAccepted,
            ChildCommandStatus::Executing,
        ];

        // Step 1: Find all active child tasks that need to be terminated.
        let child_tasks_to_terminate = ChildTaskEntity::find()
            .filter(child_command_task::Column::BatchCommandId.eq(batch_command_id))
            .filter(child_command_task::Column::Status.is_in(active_child_statuses))
            .all(&txn)
            .await?;

        if !child_tasks_to_terminate.is_empty() {
            let child_task_ids_to_update: Vec<Uuid> = child_tasks_to_terminate
                .iter()
                .map(|t| t.child_command_id)
                .collect();

            // Step 2: Update their status to Terminating.
            ChildTaskEntity::update_many()
                .col_expr(child_command_task::Column::Status, sea_orm::sea_query::Expr::value(ChildCommandStatus::Terminating))
                .col_expr(child_command_task::Column::UpdatedAt, sea_orm::sea_query::Expr::value(Utc::now()))
                .filter(child_command_task::Column::ChildCommandId.is_in(child_task_ids_to_update))
                .exec(&txn)
                .await?;
        }

        txn.commit().await?;

        // Step 3: Return the list of tasks we found in Step 1, so the caller can dispatch termination signals.
        Ok(child_tasks_to_terminate)
    }

    pub async fn terminate_single_child_task(
        &self,
        child_command_id: Uuid,
        requesting_user_id: i32,
    ) -> Result<child_command_task::Model, BatchCommandServiceError> {
        let txn = self.db.begin().await?;

        let child_task = ChildTaskEntity::find_by_id(child_command_id)
            .one(&txn)
            .await?
            .ok_or_else(|| BatchCommandServiceError::NotFound(child_command_id))?;

        let parent_batch_task = child_task.find_related(BatchTaskEntity)
            .one(&txn)
            .await?
            .ok_or_else(|| BatchCommandServiceError::NotFound(child_task.batch_command_id))?;

        if parent_batch_task.user_id != requesting_user_id {
            return Err(BatchCommandServiceError::Unauthorized);
        }

        let active_child_statuses = vec![
            ChildCommandStatus::Pending,
            ChildCommandStatus::SentToAgent,
            ChildCommandStatus::AgentAccepted,
            ChildCommandStatus::Executing,
        ];

        if !active_child_statuses.contains(&child_task.status) {
            return Err(BatchCommandServiceError::TaskNotTerminable);
        }

        let mut active_child_task: child_command_task::ActiveModel = child_task.clone().into();
        active_child_task.status = Set(ChildCommandStatus::Terminating);
        active_child_task.updated_at = Set(Utc::now());
        active_child_task.update(&txn).await?;

        txn.commit().await?;

        // Return the original model, which contains all the info needed by the caller
        Ok(child_task)
    }

    pub async fn update_child_task_status(
        &self,
        child_task_id: Uuid,
        new_status: ChildCommandStatus,
        exit_code: Option<i32>,
        error_message: Option<String>,
    ) -> Result<child_command_task::Model, BatchCommandServiceError> {
        let child_task = ChildTaskEntity::find_by_id(child_task_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| BatchCommandServiceError::NotFound(child_task_id))?; // Assuming NotFound can take child_task_id

        let mut active_child_task: child_command_task::ActiveModel = child_task.into();

        active_child_task.status = Set(new_status);
        active_child_task.updated_at = Set(Utc::now());

        if exit_code.is_some() {
            active_child_task.exit_code = Set(exit_code);
        }
        if error_message.is_some() {
            active_child_task.error_message = Set(error_message);
        }

        // Determine if the task is completed based on the new status
        match active_child_task.status.as_ref() {
            ChildCommandStatus::CompletedSuccessfully |
            ChildCommandStatus::CompletedWithFailure |
            ChildCommandStatus::Terminated |
            ChildCommandStatus::AgentUnreachable | // Consider these as final states for a child
            ChildCommandStatus::TimedOut |
            ChildCommandStatus::AgentError => {
                if active_child_task.agent_completed_at.as_ref().is_none() {
                    active_child_task.agent_completed_at = Set(Some(Utc::now()));
                }
            }
            _ => {}
        }

        let updated_task = active_child_task.update(self.db.as_ref()).await?;
        
        // Broadcast child task update
        self.result_broadcaster.broadcast_child_task_update(
            updated_task.batch_command_id,
            updated_task.child_command_id,
            updated_task.vps_id.clone(), // Assuming vps_id is String or can be cloned
            updated_task.status.to_string(),
            updated_task.exit_code,
        ).await;


        // Check if the parent batch task needs its status updated
        self.check_and_update_batch_task_status(updated_task.batch_command_id).await?;

        Ok(updated_task)
    }

    pub async fn record_child_task_output(
        &self,
        child_task_id: Uuid,
        stream_type: GrpcOutputType,
        chunk: Vec<u8>, // We'll use this later for writing
        _timestamp: Option<i64>, // Underscore for now
    ) -> Result<(), BatchCommandServiceError> {
        let child_task = ChildTaskEntity::find_by_id(child_task_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| BatchCommandServiceError::NotFound(child_task_id))?;

        let mut active_child_task: child_command_task::ActiveModel = child_task.clone().into(); // Clone for potential modification check

        let base_log_dir = PathBuf::from("logs")
            .join("batch_commands")
            .join(child_task.batch_command_id.to_string())
            .join(child_task.child_command_id.to_string());

        // Ensure the directory exists
        // Note: In a production environment, error handling for fs operations should be more robust.
        if let Err(e) = fs::create_dir_all(&base_log_dir).await {
            error!(error = %e, path = ?base_log_dir, "Failed to create log directory.");
            // Depending on policy, might return an error or proceed without log path update
            // For now, we'll proceed but log the error.
        }

        let log_file_name = match stream_type {
            GrpcOutputType::Stdout => "stdout.log",
            GrpcOutputType::Stderr => "stderr.log",
            _ => "output.log", // Default or handle unspecified
        };
        let log_file_path = base_log_dir.join(log_file_name);
        let log_file_path_str = log_file_path.to_string_lossy().into_owned();

        let mut needs_db_update = false;

        match stream_type {
            GrpcOutputType::Stdout => {
                if child_task.stdout_log_path.is_none() {
                    active_child_task.stdout_log_path = Set(Some(log_file_path_str.clone()));
                    needs_db_update = true;
                }
            }
            GrpcOutputType::Stderr => {
                if child_task.stderr_log_path.is_none() {
                    active_child_task.stderr_log_path = Set(Some(log_file_path_str.clone()));
                    needs_db_update = true;
                }
            }
            _ => {} // Do nothing for unspecified or other types for now regarding path update
        }
        // Asynchronously append `chunk` to `log_file_path_str`
        if !chunk.is_empty() {
            match fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file_path)
                .await
            {
                Ok(mut file) => {
                    if let Err(e) = tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await {
                        error!(error = %e, path = ?log_file_path, "Failed to write to log file.");
                        // Decide on error handling: return error, or just log and continue?
                        // For now, log and continue. A more robust system might retry or mark task with logging error.
                    }
                }
                Err(e) => {
                    error!(error = %e, path = ?log_file_path, "Failed to open log file.");
                }
            }
        }


        // Always update timestamps if we received output
        active_child_task.last_output_at = Set(Some(Utc::now()));
        active_child_task.updated_at = Set(Utc::now());
        needs_db_update = true; // Timestamps updated, so DB update is needed.


        if needs_db_update {
            active_child_task.update(self.db.as_ref()).await?;
        }

        // Broadcast new log output
        // Convert chunk to String - assuming UTF-8. Handle potential errors if not valid UTF-8.
        let log_line = String::from_utf8_lossy(&chunk).to_string();
        self.result_broadcaster.broadcast_new_log_output(
            child_task.batch_command_id,
            child_task.child_command_id,
            child_task.vps_id.clone(), // Assuming vps_id is String or can be cloned
            log_line,
            DisplayableGrpcOutputType(stream_type).to_string(), // Use the wrapper
            Utc::now().to_rfc3339(), // Current timestamp as string
        ).await;

        Ok(())
    }

    async fn check_and_update_batch_task_status(
        &self,
        batch_command_id: Uuid,
    ) -> Result<(), BatchCommandServiceError> {
        let child_tasks = ChildTaskEntity::find()
            .filter(child_command_task::Column::BatchCommandId.eq(batch_command_id))
            .all(self.db.as_ref())
            .await?;

        if child_tasks.is_empty() {
            // This case should ideally not happen if a batch task was created with children
            // but handle defensively.
            return Ok(());
        }

        let mut all_completed = true;
        let mut any_failed = false;
        let mut any_terminated = false; // To check if the overall status should be Terminated

        for task in &child_tasks {
            match task.status {
                ChildCommandStatus::CompletedSuccessfully => {}
                ChildCommandStatus::CompletedWithFailure | ChildCommandStatus::AgentError => {
                    any_failed = true;
                }
                ChildCommandStatus::Terminated => {
                    any_terminated = true; // If any child is terminated, the parent might reflect this
                }
                ChildCommandStatus::AgentUnreachable | ChildCommandStatus::TimedOut => {
                    any_failed = true; // Treat as failure for parent task aggregation
                }
                // If any child is still in a non-final state, the parent is not yet fully completed.
                ChildCommandStatus::Pending |
                ChildCommandStatus::SentToAgent |
                ChildCommandStatus::AgentAccepted |
                ChildCommandStatus::Executing |
                ChildCommandStatus::Terminating => {
                    all_completed = false;
                    break; // No need to check further if one is still running
                }
            }
        }

        if all_completed {
            let parent_task = BatchTaskEntity::find_by_id(batch_command_id)
                .one(self.db.as_ref())
                .await?
                .ok_or_else(|| BatchCommandServiceError::NotFound(batch_command_id))?; // Should exist

            // Avoid re-updating if already in a final state, unless it's a more specific final state
            if parent_task.status == BatchCommandStatus::CompletedSuccessfully ||
               parent_task.status == BatchCommandStatus::CompletedWithErrors ||
               parent_task.status == BatchCommandStatus::Terminated {
                return Ok(());
            }

            let mut active_parent_task: batch_command_task::ActiveModel = parent_task.into();
            
            let new_status = if any_terminated && !any_failed { // If termination was requested and no other failures
                BatchCommandStatus::Terminated
            } else if any_failed {
                BatchCommandStatus::CompletedWithErrors
            } else {
                BatchCommandStatus::CompletedSuccessfully
            };
            
            active_parent_task.status = Set(new_status);
            active_parent_task.completed_at = Set(Some(Utc::now()));
            active_parent_task.updated_at = Set(Utc::now());
            let updated_parent_task_model = active_parent_task.update(self.db.as_ref()).await?;

            // Broadcast batch task update
            self.result_broadcaster.broadcast_batch_task_update(
                batch_command_id,
                updated_parent_task_model.status.to_string(), // Use status from the updated model
                updated_parent_task_model.completed_at.map(|dt| dt.to_rfc3339()), // Use completed_at from updated model
            ).await;
        }
        // If not all completed, the parent task remains in its current state (e.g., Executing, Terminating)
        // until all children reach a final state.

        Ok(())
    }
}