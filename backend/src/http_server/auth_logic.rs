use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginRequest {
    pub email: String, // Can be username or email, will handle in logic
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: i32,
    pub username: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub token: String,
    pub user_id: i32,
    pub username: String,
    pub email: String,
}

#[derive(Error, Debug)]
pub enum AuthError {
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
}

use sqlx::PgPool;
use crate::db::models::User; // Assuming models.rs is in db module and User is public
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{encode, EncodingKey, Header};
use chrono::{Utc, Duration};
use std::env;

// JWT Claims structure
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String, // Subject (user_id or username)
    user_id: i32,
    email: String,
    exp: usize,  // Expiration time (timestamp)
}

fn get_jwt_secret() -> String {
    env::var("JWT_SECRET").unwrap_or_else(|_| {
        // Fallback for development if JWT_SECRET is not set.
        // WARNING: Do NOT use this fallback in production.
        // In a real application, this should cause a panic or a proper error if not set in production.
        eprintln!("WARNING: JWT_SECRET environment variable not set. Using a default, insecure secret for development.");
        "your-super-secret-and-long-jwt-secret-for-dev-only".to_string()
    })
}

pub async fn register_user(pool: &PgPool, req: RegisterRequest) -> Result<UserResponse, AuthError> {
    if req.username.is_empty() || req.email.is_empty() || req.password.len() < 8 {
        return Err(AuthError::InvalidInput("用户名、邮箱不能为空，密码至少需要8个字符。".to_string()));
    }
    if !req.email.contains('@') { // Basic email format check
        return Err(AuthError::InvalidInput("无效的邮箱格式。".to_string()));
    }

    let existing_user_by_username: Option<User> = sqlx::query_as("SELECT id, username, email, password_hash, created_at, updated_at FROM users WHERE username = $1")
        .bind(&req.username)
        .fetch_optional(pool)
        .await
        .map_err(|e| AuthError::DatabaseError(format!("检查用户名是否存在时出错: {}", e)))?;

    if existing_user_by_username.is_some() {
        return Err(AuthError::UserAlreadyExists("用户名已被使用。".to_string()));
    }

    let existing_user_by_email: Option<User> = sqlx::query_as("SELECT id, username, email, password_hash, created_at, updated_at FROM users WHERE email = $1")
        .bind(&req.email)
        .fetch_optional(pool)
        .await
        .map_err(|e| AuthError::DatabaseError(format!("检查邮箱是否存在时出错: {}", e)))?;

    if existing_user_by_email.is_some() {
        return Err(AuthError::UserAlreadyExists("邮箱已被注册。".to_string()));
    }

    let password_hash = hash(&req.password, DEFAULT_COST)
        .map_err(|e| AuthError::PasswordHashingError(format!("密码哈希失败: {}", e)))?;

    let new_user_result = sqlx::query_as::<_, User>(
        "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3) RETURNING id, username, email, password_hash, created_at, updated_at"
    )
    .bind(&req.username)
    .bind(&req.email)
    .bind(&password_hash)
    .fetch_one(pool)
    .await;

    match new_user_result {
        Ok(user) => Ok(UserResponse {
            id: user.id,
            username: user.username,
            email: user.email,
        }),
        Err(e) => Err(AuthError::DatabaseError(format!("创建用户失败: {}", e))),
    }
}

pub async fn login_user(pool: &PgPool, req: LoginRequest) -> Result<LoginResponse, AuthError> {
    if req.email.is_empty() || req.password.is_empty() {
        return Err(AuthError::InvalidInput("邮箱和密码不能为空。".to_string()));
    }

    let user_result = sqlx::query_as::<_, User>("SELECT id, username, email, password_hash, created_at, updated_at FROM users WHERE email = $1")
        .bind(&req.email) // Assuming login is by email
        .fetch_optional(pool)
        .await;

    let user = match user_result {
        Ok(Some(u)) => u,
        Ok(None) => return Err(AuthError::UserNotFound),
        Err(e) => return Err(AuthError::DatabaseError(format!("查询用户失败: {}", e))),
    };

    let valid_password = verify(&req.password, &user.password_hash)
        .map_err(|e| AuthError::InternalServerError(format!("密码验证过程中出错: {}",e)))?;

    if !valid_password {
        return Err(AuthError::InvalidCredentials);
    }

    let now = Utc::now();
    // Token valid for 24 hours, as per plan
    let expiration = (now + Duration::hours(24)).timestamp() as usize;

    let claims = Claims {
        sub: user.username.clone(), // Using username as subject
        user_id: user.id,
        email: user.email.clone(),
        exp: expiration,
    };

    let jwt_secret = get_jwt_secret();
    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(jwt_secret.as_ref()))
        .map_err(|e| AuthError::TokenCreationError(format!("生成Token失败: {}", e)))?;

    Ok(LoginResponse {
        token,
        user_id: user.id,
        username: user.username,
        email: user.email,
    })
}