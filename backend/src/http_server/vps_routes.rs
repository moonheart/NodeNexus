use axum::{
    extract::{State, Extension},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post}, // Added get
    Json, Router,
};
use serde::{Deserialize, Serialize}; // Serialize might be needed for Vps if not already
use std::sync::Arc;
use crate::db::{models::Vps, services};
use super::{AppState, AppError}; // Assuming AppError and AppState are in super module (http_server/mod.rs)
use crate::http_server::auth_logic::AuthenticatedUser; // Assuming AuthenticatedUser for getting user_id

#[derive(Deserialize)]
pub struct CreateVpsRequest {
    name: String,
}

// The response will be the Vps model itself, which includes id and agent_secret
// Ensure Vps model in db/models.rs is Serialize
// type CreateVpsResponse = Vps; // This is implicit if Vps implements Serialize

async fn create_vps_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>, // Get user_id from JWT
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<CreateVpsRequest>,
) -> Result<(StatusCode, Json<Vps>), AppError> {
    // user_id is now available from authenticated_user.id
    let user_id = authenticated_user.id;

    match services::create_vps(&app_state.db_pool, user_id, &payload.name).await {
        Ok(vps) => Ok((StatusCode::CREATED, Json(vps))),
        Err(sqlx_error) => {
            eprintln!("Failed to create VPS: {:?}", sqlx_error);
            // Consider more specific error mapping if needed
            Err(AppError::DatabaseError(sqlx_error.to_string()))
        }
    }
}

async fn get_all_vps_handler(
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Vec<Vps>>, AppError> {
    let user_id = authenticated_user.id;
    match services::get_all_vps_for_user(&app_state.db_pool, user_id).await {
        Ok(vps_list) => Ok(Json(vps_list)),
        Err(sqlx_error) => {
            eprintln!("Failed to retrieve VPS list for user {}: {:?}", user_id, sqlx_error);
            Err(AppError::DatabaseError(sqlx_error.to_string()))
        }
    }
}

pub fn vps_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_vps_handler))
        .route("/", get(get_all_vps_handler)) // Added GET route
}