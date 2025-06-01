use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Json,
    Router,
};
use sqlx::PgPool;
use std::sync::Arc;
use std::net::SocketAddr;
use auth_logic::{AuthError, LoginRequest, RegisterRequest};

pub mod auth_logic;

// Application state to share PgPool
#[derive(Clone)]
pub struct AppState {
    db_pool: PgPool,
}

async fn register_handler(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<auth_logic::UserResponse>, AppError> {
    match auth_logic::register_user(&app_state.db_pool, payload).await {
        Ok(user_response) => Ok(Json(user_response)),
        Err(e) => Err(e.into()),
    }
}

async fn login_handler(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<auth_logic::LoginResponse>, AppError> {
    match auth_logic::login_user(&app_state.db_pool, payload).await {
        Ok(login_response) => Ok(Json(login_response)),
        Err(e) => Err(e.into()),
    }
}

// Custom error type for Axum handlers
struct AppError(AuthError);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self.0 {
            AuthError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg),
            AuthError::UserAlreadyExists(msg) => (StatusCode::CONFLICT, msg),
            AuthError::UserNotFound => (StatusCode::NOT_FOUND, "User not found".to_string()),
            AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()),
            AuthError::PasswordHashingError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Password hashing error: {}", msg)),
            AuthError::TokenCreationError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Token creation error: {}", msg)),
            AuthError::DatabaseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", msg)),
            AuthError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        (status, Json(serde_json::json!({ "error": error_message }))).into_response()
    }
}

impl From<AuthError> for AppError {
    fn from(err: AuthError) -> Self {
        AppError(err)
    }
}

pub async fn run_http_server(db_pool: PgPool, http_addr: SocketAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app_state = Arc::new(AppState { db_pool });

    let app_router = Router::new()
        .route("/api/auth/register", post(register_handler))
        .route("/api/auth/login", post(login_handler))
        .with_state(app_state.clone());

    println!("HTTP server listening on {}", http_addr);
    let listener = tokio::net::TcpListener::bind(http_addr).await?;
    axum::serve(listener, app_router).await?;
    Ok(())
}