use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set,
};

use crate::db::entities::user; // Changed from models::User

// --- User Service Functions ---

/// Creates a new user.
pub async fn create_user(
    db: &DatabaseConnection, // Changed from pool: &PgPool
    username: &str,
    password_hash: &str,
    email: &str,
) -> Result<user::Model, DbErr> { // Changed return type
    let now = Utc::now();
    let new_user = user::ActiveModel {
        username: Set(username.to_owned()),
        password_hash: Set(password_hash.to_owned()),
        email: Set(email.to_owned()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default() // id will be set by the database
    };
    new_user.insert(db).await
}

/// Retrieves a user by their ID.
pub async fn get_user_by_id(
    db: &DatabaseConnection, // Changed from pool: &PgPool
    user_id: i32,
) -> Result<Option<user::Model>, DbErr> { // Changed return type
    user::Entity::find_by_id(user_id).one(db).await
}

/// Retrieves a user by their username.
pub async fn get_user_by_username(
    db: &DatabaseConnection, // Changed from pool: &PgPool
    username: &str,
) -> Result<Option<user::Model>, DbErr> { // Changed return type
    user::Entity::find()
        .filter(user::Column::Username.eq(username))
        .one(db)
        .await
}