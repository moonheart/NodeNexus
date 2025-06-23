// backend/src/http_server/admin_oauth_routes.rs

use std::sync::Arc;
use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post, put, delete},
    Json, Router,
};
use crate::http_server::{AppState, AppError};
use crate::db::services::oauth_service::{self, ProviderUpsertPayload};
use crate::db::entities::oauth2_provider;

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/providers", get(list_providers_handler).post(create_provider_handler))
        .route("/providers/{provider_name}", put(update_provider_handler).delete(delete_provider_handler))
}

// TODO: Add admin authentication middleware to this router.

async fn list_providers_handler(
    State(app_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let providers = oauth_service::get_all_providers_for_admin(
        &app_state.db_pool,
        &app_state.config.notification_encryption_key,
    )
    .await?;
    Ok(Json(providers))
}

async fn create_provider_handler(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<ProviderUpsertPayload>,
) -> Result<impl IntoResponse, AppError> {
    let new_provider = oauth_service::create_provider(
        &app_state.db_pool,
        payload,
        &app_state.config.notification_encryption_key,
    )
    .await?;
    Ok(Json(new_provider))
}

async fn update_provider_handler(
    State(app_state): State<Arc<AppState>>,
    Path(provider_name): Path<String>,
    Json(payload): Json<ProviderUpsertPayload>,
) -> Result<impl IntoResponse, AppError> {
    let updated_provider = oauth_service::update_provider(
        &app_state.db_pool,
        &provider_name,
        payload,
        &app_state.config.notification_encryption_key,
    )
    .await?;
    Ok(Json(updated_provider))
}

async fn delete_provider_handler(
    State(app_state): State<Arc<AppState>>,
    Path(provider_name): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    oauth_service::delete_provider(&app_state.db_pool, &provider_name).await?;
    Ok(Json(serde_json::json!({"status": "ok", "message": "Provider deleted successfully"})))
}