use axum::{
    extract::{Path, State},
    routing::{get, post},
    Extension, Json, Router,
};
use sea_orm::{DatabaseConnection, EntityTrait, ActiveModelTrait, Set, ColumnTrait, QueryFilter, ModelTrait};
use std::sync::Arc;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::{
    db::entities::{theme, user},
    web::{error::AppError, models::AuthenticatedUser, AppState},
};

pub fn create_router(_db: &DatabaseConnection) -> Router<Arc<AppState>> {
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
    let db = &app_state.db_pool;
    let themes = theme::Entity::find()
        .filter(theme::Column::UserId.eq(authenticated_user.id))
        .all(db)
        .await?;
    Ok(Json(themes))
}

#[derive(Deserialize)]
pub struct CreateThemePayload {
    pub name: String,
    pub css: String,
}

async fn create_theme(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<CreateThemePayload>,
) -> Result<Json<theme::Model>, AppError> {
    let db = &app_state.db_pool;
    
    let new_theme = theme::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(authenticated_user.id),
        name: Set(payload.name),
        is_official: Set(false),
        css: Set(payload.css),
        created_at: Set(chrono::Utc::now()),
        updated_at: Set(chrono::Utc::now()),
    };

    let created_theme = new_theme.insert(db).await?;

    Ok(Json(created_theme))
}

async fn get_theme(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<Json<theme::Model>, AppError> {
    let db = &app_state.db_pool;
    let theme = theme::Entity::find_by_id(id)
        .filter(theme::Column::UserId.eq(authenticated_user.id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Theme not found".to_string()))?;

    Ok(Json(theme))
}

#[derive(Deserialize)]
pub struct UpdateThemePayload {
    pub name: String,
    pub css: String,
}

async fn update_theme(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateThemePayload>,
) -> Result<Json<theme::Model>, AppError> {
    let db = &app_state.db_pool;
    let theme_to_update = theme::Entity::find_by_id(id)
        .filter(theme::Column::UserId.eq(authenticated_user.id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Theme not found".to_string()))?;

    let mut active_model: theme::ActiveModel = theme_to_update.into();
    
    active_model.name = Set(payload.name);
    active_model.css = Set(payload.css);
    active_model.updated_at = Set(chrono::Utc::now());

    let updated_theme = active_model.update(db).await?;

    Ok(Json(updated_theme))
}

async fn delete_theme(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    let db = &app_state.db_pool;
    let theme_to_delete = theme::Entity::find_by_id(id)
        .filter(theme::Column::UserId.eq(authenticated_user.id))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound("Theme not found".to_string()))?;

    theme_to_delete.delete(db).await?;

    Ok(())
}

#[derive(Serialize)]
pub struct UserThemeSettingsDto {
    pub theme_mode: String,
    #[serde(serialize_with = "serialize_uuid_option_as_string")]
    pub active_theme_id: Option<Uuid>,
}

fn serialize_uuid_option_as_string<S>(uuid_option: &Option<Uuid>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match uuid_option {
        Some(uuid) => serializer.serialize_str(&uuid.to_string()),
        None => serializer.serialize_none(),
    }
}

async fn get_user_theme_settings(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<UserThemeSettingsDto>, AppError> {
    let db = &app_state.db_pool;
    let user = user::Entity::find_by_id(authenticated_user.id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::UserNotFound)?;

    let settings_dto = UserThemeSettingsDto {
        theme_mode: user.theme_mode,
        active_theme_id: user.active_theme_id,
    };

    Ok(Json(settings_dto))
}

#[derive(Deserialize)]
pub struct UpdateThemeSettingsPayload {
    pub theme_mode: Option<String>,
    pub active_theme_id: Option<String>,
}

async fn update_user_theme_settings(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<UpdateThemeSettingsPayload>,
) -> Result<Json<UserThemeSettingsDto>, AppError> {
    let db = &app_state.db_pool;
    let user_to_update = user::Entity::find_by_id(authenticated_user.id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::UserNotFound)?;

    let mut active_model: user::ActiveModel = user_to_update.into();

    if let Some(theme_mode) = payload.theme_mode {
        active_model.theme_mode = Set(theme_mode);
    }

    if let Some(theme_id_str) = payload.active_theme_id {
        let theme_id = Uuid::parse_str(&theme_id_str).ok();
        active_model.active_theme_id = Set(theme_id);
    }
    
    active_model.updated_at = Set(chrono::Utc::now());

    let updated_user = active_model.update(db).await?;

    let settings_dto = UserThemeSettingsDto {
        theme_mode: updated_user.theme_mode,
        active_theme_id: updated_user.active_theme_id,
    };

    Ok(Json(settings_dto))
}