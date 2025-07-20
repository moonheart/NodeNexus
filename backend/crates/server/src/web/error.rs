use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("User already exists: {0}")]
    UserAlreadyExists(String),
    #[error("User not found")]
    UserNotFound,
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Password hashing failed: {0}")]
    PasswordHashingError(String),
    #[error("JWT creation failed: {0}")]
    TokenCreationError(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Internal server error: {0}")]
    InternalServerError(String),
    #[error("Server Error: {0}")]
    ServerError(String),
    #[error("Not Found: {0}")]
    NotFound(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Conflict: {0}")]
    Conflict(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::UserAlreadyExists(msg) => (StatusCode::CONFLICT, msg),
            AppError::UserNotFound => (StatusCode::UNAUTHORIZED, "无效凭据".to_string()),
            AppError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "无效凭据".to_string()),
            AppError::PasswordHashingError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Password hashing error: {msg}"),
            ),
            AppError::TokenCreationError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Token creation error: {msg}"),
            ),
            AppError::DatabaseError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {msg}"),
            ),
            AppError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::ServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg),
        };
        (status, Json(serde_json::json!({ "error": error_message }))).into_response()
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        AppError::DatabaseError(err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::InternalServerError(format!("JSON serialization/deserialization error: {err}"))
    }
}
