// backend/src/http_server/user_routes.rs

use axum::{
    Json, Router,
    extract::{Extension, Path, State},
    response::IntoResponse,
    routing::{delete, get, put},
};
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    db::duckdb_service,
    web::{AppError, AppState, models::AuthenticatedUser},
};

pub fn create_user_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/username", put(update_username))
        .route("/password", put(update_password))
        .route("/connected-accounts", get(get_connected_accounts))
        .route("/connected-accounts/{provider}", delete(unlink_provider))
        .route("/preference", put(update_preference))
}

#[derive(Deserialize)]
pub struct UpdatePreferenceRequest {
    pub language: String,
}

async fn update_preference(
    Extension(auth_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<UpdatePreferenceRequest>,
) -> Result<impl IntoResponse, AppError> {
    duckdb_service::user_service::update_preference(
        app_state.duckdb_pool.clone(),
        auth_user.id,
        &payload.language,
    )
    .await?;
    Ok(Json(serde_json::json!({ "message": "Preference updated successfully" })))
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
    let updated_user = duckdb_service::user_service::update_username(
        app_state.duckdb_pool.clone(),
        auth_user.id,
        &payload.username,
    )
    .await?;

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
    let user_model = duckdb_service::user_service::get_user_by_id(app_state.duckdb_pool.clone(), auth_user.id)
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

    duckdb_service::user_service::update_password(
        app_state.duckdb_pool.clone(),
        auth_user.id,
        &new_password_hash,
    )
    .await?;

    Ok(Json(
        serde_json::json!({ "message": "Password updated successfully" }),
    ))
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
    let identities = duckdb_service::user_service::get_connected_accounts(
        app_state.duckdb_pool.clone(),
        auth_user.id,
    )
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
    duckdb_service::user_service::unlink_provider(
        app_state.duckdb_pool.clone(),
        auth_user.id,
        &provider,
    )
    .await?;

    Ok(Json(
        serde_json::json!({ "message": "Account unlinked successfully" }),
    ))
}
