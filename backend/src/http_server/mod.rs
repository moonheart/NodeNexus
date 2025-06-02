use axum::{
    extract::State,
    http::{StatusCode, Method}, // Added Method and HeaderValue
    response::{IntoResponse, Response},
    routing::{post, get}, // Added get
    Json,
    Router,
};
use sqlx::PgPool;
use std::sync::Arc;
use std::net::SocketAddr;
use thiserror::Error;
use tower_http::cors::{CorsLayer, Any}; // Added CorsLayer and Any
use self::auth_logic::{LoginRequest, RegisterRequest};

pub mod auth_logic;
pub mod metrics_routes;

// Application state to share PgPool
#[derive(Clone)]
pub struct AppState {
    db_pool: PgPool,
}

async fn register_handler(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<self::auth_logic::UserResponse>, AppError> {
    match auth_logic::register_user(&app_state.db_pool, payload).await {
        Ok(user_response) => Ok(Json(user_response)),
        Err(e) => Err(e.into()),
    }
}

async fn login_handler(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<self::auth_logic::LoginResponse>, AppError> {
    match auth_logic::login_user(&app_state.db_pool, payload).await {
        Ok(login_response) => Ok(Json(login_response)),
        Err(e) => Err(e.into()),
    }
}


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
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::UserAlreadyExists(msg) => (StatusCode::CONFLICT, msg),
            AppError::UserNotFound => (StatusCode::UNAUTHORIZED, "无效凭据".to_string()), // Changed from NOT_FOUND
            AppError::InvalidCredentials => (StatusCode::UNAUTHORIZED, "无效凭据".to_string()), // Ensured same message
            AppError::PasswordHashingError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Password hashing error: {}", msg)),
            AppError::TokenCreationError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Token creation error: {}", msg)),
            AppError::DatabaseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", msg)),
            AppError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::ServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };
        (status, Json(serde_json::json!({ "error": error_message }))).into_response()
    }
}

// Simple health check handler
async fn health_check_handler() -> &'static str {
    "OK"
}

// Simple POST test handler
async fn login_test_handler() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(serde_json::json!({ "message": "POST test successful" })))
}

pub async fn run_http_server(db_pool: PgPool, http_addr: SocketAddr) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app_state = Arc::new(AppState { db_pool });

    // Configure CORS
    let cors = CorsLayer::new()
        // .allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap()) // Example for specific origin
        .allow_origin(Any) // Allow any origin
        .allow_methods(vec![Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS]) // Specify allowed methods
        .allow_headers(Any); // Allow any headers

    let app_router = Router::new()
        .route("/api/health", get(health_check_handler))
        .route("/login_test_simple", post(login_test_handler))
        .route("/api/auth/login_test", post(login_test_handler))
        .route("/api/auth/register", post(register_handler))
        .route("/api/auth/login", post(login_handler))
        .merge(metrics_routes::metrics_router()) // 合并指标路由
        .with_state(app_state.clone())
        .layer(cors);

    println!("HTTP server listening on {}", http_addr);
    let listener = tokio::net::TcpListener::bind(http_addr).await?;
    axum::serve(listener, app_router).await?;
    Ok(())
}