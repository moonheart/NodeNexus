use axum::{
    extract::{Extension, Path, State},
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;
use serde::Deserialize;

use crate::db::entities::command_script::{self, ScriptLanguage};
use crate::db::services::command_script_service::{CommandScriptService, CommandScriptError};
use crate::web::{AppState, AppError};
use crate::web::models::AuthenticatedUser;

#[derive(Deserialize)]
pub struct ScriptPayload {
    pub name: String,
    pub description: Option<String>,
    pub language: ScriptLanguage,
    pub script_content: String,
    pub working_directory: String,
}

pub fn command_script_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_script).get(list_scripts))
        .route("/{id}", get(get_script).put(update_script).delete(delete_script))
}

async fn create_script(
    State(app_state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<ScriptPayload>,
) -> Result<Json<command_script::Model>, AppError> {
    let script = CommandScriptService::create_script(
        &app_state.db_pool,
        user.id,
        payload.name,
        payload.description,
        payload.language,
        payload.script_content,
        payload.working_directory,
    )
    .await?;
    Ok(Json(script))
}

async fn list_scripts(
    State(app_state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<command_script::Model>>, AppError> {
    let scripts = CommandScriptService::get_scripts_by_user(&app_state.db_pool, user.id).await?;
    Ok(Json(scripts))
}

async fn get_script(
    State(app_state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
) -> Result<Json<command_script::Model>, AppError> {
    let script = CommandScriptService::get_script_by_id(&app_state.db_pool, id, user.id).await?;
    Ok(Json(script))
}

async fn update_script(
    State(app_state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
    Json(payload): Json<ScriptPayload>,
) -> Result<Json<command_script::Model>, AppError> {
    let script = CommandScriptService::update_script(
        &app_state.db_pool,
        id,
        user.id,
        payload.name,
        payload.description,
        payload.language,
        payload.script_content,
        payload.working_directory,
    )
    .await?;
    Ok(Json(script))
}

async fn delete_script(
    State(app_state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
) -> Result<Json<()>, AppError> {
    CommandScriptService::delete_script(&app_state.db_pool, id, user.id).await?;
    Ok(Json(()))
}

// Implement the From trait to convert CommandScriptError to AppError
impl From<CommandScriptError> for AppError {
    fn from(err: CommandScriptError) -> Self {
        match err {
            CommandScriptError::DbErr(e) => AppError::InternalServerError(e.to_string()),
            CommandScriptError::NotFound(id) => AppError::NotFound(format!("Script with ID {} not found", id)),
            CommandScriptError::Unauthorized => AppError::Unauthorized("You are not authorized to perform this action.".to_string()),
            CommandScriptError::DuplicateName(name) => AppError::Conflict(format!("A script with the name '{}' already exists.", name)),
        }
    }
}