use axum::{
    extract::{Extension, Path, State}, // Added Extension
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use uuid::Uuid;
use tracing::{info, error, warn};

use super::models::{
    CreateBatchCommandRequest,
    BatchCommandAcceptedResponse,
    BatchCommandTaskDetailResponse,
    // BatchCommandTaskListItem, // For listing, if implemented later
};
// use crate::http_server::auth_logic::Claims; // No longer directly extracting Claims
use crate::http_server::auth_logic::AuthenticatedUser; // Use AuthenticatedUser
use crate::http_server::{AppState, AppError}; // Import AppState and AppError
use crate::agent_service::CommandType as GrpcCommandType; // For dispatching

// pub fn batch_command_routes(db: Arc<DatabaseConnection>) -> Router {
pub fn batch_command_routes() -> Router<Arc<AppState>> { // Expects AppState
    Router::<Arc<AppState>>::new() // Explicitly type Router::new()
        .route("/", post(create_batch_command))
        .route("/{batch_command_id}", get(get_batch_command_detail))
        .route("/{batch_command_id}/terminate", post(terminate_batch_command))
        .route("/{batch_id}/tasks/{child_id}/terminate", post(terminate_child_command)) // More granular control
}

#[axum::debug_handler]
async fn create_batch_command(
    Extension(authenticated_user): Extension<AuthenticatedUser>, // Extract AuthenticatedUser
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<CreateBatchCommandRequest>,
) -> Result<Json<BatchCommandAcceptedResponse>, AppError> {
    let command_manager = app_state.batch_command_manager.clone();
    let dispatcher = app_state.command_dispatcher.clone();
    let user_id = authenticated_user.id; // No longer converting to string

    match command_manager
        .create_batch_command(user_id, payload.clone()) // Pass i32 directly
        .await
    {
        Ok((batch_task_model, child_tasks)) => {
            // Asynchronously dispatch commands for each child task
            for child_task in child_tasks {
                let dispatcher_clone = dispatcher.clone();
                let command_manager_clone = command_manager.clone(); // Clone for status updates
                let payload_clone = payload.clone(); // Clone for the spawned task
                tokio::spawn(async move {
                    let command_content = payload_clone.command_content.unwrap_or_default(); // Assuming script_id implies content is elsewhere
                    let command_type = if payload_clone.script_id.is_some() {
                        GrpcCommandType::SavedScript
                    } else {
                        GrpcCommandType::AdhocCommand
                    };
                    let working_directory = payload_clone.working_directory;

                    // TODO: Determine actual command content if script_id is used.
                    // This might involve fetching script content from DB based on script_id.
                    // For now, if script_id is present, command_content might be empty or a placeholder.
                    // The CommandDispatcher expects the actual command string.
                    // For simplicity here, we'll pass the raw command_content or an empty string if it's a script.
                    // This needs refinement based on how script_id translates to executable content.
                    let effective_command_content = if command_type == GrpcCommandType::SavedScript {
                        // Placeholder: In a real scenario, fetch script content using payload_clone.script_id
                        // For now, let's assume script_id itself or some related info is the "content"
                        // or that the agent knows how to interpret script_id.
                        // This part is crucial and needs to align with agent's capabilities.
                        // For this iteration, we'll pass the script_id as content if command_content is empty.
                        if command_content.is_empty() && payload_clone.script_id.is_some() {
                            payload_clone.script_id.unwrap_or_default()
                        } else {
                            command_content
                        }
                    } else {
                        command_content
                    };


                    let dispatch_result = dispatcher_clone.dispatch_command_to_agent(
                        child_task.child_command_id,
                        child_task.vps_id, // vps_id is String in ChildCommandTask model
                        &effective_command_content,
                        command_type,
                        working_directory,
                    ).await;

                    let (new_status, error_message) = if let Err(e) = dispatch_result {
                        error!(
                            child_task_id = %child_task.child_command_id,
                            error = ?e,
                            "Failed to dispatch command."
                        );
                        (crate::db::enums::ChildCommandStatus::AgentUnreachable, Some(e.to_string()))
                    } else {
                        (crate::db::enums::ChildCommandStatus::SentToAgent, None)
                    };

                    if let Err(update_err) = command_manager_clone.update_child_task_status(
                        child_task.child_command_id,
                        new_status,
                        None, // No exit code at this stage
                        error_message,
                    ).await {
                        error!(
                            child_task_id = %child_task.child_command_id,
                            error = ?update_err,
                            "Failed to update status after dispatch."
                        );
                    }
                });
            }

            Ok(Json(BatchCommandAcceptedResponse {
                batch_command_id: batch_task_model.batch_command_id,
                status: batch_task_model.status.to_string(),
                message: "Batch command task accepted and is being processed.".to_string(),
            }))
        }
        Err(e) => {
            error!(error = ?e, "Failed to create batch command.");
            Err(AppError::InternalServerError(format!(
                "Failed to create batch command: {}",
                e
            )))
        }
    }
}

#[axum::debug_handler]
async fn get_batch_command_detail(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(batch_command_id): Path<Uuid>,
) -> Result<Json<BatchCommandTaskDetailResponse>, AppError> {
    match app_state
        .batch_command_manager
        .get_batch_command_detail_dto(batch_command_id, authenticated_user.id)
        .await
    {
        Ok(Some(detail_response)) => Ok(Json(detail_response)),
        Ok(None) => Err(AppError::NotFound(format!(
            "Batch command task with ID {} not found.",
            batch_command_id
        ))),
        Err(service_err) => {
            error!(
                batch_command_id = %batch_command_id,
                error = ?service_err,
                "Error fetching batch command detail."
            );
            // Map BatchCommandServiceError to AppError
            match service_err {
                crate::db::services::batch_command_service::BatchCommandServiceError::Unauthorized => {
                    Err(AppError::Unauthorized("You are not authorized to view this batch command task.".to_string()))
                }
                crate::db::services::batch_command_service::BatchCommandServiceError::NotFound(_) => {
                     Err(AppError::NotFound(format!(
                        "Batch command task with ID {} not found.",
                        batch_command_id
                    )))
                }
                _ => Err(AppError::InternalServerError(format!(
                    "Failed to fetch batch command detail: {}",
                    service_err
                ))),
            }
        }
    }
}

#[axum::debug_handler]
async fn terminate_batch_command(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(batch_command_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let batch_manager = app_state.batch_command_manager.clone();
    let dispatcher = app_state.command_dispatcher.clone();
    let user_id = authenticated_user.id; // No longer converting to string

    match batch_manager
        .terminate_batch_command(batch_command_id, user_id) // Pass i32 directly
        .await
    {
        Ok(child_tasks_to_terminate) => {
            if !child_tasks_to_terminate.is_empty() {
                tokio::spawn(async move {
                    for child_task in child_tasks_to_terminate {
                        if let Err(e) = dispatcher.terminate_command_on_agent(
                            child_task.child_command_id,
                            child_task.vps_id,
                        ).await {
                            error!(
                                child_task_id = %child_task.child_command_id,
                                error = ?e,
                                "Failed to dispatch terminate command."
                            );
                            // Optionally, log this failure more permanently or alert.
                        }
                    }
                });
            }
            Ok(Json(serde_json::json!({
                "message": format!("Batch command task {} marked for termination. Termination signals sent to active agents.", batch_command_id)
            })))
        }
        Err(service_err) => {
            error!(
                batch_command_id = %batch_command_id,
                error = ?service_err,
                "Error terminating batch command."
            );
            match service_err {
                crate::db::services::batch_command_service::BatchCommandServiceError::Unauthorized => {
                    Err(AppError::Unauthorized("You are not authorized to terminate this batch command task.".to_string()))
                }
                crate::db::services::batch_command_service::BatchCommandServiceError::NotFound(_) => {
                     Err(AppError::NotFound(format!(
                        "Batch command task with ID {} not found for termination.",
                        batch_command_id
                    )))
                }
                _ => Err(AppError::InternalServerError(format!(
                    "Failed to terminate batch command: {}",
                    service_err
                ))),
            }
        }
    }
}

#[axum::debug_handler]
async fn terminate_child_command(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path((_batch_id, child_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let batch_manager = app_state.batch_command_manager.clone();
    let dispatcher = app_state.command_dispatcher.clone();
    let user_id = authenticated_user.id;

    match batch_manager
        .terminate_single_child_task(child_id, user_id)
        .await
    {
        Ok(child_task_to_terminate) => {
            // Spawn a task to send the termination signal without blocking the response
            tokio::spawn(async move {
                if let Err(e) = dispatcher.terminate_command_on_agent(
                    child_task_to_terminate.child_command_id,
                    child_task_to_terminate.vps_id,
                ).await {
                    error!(
                        child_task_id = %child_task_to_terminate.child_command_id,
                        error = ?e,
                        "Failed to dispatch terminate command for child task."
                    );
                }
            });

            Ok(Json(serde_json::json!({
                "message": format!("Child command task {} marked for termination. Termination signal sent to agent.", child_id)
            })))
        }
        Err(service_err) => {
            error!(
                child_command_id = %child_id,
                error = ?service_err,
                "Error terminating child command."
            );
            match service_err {
                crate::db::services::batch_command_service::BatchCommandServiceError::Unauthorized => {
                    Err(AppError::Unauthorized("You are not authorized to terminate this task.".to_string()))
                }
                crate::db::services::batch_command_service::BatchCommandServiceError::NotFound(_) => {
                     Err(AppError::NotFound(format!(
                        "Child command task with ID {} not found.",
                        child_id
                    )))
                }
                crate::db::services::batch_command_service::BatchCommandServiceError::TaskNotTerminable => {
                    Err(AppError::Conflict("Task is already completed or in a non-terminable state.".to_string()))
                }
                _ => Err(AppError::InternalServerError(format!(
                    "Failed to terminate child command: {}",
                    service_err
                ))),
            }
        }
    }
}