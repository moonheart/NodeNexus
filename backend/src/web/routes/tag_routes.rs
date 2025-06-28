use axum::{
    extract::{State, Extension, Path},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use sea_orm::{DbErr}; // Removed DeleteResult
use crate::db::{
    entities::tag,
    models::Tag as DtoTag, // Use DtoTag for the DTO
    services,
};
use crate::web::{AppState, AppError};
use crate::web::models::AuthenticatedUser;

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
) -> Result<(StatusCode, Json<DtoTag>), AppError> { // Return DtoTag
    let user_id = authenticated_user.id;
    let tag_model: tag::Model = services::create_tag(
        &app_state.db_pool,
        user_id,
        &payload.name,
        &payload.color,
        payload.icon.as_deref(),
        payload.url.as_deref(),
        payload.is_visible.unwrap_or(true),
    )
    .await
    .map_err(|db_err: DbErr| {
        match &db_err {
            DbErr::Query(sea_orm::RuntimeErr::SqlxError(sqlx_error_value)) => {
                if let sqlx::Error::Database(database_error) = sqlx_error_value {
                    if database_error.is_unique_violation() {
                        return AppError::Conflict("A tag with this name already exists.".to_string());
                    }
                }
                // If not a unique violation, or a different kind of SqlxError
                AppError::DatabaseError(sqlx_error_value.to_string())
            }
            // Add other specific DbErr cases if needed, e.g., DbErr::RecordNotInserted
            _ => AppError::DatabaseError(db_err.to_string()),
        }
    })?;

    let dto_tag = DtoTag {
        id: tag_model.id,
        user_id: tag_model.user_id,
        name: tag_model.name,
        color: tag_model.color,
        icon: tag_model.icon,
        url: tag_model.url,
        is_visible: tag_model.is_visible,
        created_at: tag_model.created_at, // Direct assignment
        updated_at: tag_model.updated_at, // Direct assignment
    };
    Ok((StatusCode::CREATED, Json(dto_tag)))
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
) -> Result<Json<DtoTag>, AppError> { // Changed return type to Json<DtoTag>
    let user_id = authenticated_user.id;
    // Assuming services::update_tag now returns Result<tag::Model, DbErr>
    let updated_tag_model: tag::Model = services::update_tag(
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
    .map_err(|db_err: DbErr| {
        match &db_err {
            DbErr::RecordNotUpdated => AppError::NotFound("Tag not found, permission denied, or no changes needed.".to_string()),
            DbErr::Query(sea_orm::RuntimeErr::SqlxError(sqlx_error_value)) => {
                if let sqlx::Error::Database(database_error) = sqlx_error_value {
                    if database_error.is_unique_violation() {
                        return AppError::Conflict("A tag with this name already exists.".to_string());
                    }
                }
                AppError::DatabaseError(sqlx_error_value.to_string())
            }
            // Add other specific DbErr cases if needed
            _ => AppError::DatabaseError(db_err.to_string()),
        }
    })?;

    let dto_tag = DtoTag {
        id: updated_tag_model.id,
        user_id: updated_tag_model.user_id,
        name: updated_tag_model.name,
        color: updated_tag_model.color,
        icon: updated_tag_model.icon,
        url: updated_tag_model.url,
        is_visible: updated_tag_model.is_visible,
        created_at: updated_tag_model.created_at, // Direct assignment
        updated_at: updated_tag_model.updated_at, // Direct assignment
    };
    Ok(Json(dto_tag))
}

async fn delete_tag_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(tag_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    let user_id = authenticated_user.id;
    // Assuming services::delete_tag now returns Result<DeleteResult, DbErr>
    let delete_result = services::delete_tag(&app_state.db_pool, tag_id, user_id)
        .await
        .map_err(|db_err| AppError::DatabaseError(db_err.to_string()))?; // Changed e to db_err

    if delete_result.rows_affected > 0 { // Changed to use delete_result.rows_affected
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