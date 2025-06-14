use serde::{Deserialize, Serialize};

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


use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set, DbErr}; // Added SeaORM imports, removed ActiveValue
use crate::db::entities::user; // Changed to use user entity
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{decode, encode, EncodingKey, Header, Validation, DecodingKey};
use chrono::{Utc, Duration};
use std::env;
use crate::http_server::AppError;
use axum::http::{Request, header};
use tracing::warn;
use axum::middleware::Next;
use axum::{response::Response, body::Body as AxumBody}; // Import AxumBody

// JWT Claims structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims { // Made public
    pub sub: String, // Subject (user_id or username)
    pub user_id: i32,
    pub email: String,
    pub exp: usize,  // Expiration time (timestamp)
}

/// Struct to hold authenticated user details, to be passed as a request extension.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub id: i32,
    pub username: String,
    pub email: String,
}

pub fn get_jwt_secret() -> String { // Made public
    env::var("JWT_SECRET").unwrap_or_else(|_| {
        // Fallback for development if JWT_SECRET is not set.
        // WARNING: Do NOT use this fallback in production.
        // In a real application, this should cause a panic or a proper error if not set in production.
        warn!("SECURITY WARNING: JWT_SECRET environment variable not set. Using a default, insecure secret for development.");
        "your-super-secret-and-long-jwt-secret-for-dev-only".to_string()
    })
}

pub async fn register_user(pool: &DatabaseConnection, req: RegisterRequest) -> Result<UserResponse, AppError> {
    if req.username.is_empty() || req.email.is_empty() || req.password.len() < 8 {
        return Err(AppError::InvalidInput("用户名、邮箱不能为空，密码至少需要8个字符。".to_string()));
    }
    if !req.email.contains('@') { // Basic email format check
        return Err(AppError::InvalidInput("无效的邮箱格式。".to_string()));
    }

    let existing_user_by_username: Option<user::Model> = user::Entity::find()
        .filter(user::Column::Username.eq(&req.username))
        .one(pool)
        .await
        .map_err(|e: DbErr| AppError::DatabaseError(format!("检查用户名是否存在时出错: {}", e)))?;

    if existing_user_by_username.is_some() {
        return Err(AppError::UserAlreadyExists("用户名已被使用。".to_string()));
    }

    let existing_user_by_email: Option<user::Model> = user::Entity::find()
        .filter(user::Column::Email.eq(&req.email))
        .one(pool)
        .await
        .map_err(|e: DbErr| AppError::DatabaseError(format!("检查邮箱是否存在时出错: {}", e)))?;

    if existing_user_by_email.is_some() {
        return Err(AppError::UserAlreadyExists("邮箱已被注册。".to_string()));
    }

    let password_hash = hash(&req.password, DEFAULT_COST)
        .map_err(|e| AppError::PasswordHashingError(format!("密码哈希失败: {}", e)))?;

    let new_user = user::ActiveModel {
        username: Set(req.username.clone()),
        email: Set(req.email.clone()),
        password_hash: Set(password_hash),
        ..Default::default() // Handles id, created_at, updated_at
    };

    match new_user.insert(pool).await {
        Ok(user_model) => Ok(UserResponse {
            id: user_model.id,
            username: user_model.username,
            email: user_model.email,
        }),
        Err(e) => Err(AppError::DatabaseError(format!("创建用户失败: {}", e))),
    }
}

pub async fn login_user(pool: &DatabaseConnection, req: LoginRequest) -> Result<LoginResponse, AppError> {
    if req.email.is_empty() || req.password.is_empty() {
        return Err(AppError::InvalidInput("邮箱和密码不能为空。".to_string()));
    }

    // Allow login with either username or email
    let user_model_option = if req.email.contains('@') {
        user::Entity::find()
            .filter(user::Column::Email.eq(&req.email))
            .one(pool)
            .await
            .map_err(|e: DbErr| AppError::DatabaseError(format!("通过邮箱查询用户失败: {}", e)))?
    } else {
        user::Entity::find()
            .filter(user::Column::Username.eq(&req.email))
            .one(pool)
            .await
            .map_err(|e: DbErr| AppError::DatabaseError(format!("通过用户名查询用户失败: {}", e)))?
    };

    let user = match user_model_option {
        Some(u) => u,
        None => return Err(AppError::UserNotFound),
    };

    let valid_password = verify(&req.password, &user.password_hash)
        .map_err(|e| AppError::InternalServerError(format!("密码验证过程中出错: {}",e)))?;

    if !valid_password {
        return Err(AppError::InvalidCredentials);
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
        .map_err(|e| AppError::TokenCreationError(format!("生成Token失败: {}", e)))?;

    Ok(LoginResponse {
        token,
        user_id: user.id,
        username: user.username,
        email: user.email,
    })
}

pub async fn auth(
    mut req: Request<AxumBody>, // Changed from Request<B> to Request<AxumBody>
    next: Next, // Removed <B> from Next
) -> Result<Response, AppError> {
    let auth_header = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let auth_header = if let Some(auth_header) = auth_header {
        auth_header
    } else {
        return Err(AppError::InvalidCredentials); // Or a more specific "MissingToken" error
    };

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::InvalidCredentials); // Or "InvalidTokenFormat"
    }

    let token = &auth_header[7..];
    let jwt_secret = get_jwt_secret();

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &Validation::default(), // TODO: Consider more specific validation if needed (e.g., issuer)
    ).map_err(|e| {
        warn!(error = ?e, "JWT decoding error during auth middleware.");
        AppError::InvalidCredentials // Or "InvalidToken"
    })?;

    let authenticated_user = AuthenticatedUser {
        id: token_data.claims.user_id,
        username: token_data.claims.sub, // Assuming 'sub' is username
        email: token_data.claims.email,
    };
    req.extensions_mut().insert(authenticated_user);
    Ok(next.run(req).await)
}