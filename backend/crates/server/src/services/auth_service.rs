use crate::db::duckdb_service::{user_service, DuckDbPool};
use axum::Extension;
use bcrypt::{DEFAULT_COST, hash, verify};
use chrono::{Duration, Utc};
use jsonwebtoken::{EncodingKey, Header, encode};

use crate::db::entities::user;
use crate::web::error::AppError;
use crate::web::models::{
    AuthenticatedUser, Claims, LoginRequest, LoginResponse, RegisterRequest, UserResponse,
};

pub async fn register_user(
    pool: DuckDbPool,
    req: RegisterRequest,
) -> Result<UserResponse, AppError> {
    if req.username.is_empty() || req.password.len() < 8 {
        return Err(AppError::InvalidInput(
            "用户名不能为空，密码至少需要8个字符。".to_string(),
        ));
    }

    let existing_user = user_service::get_user_by_username(pool.clone(), req.username.clone()).await?;
    if existing_user.is_some() {
        return Err(AppError::UserAlreadyExists("用户名已被使用。".to_string()));
    }

    let password_hash = hash(&req.password, DEFAULT_COST)
        .map_err(|e| AppError::PasswordHashingError(format!("密码哈希失败: {e}")))?;

    let user_model = user_service::create_user(pool, req.username, password_hash).await?;
    
    Ok(UserResponse {
        id: user_model.id,
        username: user_model.username,
    })
}

pub async fn login_user(
    pool: DuckDbPool,
    req: LoginRequest,
    jwt_secret: &str,
) -> Result<LoginResponse, AppError> {
    if req.username.is_empty() || req.password.is_empty() {
        return Err(AppError::InvalidInput("用户名和密码不能为空。".to_string()));
    }

    let user_model_option = user_service::get_user_by_username(pool, req.username).await?;

    let user = match user_model_option {
        Some(u) => u,
        None => return Err(AppError::UserNotFound),
    };

    if user.password_login_disabled {
        return Err(AppError::InvalidCredentials);
    }

    let password_hash = match user.password_hash.as_ref() {
        Some(hash) => hash,
        None => return Err(AppError::InvalidCredentials), // No password set for this user
    };

    let valid_password = verify(&req.password, password_hash)
        .map_err(|e| AppError::InternalServerError(format!("密码验证过程中出错: {e}")))?;

    if !valid_password {
        return Err(AppError::InvalidCredentials);
    }

    create_jwt_for_user(&user, jwt_secret)
}

pub fn create_jwt_for_user(
    user: &user::Model,
    jwt_secret: &str,
) -> Result<LoginResponse, AppError> {
    let now = Utc::now();
    // Token valid for 24 hours, as per plan
    let expiration = (now + Duration::hours(24 * 7)).timestamp() as usize;

    let claims = Claims {
        sub: user.username.clone(), // Using username as subject
        user_id: user.id,
        exp: expiration,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_ref()),
    )
    .map_err(|e| AppError::TokenCreationError(format!("生成Token失败: {e}")))?;

    Ok(LoginResponse {
        token,
        user_id: user.id,
        username: user.username.clone(),
    })
}

pub async fn me(
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<axum::Json<UserResponse>, AppError> {
    Ok(axum::Json(UserResponse {
        id: user.id,
        username: user.username,
    }))
}
