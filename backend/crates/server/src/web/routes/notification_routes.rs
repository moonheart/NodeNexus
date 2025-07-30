use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use std::sync::Arc;

use crate::{
    db::duckdb_service,
    notifications::models::{
        ChannelTemplate, ChannelTemplateField, CreateChannelRequest, TestChannelRequest,
        UpdateChannelRequest,
    },
    web::{AppError, AppState, models::AuthenticatedUser},
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
) -> Result<Json<Vec<crate::notifications::models::ChannelTemplate>>, AppError> {
    // This data was static in the original service, so we can replicate it here.
    let templates = vec![
        ChannelTemplate {
            channel_type: "telegram".to_string(),
            name: "Telegram".to_string(),
            fields: vec![
                ChannelTemplateField {
                    name: "bot_token".to_string(),
                    field_type: "password".to_string(),
                    required: true,
                    label: "Bot Token".to_string(),
                    help_text: Some("Your Telegram Bot Token.".to_string()),
                },
                ChannelTemplateField {
                    name: "chat_id".to_string(),
                    field_type: "text".to_string(),
                    required: true,
                    label: "Chat ID".to_string(),
                    help_text: Some(
                        "The target chat ID (user, group, or channel).".to_string(),
                    ),
                },
            ],
        },
        ChannelTemplate {
            channel_type: "webhook".to_string(),
            name: "Custom Webhook".to_string(),
            fields: vec![
                ChannelTemplateField {
                    name: "url".to_string(),
                    field_type: "text".to_string(),
                    required: true,
                    label: "Webhook URL".to_string(),
                    help_text: None,
                },
                ChannelTemplateField {
                    name: "method".to_string(),
                    field_type: "text".to_string(),
                    required: true,
                    label: "HTTP Method".to_string(),
                    help_text: Some("Usually POST or GET.".to_string()),
                },
                ChannelTemplateField {
                    name: "headers".to_string(),
                    field_type: "textarea".to_string(),
                    required: false,
                    label: "Headers (JSON)".to_string(),
                    help_text: Some(
                        "A JSON object of key-value pairs for headers.".to_string(),
                    ),
                },
                ChannelTemplateField {
                    name: "body_template".to_string(),
                    field_type: "textarea".to_string(),
                    required: false,
                    label: "Body Template (for POST)".to_string(),
                    help_text: Some(
                        "JSON body with Tera template variables like {{ vps_name }}."
                            .to_string(),
                    ),
                },
            ],
        },
    ];
    Ok(Json(templates))
}

// Handler to create a new notification channel
async fn create_channel(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<CreateChannelRequest>,
) -> Result<impl IntoResponse, AppError> {
    let channel = duckdb_service::notification_service::create_channel(
        app_state.duckdb_pool.clone(),
        app_state.encryption_service.clone(),
        authenticated_user.id,
        payload,
    )
    .await?;
    Ok((StatusCode::CREATED, Json(channel)))
}

// Handler to get all of a user's configured channels
async fn get_all_channels(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<crate::notifications::models::ChannelResponse>>, AppError> {
    let channels = duckdb_service::notification_service::get_all_channels_for_user(
        app_state.duckdb_pool.clone(),
        app_state.encryption_service.clone(),
        authenticated_user.id,
    )
    .await?;
    Ok(Json(channels))
}

// Handler to get a single channel by its ID
async fn get_channel_by_id(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
) -> Result<Json<crate::notifications::models::ChannelResponse>, AppError> {
    let channel = duckdb_service::notification_service::get_channel_by_id(
        app_state.duckdb_pool.clone(),
        app_state.encryption_service.clone(),
        authenticated_user.id,
        id,
    )
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
    let updated_channel = duckdb_service::notification_service::update_channel(
        app_state.duckdb_pool.clone(),
        app_state.encryption_service.clone(),
        authenticated_user.id,
        id,
        payload,
    )
    .await?;
    Ok(Json(updated_channel))
}

// Handler to delete a channel
async fn delete_channel(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
) -> Result<StatusCode, AppError> {
    duckdb_service::notification_service::delete_channel(
        app_state.duckdb_pool.clone(),
        authenticated_user.id,
        id,
    )
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
    duckdb_service::notification_service::send_test_notification(
        app_state.duckdb_pool.clone(),
        app_state.encryption_service.clone(),
        authenticated_user.id,
        id,
        payload.message.unwrap_or_else(|| "This is a test message from your monitoring system.".to_string()),
    )
    .await?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({"message": "Test notification sent successfully."})),
    ))
}
