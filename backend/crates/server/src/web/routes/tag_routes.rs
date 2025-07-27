use crate::db::{
    duckdb_service::tag_service as duckdb_tag_service,
    entities::tag,
};
use crate::server::update_service;
use crate::web::models::AuthenticatedUser;
use crate::web::{AppError, AppState};
use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

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
) -> Result<(StatusCode, Json<tag::Model>), AppError> {
    let user_id = authenticated_user.id;
    let tag_model = duckdb_tag_service::create_tag(
        app_state.duckdb_pool.clone(),
        user_id,
        &payload.name,
        &payload.color,
        payload.icon.as_deref(),
        payload.url.as_deref(),
        payload.is_visible.unwrap_or(true),
    )
    .await?;

    Ok((StatusCode::CREATED, Json(tag_model)))
}

async fn get_user_tags_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Vec<duckdb_tag_service::TagWithCount>>, AppError> {
    let user_id = authenticated_user.id;
    let tags = duckdb_tag_service::get_tags_by_user_id_with_count(app_state.duckdb_pool.clone(), user_id).await?;
    Ok(Json(tags))
}

async fn update_tag_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(tag_id): Path<i32>,
    Json(payload): Json<UpdateTagRequest>,
) -> Result<Json<tag::Model>, AppError> {
    let user_id = authenticated_user.id;
    let updated_tag_model = duckdb_tag_service::update_tag(
        app_state.duckdb_pool.clone(),
        tag_id,
        user_id,
        &payload.name,
        &payload.color,
        payload.icon.as_deref(),
        payload.url.as_deref(),
        payload.is_visible,
    )
    .await?;

    update_service::broadcast_full_state_update(
        app_state.duckdb_pool.clone(),
        &app_state.live_server_data_cache,
        &app_state.ws_data_broadcaster_tx,
    )
    .await;

    Ok(Json(updated_tag_model))
}

async fn delete_tag_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(tag_id): Path<i32>,
) -> Result<StatusCode, AppError> {
    let user_id = authenticated_user.id;
    let rows_affected = duckdb_tag_service::delete_tag(app_state.duckdb_pool.clone(), tag_id, user_id).await?;

    if rows_affected > 0 {
        update_service::broadcast_full_state_update(
            app_state.duckdb_pool.clone(),
            &app_state.live_server_data_cache,
            &app_state.ws_data_broadcaster_tx,
        )
        .await;
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(
            "Tag not found or permission denied".to_string(),
        ))
    }
}

// --- Router ---

pub fn create_tags_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_user_tags_handler).post(create_tag_handler))
        .route(
            "/{tag_id}",
            put(update_tag_handler).delete(delete_tag_handler),
        )
}
