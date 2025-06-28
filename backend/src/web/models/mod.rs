use chrono::{DateTime, Utc};
use uuid::Uuid;
use serde::{Deserialize, Serialize};

pub mod alert_models;
pub mod service_monitor_models;
pub mod websocket_models;
pub mod batch_command_models;

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: i32,
    pub username: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user_id: i32,
    pub username: String,
}

// JWT Claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (user_id or username)
    pub user_id: i32,
    pub exp: usize,  // Expiration time (timestamp)
}

/// Struct to hold authenticated user details, to be passed as a request extension.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub id: i32,
    pub username: String,
}
