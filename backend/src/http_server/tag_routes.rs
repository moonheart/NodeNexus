use axum::{
    extract::{State, Extension, Path},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use crate::db::{services, models::Tag};
use super::{AppState, AppError};
use crate::http_server::auth_logic::AuthenticatedUser;

// --- Request/Response Structs ---

#[derive(Deserialize)]
pub struct CreateTagRequest {
    name: String,
    color: String,
    icon: Option<String>,
    url: Option<String>,
    is_visible: Option<bool>,
}

#[derive(Deserialize)]
pub struct UpdateTagRequest {
    name: String,
    color: String,
    icon: Option<String>,
    url: Option<String>,
    is_visible: bool,
}

// --- Route Handlers ---

async fn create_tag_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<CreateTagRequest>,
) -> Result<(StatusCode, Json<Tag>), AppError> {
    let user_id = authenticated_user.id;
    let tag = services::create_tag(
        &app_state.db_pool,
        user_id,
        &payload.name,
        &payload.color,
        payload.icon.as_deref(),
        payload.url.as_deref(),
        payload.is_visible.unwrap_or(true),
    )
    .await
    .map_err(|e| {
        // Handle unique constraint violation specifically
        if let Some(db_err) = e.as_database_error() {
            if db_err.is_unique_violation() {
                return AppError::Conflict("A tag with this name already exists.".to_string());
            }
        }
        AppError::DatabaseError(e.to_string())
    })?;
    Ok((StatusCode::CREATED, Json(tag)))
}

async fn get_user_tags_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Vec<services::TagWithCount>>, AppError> {
    let user_id = authenticated_user.id;
    let tags = services::get_tags_by_user_id_with_count(&app_state.db_pool, user_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
    Ok(Json(tags))
}

async fn update_tag_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(tag_id): Path<i32>,
    Json(payload): Json<UpdateTagRequest>,
) -> Result<StatusCode, AppError> {
    let user_id = authenticated_user.id;
    let rows_affected = services::update_tag(
        &app_state.db_pool,
        tag_id,
        user_id,
        &payload.name,
        &payload.color,
        payload.icon.as_deref(),
        payload.url.as_deref(),
        payload.is_visible,
    )
    .await
    .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if rows_affected > 0 {
        Ok(StatusCode::OK)
    } else {
        Err(AppError::NotFound("Tag not found or permission denied".to_string()))
    }
}

async fn delete_tag_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(tag_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    let user_id = authenticated_user.id;
    let rows_affected = services::delete_tag(&app_state.db_pool, tag_id, user_id)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    if rows_affected > 0 {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound("Tag not found or permission denied".to_string()))
    }
}

// --- Router ---

pub fn create_tags_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_user_tags_handler).post(create_tag_handler))
        .route("/{tag_id}", put(update_tag_handler).delete(delete_tag_handler))
}