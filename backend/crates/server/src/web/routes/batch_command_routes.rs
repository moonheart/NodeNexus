use axum::{
    Json,
    Router,
    extract::{Extension, Path, State},
    routing::{get, post},
};
use std::sync::Arc;
use tracing::error;
use uuid::Uuid;

use crate::db::duckdb_service::batch_command_service;
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
    let detail_response = batch_command_service::get_batch_command_detail_dto(
        app_state.duckdb_pool.clone(),
        batch_command_id,
        authenticated_user.id,
    )
    .await?;

    match detail_response {
        Some(detail) => Ok(Json(detail)),
        None => Err(AppError::NotFound(format!(
            "Batch command task with ID {batch_command_id} not found."
        ))),
    }
}

#[axum::debug_handler]
async fn terminate_batch_command(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(batch_command_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    let dispatcher = app_state.command_dispatcher.clone();
    let user_id = authenticated_user.id;

    let child_tasks_to_terminate = batch_command_service::terminate_batch_command(
        app_state.duckdb_pool.clone(),
        batch_command_id,
        user_id,
    )
    .await?;

    if !child_tasks_to_terminate.is_empty() {
        tokio::spawn(async move {
            for (child_command_id, vps_id) in child_tasks_to_terminate {
                if let Err(e) = dispatcher
                    .terminate_command_on_agent(child_command_id, vps_id)
                    .await
                {
                    error!(
                        child_task_id = %child_command_id,
                        error = ?e,
                        "Failed to dispatch terminate command."
                    );
                }
            }
        });
    }

    Ok(Json(serde_json::json!({
        "message": format!("Batch command task {} marked for termination. Termination signals sent to active agents.", batch_command_id)
    })))
}

#[axum::debug_handler]
async fn terminate_child_command(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path((_batch_id, child_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let dispatcher = app_state.command_dispatcher.clone();
    let user_id = authenticated_user.id;

    let task_to_terminate = batch_command_service::terminate_single_child_task(
        app_state.duckdb_pool.clone(),
        child_id,
        user_id,
    )
    .await?;

    if let Some((child_command_id, vps_id)) = task_to_terminate {
        tokio::spawn(async move {
            if let Err(e) = dispatcher
                .terminate_command_on_agent(child_command_id, vps_id)
                .await
            {
                error!(
                    child_command_id = %child_command_id,
                    error = ?e,
                    "Failed to dispatch terminate command for child task."
                );
            }
        });
    }

    Ok(Json(serde_json::json!({
        "message": format!("Child command task {} marked for termination. Termination signal sent to agent.", child_id)
    })))
}
