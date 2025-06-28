use axum::{
    Json,
    Router,
    extract::{Extension, Path, State},
    routing::{get, post},
};
use std::sync::Arc;
use tracing::error;
use uuid::Uuid;

use crate::web::handlers::batch_command_upgrade_handler::batch_command_upgrade_handler;
use crate::web::models::batch_command_models::BatchCommandTaskDetailResponse;
use crate::web::models::AuthenticatedUser;
use crate::web::{AppState, error::AppError};

pub fn batch_command_routes() -> Router<Arc<AppState>> {
    Router::<Arc<AppState>>::new()
        .route("/", get(batch_command_upgrade_handler)) // Changed to GET for WebSocket upgrade
        .route("/{batch_command_id}", get(get_batch_command_detail))
        .route(
            "/{batch_command_id}/terminate",
            post(terminate_batch_command),
        )
        .route(
            "/{batch_id}/tasks/{child_id}/terminate",
            post(terminate_child_command),
        ) // More granular control
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
            "Batch command task with ID {batch_command_id} not found."
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
                        "Batch command task with ID {batch_command_id} not found."
                    )))
                }
                _ => Err(AppError::InternalServerError(format!(
                    "Failed to fetch batch command detail: {service_err}"
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
                        if let Err(e) = dispatcher
                            .terminate_command_on_agent(
                                child_task.child_command_id,
                                child_task.vps_id,
                            )
                            .await
                        {
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
                        "Batch command task with ID {batch_command_id} not found for termination."
                    )))
                }
                _ => Err(AppError::InternalServerError(format!(
                    "Failed to terminate batch command: {service_err}"
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
                if let Err(e) = dispatcher
                    .terminate_command_on_agent(
                        child_task_to_terminate.child_command_id,
                        child_task_to_terminate.vps_id,
                    )
                    .await
                {
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
                        "Child command task with ID {child_id} not found."
                    )))
                }
                crate::db::services::batch_command_service::BatchCommandServiceError::TaskNotTerminable => {
                    Err(AppError::Conflict("Task is already completed or in a non-terminable state.".to_string()))
                }
                _ => Err(AppError::InternalServerError(format!(
                    "Failed to terminate child command: {service_err}"
                ))),
            }
        }
    }
}
