use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set, DbErr};
use crate::db::entities::user;
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{encode, EncodingKey, Header};
use chrono::{Utc, Duration};
use axum::Extension;

use crate::web::error::AppError;
use crate::web::models::{RegisterRequest, LoginRequest, UserResponse, LoginResponse, Claims, AuthenticatedUser};

pub async fn register_user(pool: &DatabaseConnection, req: RegisterRequest) -> Result<UserResponse, AppError> {
    if req.username.is_empty() || req.password.len() < 8 {
        return Err(AppError::InvalidInput("用户名不能为空，密码至少需要8个字符。".to_string()));
    }

    let existing_user_by_username: Option<user::Model> = user::Entity::find()
        .filter(user::Column::Username.eq(&req.username))
        .one(pool)
        .await
        .map_err(|e: DbErr| AppError::DatabaseError(format!("检查用户名是否存在时出错: {}", e)))?;

    if existing_user_by_username.is_some() {
        return Err(AppError::UserAlreadyExists("用户名已被使用。".to_string()));
    }

    let password_hash = hash(&req.password, DEFAULT_COST)
        .map_err(|e| AppError::PasswordHashingError(format!("密码哈希失败: {}", e)))?;

    let new_user = user::ActiveModel {
        username: Set(req.username.clone()),
        password_hash: Set(Some(password_hash)),
        ..Default::default() // Handles id, created_at, updated_at
    };

    match new_user.insert(pool).await {
        Ok(user_model) => Ok(UserResponse {
            id: user_model.id,
            username: user_model.username,
        }),
        Err(e) => Err(AppError::DatabaseError(format!("创建用户失败: {}", e))),
    }
}

pub async fn login_user(pool: &DatabaseConnection, req: LoginRequest, jwt_secret: &str) -> Result<LoginResponse, AppError> {
    if req.username.is_empty() || req.password.is_empty() {
        return Err(AppError::InvalidInput("用户名和密码不能为空。".to_string()));
    }

    // Allow login with username
    let user_model_option = user::Entity::find()
            .filter(user::Column::Username.eq(&req.username))
            .one(pool)
            .await
            .map_err(|e: DbErr| AppError::DatabaseError(format!("通过用户名查询用户失败: {}", e)))?;

    let user = match user_model_option {
        Some(u) => u,
        None => return Err(AppError::UserNotFound),
    };

    if user.password_login_disabled {
        return Err(AppError::InvalidCredentials); // Or a more specific error
    }

    let password_hash = match user.password_hash.as_ref() {
        Some(hash) => hash,
        None => return Err(AppError::InvalidCredentials), // No password set for this user
    };

    let valid_password = verify(&req.password, password_hash)
        .map_err(|e| AppError::InternalServerError(format!("密码验证过程中出错: {}",e)))?;

    if !valid_password {
        return Err(AppError::InvalidCredentials);
    }

    create_jwt_for_user(&user, jwt_secret)
}

pub fn create_jwt_for_user(user: &user::Model, jwt_secret: &str) -> Result<LoginResponse, AppError> {
    let now = Utc::now();
    // Token valid for 24 hours, as per plan
    let expiration = (now + Duration::hours(24)).timestamp() as usize;

    let claims = Claims {
        sub: user.username.clone(), // Using username as subject
        user_id: user.id,
        exp: expiration,
    };

    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(jwt_secret.as_ref()))
        .map_err(|e| AppError::TokenCreationError(format!("生成Token失败: {}", e)))?;

    Ok(LoginResponse {
        token,
        user_id: user.id,
        username: user.username.clone(),
    })
}

pub async fn me(
    Extension(user): Extension<AuthenticatedUser>
) -> Result<axum::Json<UserResponse>, AppError> {
    Ok(axum::Json(UserResponse {
        id: user.id,
        username: user.username,
    }))
}