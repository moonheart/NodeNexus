use axum::{
    extract::State,
    middleware as axum_middleware,
    http::{Method},
    response::{IntoResponse},
    routing::{post, get},
    Json,
    Router,
};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};
use rust_embed::RustEmbed;

use crate::server::agent_state::{ConnectedAgents, LiveServerDataCache};
use crate::server::config::ServerConfig;
use crate::web::models::websocket_models::WsMessage;
use tower_http::cors::{CorsLayer, Any};
use axum_extra::extract::cookie::{Cookie, SameSite};
use crate::notifications::service::NotificationService;
use crate::db::services::{AlertService, BatchCommandManager};
use crate::server::command_dispatcher::CommandDispatcher;
use crate::server::result_broadcaster::{ResultBroadcaster, BatchCommandUpdateMsg};
use crate::axum_embed::{ServeEmbed, FallbackBehavior};

use crate::services::auth_service;
use crate::web::{
    error::AppError,
    models::{LoginRequest, RegisterRequest},
    middleware::auth,
    routes::*,
    handlers::*,
};

pub mod error;
pub mod handlers;
pub mod models;
pub mod routes;
pub mod middleware;

#[derive(RustEmbed, Clone)]
#[folder = "../frontend/dist"]
pub struct Assets;

pub fn create_static_file_service() -> ServeEmbed<Assets> {
    ServeEmbed::<Assets>::with_parameters(
        Some("index.html".to_string()),
        FallbackBehavior::Ok,
        Some("index.html".to_string()),
    )
}

#[derive(Clone)]
pub struct AppState {
    pub db_pool: DatabaseConnection,
    pub live_server_data_cache: LiveServerDataCache,
    pub ws_data_broadcaster_tx: broadcast::Sender<WsMessage>,
    pub public_ws_data_broadcaster_tx: broadcast::Sender<WsMessage>,
    pub connected_agents: Arc<Mutex<ConnectedAgents>>,
    pub update_trigger_tx: mpsc::Sender<()>,
    pub notification_service: Arc<NotificationService>,
    pub alert_service: Arc<AlertService>,
    pub batch_command_manager: Arc<BatchCommandManager>,
    pub command_dispatcher: Arc<CommandDispatcher>,
    pub batch_command_updates_tx: broadcast::Sender<BatchCommandUpdateMsg>,
    pub result_broadcaster: Arc<ResultBroadcaster>,
    pub config: Arc<ServerConfig>,
}

async fn register_handler(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<models::UserResponse>, AppError> {
    match auth_service::register_user(&app_state.db_pool, payload).await {
        Ok(user_response) => Ok(Json(user_response)),
        Err(e) => Err(e),
    }
}

async fn login_handler(
    State(app_state): State<Arc<AppState>>,
    Json(payload): Json<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    let login_response = auth_service::login_user(&app_state.db_pool, payload, &app_state.config.jwt_secret).await?;

    let auth_cookie = Cookie::build(("token", login_response.token.clone()))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(true)
        .build();

    let mut response = Json(login_response).into_response();
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        auth_cookie.to_string().parse().unwrap(),
    );

    Ok(response)
}

async fn health_check_handler() -> &'static str {
    "OK"
}

async fn login_test_handler() -> (axum::http::StatusCode, Json<serde_json::Value>) {
    (axum::http::StatusCode::OK, Json(serde_json::json!({ "message": "POST test successful" })))
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
    config: Arc<ServerConfig>,
) -> Router {
    let command_dispatcher = Arc::new(CommandDispatcher::new(
        connected_agents.clone(),
        batch_command_manager.clone(),
    ));

    let app_state = Arc::new(AppState {
        db_pool,
        live_server_data_cache,
        ws_data_broadcaster_tx: ws_data_broadcaster_tx.clone(),
        public_ws_data_broadcaster_tx,
        connected_agents,
        update_trigger_tx,
        notification_service,
        alert_service,
        batch_command_manager,
        command_dispatcher,
        batch_command_updates_tx,
        result_broadcaster,
        config,
    });

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(vec![Method::GET, Method::POST, Method::PUT, Method::DELETE, Method::OPTIONS])
        .allow_headers(Any);

    

    Router::new()
        .route("/api/health", get(health_check_handler))
        .route("/login_test_simple", post(login_test_handler))
        .route("/api/auth/login_test", post(login_test_handler))
        .route("/api/auth/register", post(register_handler))
        .route("/api/auth/login", post(login_handler))
        .route("/api/auth/me", get(auth_service::me).route_layer(axum_middleware::from_fn_with_state(app_state.clone(), auth::auth)))
        .nest(
            "/api/auth",
            oauth_routes::create_public_router().merge(
                oauth_routes::create_protected_router()
                    .route_layer(axum_middleware::from_fn_with_state(app_state.clone(), auth::auth)),
            ),
        )
        .route("/ws/metrics", get(websocket_handler::websocket_handler))
        .route("/ws/public", get(websocket_handler::public_websocket_handler))
        .route("/ws/agent", get(crate::server::ws_agent_handler::ws_agent_handler))
        .merge(metrics_routes::metrics_router())
        .nest(
            "/api/vps",
            vps_routes::vps_router().route_layer(axum_middleware::from_fn_with_state(app_state.clone(), auth::auth)),
        )
        .nest(
            "/api/settings",
            config_routes::create_settings_router().route_layer(axum_middleware::from_fn_with_state(app_state.clone(), auth::auth)),
        )
       .nest(
           "/api/tags",
           tag_routes::create_tags_router().route_layer(axum_middleware::from_fn_with_state(app_state.clone(), auth::auth)),
       )
       .nest(
            "/api/admin/oauth",
            admin_oauth_routes::create_router().route_layer(axum_middleware::from_fn_with_state(app_state.clone(), auth::auth)),
       )
       .nest(
            "/api/notifications",
            notification_routes::create_notification_router().route_layer(axum_middleware::from_fn_with_state(app_state.clone(), auth::auth)),
        )
        .nest(
            "/api/alerts",
            alert_routes::create_alert_router().route_layer(axum_middleware::from_fn_with_state(app_state.clone(), auth::auth)),
        )
        .nest(
            "/api/batch_commands",
            batch_command_routes::batch_command_routes()
                .route_layer(axum_middleware::from_fn_with_state(app_state.clone(), auth::auth)),
        )
        .nest(
            "/api/monitors",
            service_monitor_routes::create_service_monitor_router().route_layer(axum_middleware::from_fn_with_state(app_state.clone(), auth::auth)),
        )
        .nest(
            "/api/command-scripts",
            command_script_routes::command_script_routes().route_layer(axum_middleware::from_fn_with_state(app_state.clone(), auth::auth)),
        )
       .nest(
           "/api/user",
           user_routes::create_user_router().route_layer(axum_middleware::from_fn_with_state(app_state.clone(), auth::auth)),
       )
        .route("/ws/batch-command/{batch_command_id}", get(ws_batch_command_handler::batch_command_ws_handler))
        .with_state(app_state.clone())
        .layer(cors)
}