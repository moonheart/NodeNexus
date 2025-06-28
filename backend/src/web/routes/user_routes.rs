// backend/src/http_server/user_routes.rs

use axum::{
    extract::{Extension, State, Path},
    response::IntoResponse,
    routing::{get, put, delete},
    Json, Router,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    db::entities::{user, user_identity_provider},
    web::{models::AuthenticatedUser, AppError, AppState},
};

pub fn create_user_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/username", put(update_username))
        .route("/password", put(update_password))
        .route("/connected-accounts", get(get_connected_accounts))
        .route("/connected-accounts/{provider}", delete(unlink_provider))
}

#[derive(Deserialize)]
pub struct UpdateUsernameRequest {
    pub username: String,
}

async fn update_username(
    Extension(auth_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<UpdateUsernameRequest>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Add validation (e.g., check if username is already taken)
    let mut user: user::ActiveModel = user::Entity::find_by_id(auth_user.id)
        .one(&app_state.db_pool)
        .await?
        .ok_or(AppError::UserNotFound)?
        .into();

    user.username = Set(payload.username);
    let updated_user = user.update(&app_state.db_pool).await?;

    Ok(Json(serde_json::json!({
        "id": updated_user.id,
        "username": updated_user.username
    })))
}

#[derive(Deserialize)]
pub struct UpdatePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

async fn update_password(
    Extension(auth_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<UpdatePasswordRequest>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Implement password update logic
    let user_model = user::Entity::find_by_id(auth_user.id)
        .one(&app_state.db_pool)
        .await?
        .ok_or(AppError::UserNotFound)?;

    let password_hash = user_model.password_hash.as_ref().ok_or_else(|| {
        AppError::InvalidInput("This account does not have a password set.".to_string())
    })?;

    let valid_password = bcrypt::verify(&payload.current_password, password_hash)
        .map_err(|_| AppError::InternalServerError("Password verification failed".to_string()))?;

    if !valid_password {
        return Err(AppError::InvalidCredentials);
    }

    if payload.new_password.len() < 8 {
        return Err(AppError::InvalidInput(
            "Password must be at least 8 characters long.".to_string(),
        ));
    }

    let new_password_hash = bcrypt::hash(&payload.new_password, bcrypt::DEFAULT_COST)
        .map_err(|_| AppError::InternalServerError("Failed to hash new password".to_string()))?;

    let mut user_active_model: user::ActiveModel = user_model.into();
    user_active_model.password_hash = Set(Some(new_password_hash));
    user_active_model.update(&app_state.db_pool).await?;

    Ok(Json(serde_json::json!({ "message": "Password updated successfully" })))
}

#[derive(Serialize)]
pub struct ConnectedAccountResponse {
    provider_name: String,
    provider_user_id: String, // This is the ID from the provider, not our internal user ID
}

async fn get_connected_accounts(
    Extension(auth_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let identities = user_identity_provider::Entity::find()
        .filter(user_identity_provider::Column::UserId.eq(auth_user.id))
        .all(&app_state.db_pool)
        .await?;

    let response: Vec<ConnectedAccountResponse> = identities
        .into_iter()
        .map(|identity| ConnectedAccountResponse {
            provider_name: identity.provider_name,
            provider_user_id: identity.provider_user_id,
        })
        .collect();

    Ok(Json(response))
}

async fn unlink_provider(
    Extension(auth_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Path(provider): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    // TODO: Add logic to prevent unlinking the last/only sign-in method if no password is set.
    user_identity_provider::Entity::delete_many()
        .filter(user_identity_provider::Column::UserId.eq(auth_user.id))
        .filter(user_identity_provider::Column::ProviderName.eq(provider))
        .exec(&app_state.db_pool)
        .await?;

    Ok(Json(serde_json::json!({ "message": "Account unlinked successfully" })))
}