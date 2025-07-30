use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use serde_json::json;

use crate::{
    db::duckdb_service::theme_service,
    db::entities::theme,
    web::{error::AppError, models::AuthenticatedUser, AppState},
};

#[derive(Deserialize)]
pub struct CreateThemePayload {
    pub name: String,
    pub css: String,
}

#[derive(Deserialize)]
pub struct UpdateThemePayload {
    pub name: Option<String>,
    pub css: Option<String>,
}

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/themes", get(list_themes).post(create_theme))
        .route(
            "/themes/{id}",
            get(get_theme).put(update_theme).delete(delete_theme),
        )
        .route(
            "/user/theme-settings",
            get(get_user_theme_settings).put(update_user_theme_settings),
        )
}

async fn list_themes(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<theme::Model>>, AppError> {
    let themes =
        theme_service::get_themes_for_user(app_state.duckdb_pool.clone(), authenticated_user.id)
            .await?;
    Ok(Json(themes))
}

async fn create_theme(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<CreateThemePayload>,
) -> Result<impl IntoResponse, AppError> {
    let new_theme = theme_service::create_theme(
        app_state.duckdb_pool.clone(),
        authenticated_user.id,
        payload.name,
        payload.css,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(new_theme)))
}

async fn get_theme(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<theme::Model>, AppError> {
    let theme = theme_service::get_theme_by_id(app_state.duckdb_pool.clone(), id, authenticated_user.id)
        .await?
        .ok_or_else(|| AppError::NotFound("Theme not found".to_string()))?;
    Ok(Json(theme))
}

async fn update_theme(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateThemePayload>,
) -> Result<Json<theme::Model>, AppError> {
    let updated_theme = theme_service::update_theme(
        app_state.duckdb_pool.clone(),
        id,
        authenticated_user.id,
        payload.name,
        payload.css,
    )
    .await?;
    Ok(Json(updated_theme))
}

async fn delete_theme(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    theme_service::delete_theme(app_state.duckdb_pool.clone(), id, authenticated_user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize, Default)]
pub struct UserThemeSettingsDto {
    pub theme_mode: String,
    pub active_theme_id: Option<String>,
    pub background_image_url: Option<String>,
}

async fn get_user_theme_settings(
    State(app_state): State<Arc<AppState>>,
    _authenticated_user: Extension<AuthenticatedUser>,
) -> Result<Json<UserThemeSettingsDto>, AppError> {
    let mut settings_dto = UserThemeSettingsDto::default();
    settings_dto.theme_mode = "system".to_string(); // Default value

    if let Some(setting) = crate::db::duckdb_service::settings_service::get_setting(app_state.duckdb_pool.clone(), "theme_mode").await? {
        if let Some(val) = setting.value.as_str() {
            settings_dto.theme_mode = val.to_string();
        }
    }
    if let Some(setting) = crate::db::duckdb_service::settings_service::get_setting(app_state.duckdb_pool.clone(), "active_theme_id").await? {
        settings_dto.active_theme_id = setting.value.as_str().map(String::from);
    }
    if let Some(setting) = crate::db::duckdb_service::settings_service::get_setting(app_state.duckdb_pool.clone(), "background_image_url").await? {
        settings_dto.background_image_url = setting.value.as_str().map(String::from);
    }

    Ok(Json(settings_dto))
}

#[derive(Deserialize)]
pub struct UpdateThemeSettingsPayload {
    pub theme_mode: Option<String>,
    pub active_theme_id: Option<String>,
    pub background_image_url: Option<String>,
}

async fn update_user_theme_settings(
    State(app_state): State<Arc<AppState>>,
    _authenticated_user: Extension<AuthenticatedUser>,
    Json(payload): Json<UpdateThemeSettingsPayload>,
) -> Result<Json<()>, AppError> {
    if let Some(theme_mode) = payload.theme_mode {
        crate::db::duckdb_service::settings_service::update_setting(app_state.duckdb_pool.clone(), "theme_mode", &json!(theme_mode)).await?;
    }

    if let Some(active_theme_id) = payload.active_theme_id {
        crate::db::duckdb_service::settings_service::update_setting(app_state.duckdb_pool.clone(), "active_theme_id", &json!(active_theme_id)).await?;
    }

    if let Some(background_image_url) = payload.background_image_url {
        crate::db::duckdb_service::settings_service::update_setting(app_state.duckdb_pool.clone(), "background_image_url", &json!(background_image_url)).await?;
    }

    Ok(Json(()))
}