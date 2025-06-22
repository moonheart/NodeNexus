use axum::{
    extract::State,
    middleware, // Added for from_fn
    http::{StatusCode, Method}, // Added Method and HeaderValue
    response::{IntoResponse, Response},
    routing::{post, get}, // Added get
    Json,
    Router,
};
use sea_orm::DatabaseConnection; // Replaced PgPool
use std::sync::Arc;
 // Added for LiveServerDataCache
use tokio::sync::{broadcast, mpsc, Mutex}; // Added Mutex and mpsc
use thiserror::Error;
use crate::server::agent_state::{ConnectedAgents, LiveServerDataCache};
use crate::websocket_models::WsMessage;
use tower_http::cors::{CorsLayer, Any}; // Added CorsLayer and Any
use self::auth_logic::{LoginRequest, RegisterRequest};
use crate::notifications::service::NotificationService;
use crate::db::services::{AlertService, BatchCommandManager}; // Added BatchCommandManager
use crate::server::command_dispatcher::CommandDispatcher; // Added CommandDispatcher
use crate::server::result_broadcaster::{ResultBroadcaster, BatchCommandUpdateMsg}; // Added ResultBroadcaster
use rust_embed::RustEmbed;
use crate::axum_embed::{ServeEmbed, FallbackBehavior};

pub fn create_static_file_service() -> ServeEmbed<Assets> {
    ServeEmbed::<Assets>::with_parameters(
        Some("index.html".to_string()),
        FallbackBehavior::Ok,
        Some("index.html".to_string()),
    )
}

pub mod auth_logic;
pub mod metrics_routes;
pub mod vps_routes; // Added VPS routes module
pub mod websocket_handler; // Added WebSocket handler module
pub mod config_routes;
pub mod tag_routes;
pub mod notification_routes;
pub mod models; // Added models module
pub mod alert_routes; // Added alert_routes module
pub mod batch_command_routes;
pub mod ws_batch_command_handler; // Added WebSocket handler for batch commands
pub mod service_monitor_routes;
pub mod command_script_routes;
 
#[derive(RustEmbed, Clone)]
#[folder = "../frontend/dist"]
pub struct Assets;

 // Application state to share PgPool
#[derive(Clone)]
pub struct AppState {
    pub db_pool: DatabaseConnection, // Changed PgPool to DatabaseConnection
    pub live_server_data_cache: LiveServerDataCache,
    pub ws_data_broadcaster_tx: broadcast::Sender<WsMessage>,
    pub public_ws_data_broadcaster_tx: broadcast::Sender<WsMessage>, // For public, desensitized data
    pub connected_agents: Arc<Mutex<ConnectedAgents>>,
    pub update_trigger_tx: mpsc::Sender<()>,
    pub notification_service: Arc<NotificationService>,
    pub alert_service: Arc<AlertService>, // Added AlertService to AppState
    pub batch_command_manager: Arc<BatchCommandManager>,
    pub command_dispatcher: Arc<CommandDispatcher>, // Added CommandDispatcher
    pub batch_command_updates_tx: broadcast::Sender<BatchCommandUpdateMsg>, // For batch command WS updates
    pub result_broadcaster: Arc<ResultBroadcaster>, // For broadcasting batch command events
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
    #[error("Not Found: {0}")] // Added NotFound
    NotFound(String),
    #[error("Unauthorized: {0}")] // Added Unauthorized
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
            AppError::PasswordHashingError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Password hashing error: {}", msg)),
            AppError::TokenCreationError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Token creation error: {}", msg)),
            AppError::DatabaseError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", msg)),
            AppError::InternalServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::ServerError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg), // Or StatusCode::FORBIDDEN depending on context
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

// Simple health check handler
async fn health_check_handler() -> &'static str {
    "OK"
}

// Simple POST test handler
async fn login_test_handler() -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(serde_json::json!({ "message": "POST test successful" })))
}

pub fn create_axum_router(
    db_pool: DatabaseConnection,
    live_server_data_cache: LiveServerDataCache,
    ws_data_broadcaster_tx: broadcast::Sender<WsMessage>,
    public_ws_data_broadcaster_tx: broadcast::Sender<WsMessage>,
    connected_agents: Arc<Mutex<ConnectedAgents>>,
    update_trigger_tx: mpsc::Sender<()>,
    notification_service: Arc<NotificationService>,
    alert_service: Arc<AlertService>,
    batch_command_manager: Arc<BatchCommandManager>,
    batch_command_updates_tx: broadcast::Sender<BatchCommandUpdateMsg>,
    result_broadcaster: Arc<ResultBroadcaster>,
) -> Router {
    // batch_command_manager is now passed in
    let command_dispatcher = Arc::new(CommandDispatcher::new(
        connected_agents.clone(), // Assuming connected_agents is Arc<Mutex<ConnectedAgents>>
        batch_command_manager.clone(), // Use the passed-in batch_command_manager
    ));
    // result_broadcaster is now passed in

    let app_state = Arc::new(AppState {
        db_pool,
        live_server_data_cache,
        ws_data_broadcaster_tx: ws_data_broadcaster_tx.clone(),
        public_ws_data_broadcaster_tx,
        connected_agents,
        update_trigger_tx,
        notification_service,
        alert_service, // Initialize alert_service in AppState
        batch_command_manager, // Add BatchCommandManager to AppState
        command_dispatcher, // Add CommandDispatcher to AppState
        batch_command_updates_tx, // Add batch_command_updates_tx to AppState
        result_broadcaster, // Add result_broadcaster to AppState
    });

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
        .route("/ws/metrics", get(websocket_handler::websocket_handler)) // Added WebSocket route
        .route("/ws/public", get(websocket_handler::public_websocket_handler)) // Added Public WebSocket route
        .merge(metrics_routes::metrics_router()) // 合并指标路由
        .nest(
            "/api/vps",
            vps_routes::vps_router().route_layer(middleware::from_fn(auth_logic::auth)),
        ) // Added VPS routes with auth middleware
        .nest(
            "/api/settings",
            config_routes::create_settings_router().route_layer(middleware::from_fn(auth_logic::auth)),
        )
       .nest(
           "/api/tags",
           tag_routes::create_tags_router().route_layer(middleware::from_fn(auth_logic::auth)),
       )
       .nest(
            "/api/notifications",
            notification_routes::create_notification_router().route_layer(middleware::from_fn(auth_logic::auth)),
        )
        .nest(
            "/api/alerts",
            alert_routes::create_alert_router().route_layer(middleware::from_fn(auth_logic::auth)),
        )
        .nest(
            "/api/batch_commands",
            batch_command_routes::batch_command_routes() // Removed argument
                .route_layer(middleware::from_fn(auth_logic::auth)),
        )
        .nest(
            "/api/monitors",
            service_monitor_routes::create_service_monitor_router().route_layer(middleware::from_fn(auth_logic::auth)),
        )
        .nest(
            "/api/command-scripts",
            command_script_routes::command_script_routes().route_layer(middleware::from_fn(auth_logic::auth)),
        )
        .route("/ws/batch-command/{batch_command_id}", get(ws_batch_command_handler::batch_command_ws_handler)) // Corrected WebSocket route for batch command updates
        .with_state(app_state.clone())
        .layer(cors);

    app_router
}