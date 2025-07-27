use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    routing::{get, post},
};
use serde::Deserialize;
use std::sync::Arc;

use crate::db::duckdb_service::command_script_service;
use crate::db::entities::command_script::ScriptLanguage;
use crate::web::models::AuthenticatedUser;
use crate::web::{AppError, AppState};

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
        .route(
            "/{id}",
            get(get_script).put(update_script).delete(delete_script),
        )
}

async fn create_script(
    State(app_state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Json(payload): Json<ScriptPayload>,
) -> Result<Json<command_script_service::CommandScript>, AppError> {
    let script = command_script_service::create_script(
        app_state.duckdb_pool.clone(),
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
) -> Result<Json<Vec<command_script_service::CommandScript>>, AppError> {
    let scripts = command_script_service::get_scripts_by_user(app_state.duckdb_pool.clone(), user.id).await?;
    Ok(Json(scripts))
}

async fn get_script(
    State(app_state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
) -> Result<Json<command_script_service::CommandScript>, AppError> {
    let script = command_script_service::get_script_by_id(app_state.duckdb_pool.clone(), id, user.id).await?;
    Ok(Json(script))
}

async fn update_script(
    State(app_state): State<Arc<AppState>>,
    Extension(user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
    Json(payload): Json<ScriptPayload>,
) -> Result<Json<command_script_service::CommandScript>, AppError> {
    let script = command_script_service::update_script(
        app_state.duckdb_pool.clone(),
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
    command_script_service::delete_script(app_state.duckdb_pool.clone(), id, user.id).await?;
    Ok(Json(()))
}

