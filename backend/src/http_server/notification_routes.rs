use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Extension, Json, Router,
};
use std::sync::Arc;

use crate::http_server::{AppError, AppState, auth_logic::AuthenticatedUser}; // Import AuthenticatedUser
use crate::notifications::{
    models::{CreateChannelRequest, TestChannelRequest, UpdateChannelRequest},
    service::NotificationError,
};

pub fn create_notification_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/channels", get(get_all_channels).post(create_channel))
        .route("/channels/templates", get(get_channel_templates))
        .route(
            "/channels/{id}",
            get(get_channel_by_id)
                .put(update_channel)
                .delete(delete_channel),
        )
        .route("/channels/{id}/test", post(test_channel))
}

// Handler to get all available channel templates
async fn get_channel_templates(
    State(app_state): State<Arc<AppState>>,
) -> Result<Json<Vec<crate::notifications::models::ChannelTemplate>>, AppError> {
    let templates = app_state.notification_service.get_channel_templates();
    Ok(Json(templates))
}

// Handler to create a new notification channel
async fn create_channel(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<CreateChannelRequest>,
) -> Result<impl IntoResponse, AppError> {
    let channel = app_state
        .notification_service
        .create_channel(authenticated_user.id, payload)
        .await?;
    Ok((StatusCode::CREATED, Json(channel)))
}

// Handler to get all of a user's configured channels
async fn get_all_channels(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<crate::notifications::models::ChannelResponse>>, AppError> {
    let channels = app_state
        .notification_service
        .get_all_channels_for_user(authenticated_user.id)
        .await?;
    Ok(Json(channels))
}

// Handler to get a single channel by its ID
async fn get_channel_by_id(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
) -> Result<Json<crate::notifications::models::ChannelResponse>, AppError> {
    let channel = app_state
        .notification_service
        .get_channel_by_id(authenticated_user.id, id)
        .await?;
    Ok(Json(channel))
}

// Handler to update a channel
async fn update_channel(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateChannelRequest>,
) -> Result<Json<crate::notifications::models::ChannelResponse>, AppError> {
    let updated_channel = app_state
        .notification_service
        .update_channel(authenticated_user.id, id, payload)
        .await?;
    Ok(Json(updated_channel))
}

// Handler to delete a channel
async fn delete_channel(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
) -> Result<StatusCode, AppError> {
    app_state
        .notification_service
        .delete_channel(authenticated_user.id, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// Handler to send a test message to a channel
async fn test_channel(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
    Json(payload): Json<TestChannelRequest>,
) -> Result<impl IntoResponse, AppError> {
    app_state
        .notification_service
        .test_channel(authenticated_user.id, id, payload.message)
        .await?;
    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"message": "Test message sent successfully."})),
    ))
}

// Implement the conversion from NotificationError to AppError
impl From<NotificationError> for AppError {
    fn from(err: NotificationError) -> Self {
        match err {
            NotificationError::DatabaseError(e) => AppError::DatabaseError(e.to_string()),
            NotificationError::EncryptionError(e) => {
                AppError::InternalServerError(format!("Encryption error: {}", e))
            }
            NotificationError::SerializationError(e) => {
                AppError::InvalidInput(format!("Failed to process configuration: {}", e))
            }
            NotificationError::NotFound(_) => AppError::NotFound("Notification channel not found.".to_string()),
            NotificationError::UnsupportedChannel(channel_type) => {
                AppError::InvalidInput(format!("Unsupported channel type: {}", channel_type))
            }
            NotificationError::SenderError(e) => {
                AppError::InternalServerError(format!("Failed to send notification: {}", e))
            }
            NotificationError::PermissionDenied => {
                AppError::Unauthorized("You do not have permission to access this resource.".to_string())
            }
            NotificationError::Generic(s) => AppError::InternalServerError(s),
        }
    }
}