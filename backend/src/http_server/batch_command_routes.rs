use axum::{
    extract::{Extension, Path, State}, // Added Extension
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use uuid::Uuid;

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
        // .route("/:batch_command_id/tasks/:child_command_id/terminate", post(terminate_child_command)) // More granular control
        // .with_state(db) // State will be provided by the parent router
}

#[axum::debug_handler]
async fn create_batch_command(
    Extension(authenticated_user): Extension<AuthenticatedUser>, // Extract AuthenticatedUser
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<CreateBatchCommandRequest>,
) -> Result<Json<BatchCommandAcceptedResponse>, AppError> {
    let command_manager = app_state.batch_command_manager.clone();
    let dispatcher = app_state.command_dispatcher.clone();
    let user_id_str = authenticated_user.id.to_string();

    match command_manager
        .create_batch_command(user_id_str, payload.clone()) // Clone payload for later use
        .await
    {
        Ok((batch_task_model, child_tasks)) => {
            // Asynchronously dispatch commands for each child task
            for child_task in child_tasks {
                let dispatcher_clone = dispatcher.clone();
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


                    if let Err(e) = dispatcher_clone.dispatch_command_to_agent(
                        child_task.child_command_id,
                        &child_task.vps_id, // vps_id is String in ChildCommandTask model
                        &effective_command_content,
                        command_type,
                        working_directory,
                    ).await {
                        eprintln!(
                            "Failed to dispatch command for child_task_id {}: {:?}",
                            child_task.child_command_id, e
                        );
                        // Optionally, update child task status to a dispatch failure state here
                        // This might require another call to batch_command_manager.update_child_task_status
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
            eprintln!("Failed to create batch command: {:?}", e);
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
        .get_batch_command_detail_dto(batch_command_id, &authenticated_user.id.to_string())
        .await
    {
        Ok(Some(detail_response)) => Ok(Json(detail_response)),
        Ok(None) => Err(AppError::NotFound(format!(
            "Batch command task with ID {} not found.",
            batch_command_id
        ))),
        Err(service_err) => {
            eprintln!(
                "Error fetching batch command detail for ID {}: {:?}",
                batch_command_id, service_err
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
    let user_id_str = authenticated_user.id.to_string();

    match batch_manager
        .terminate_batch_command(batch_command_id, &user_id_str)
        .await
    {
        Ok(child_tasks_to_terminate) => {
            if !child_tasks_to_terminate.is_empty() {
                tokio::spawn(async move {
                    for child_task in child_tasks_to_terminate {
                        if let Err(e) = dispatcher.terminate_command_on_agent(
                            child_task.child_command_id,
                            &child_task.vps_id,
                        ).await {
                            eprintln!(
                                "Failed to dispatch terminate command for child_task_id {}: {:?}",
                                child_task.child_command_id, e
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
            eprintln!(
                "Error terminating batch command for ID {}: {:?}",
                batch_command_id, service_err
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

// async fn terminate_child_command(
//     claims: Claims,
//     State(app_state): State<Arc<AppState>>, // Use AppState
//     Path((batch_command_id, child_command_id)): Path<(Uuid, Uuid)>,
// ) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
//     // TODO: Terminate a specific child task
//     Err((axum::http::StatusCode::NOT_IMPLEMENTED, "Not yet implemented".to_string()))
// }