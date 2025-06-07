use chrono::Utc;
use sqlx::{PgPool, Result};

use crate::db::models::User;

// --- User Service Functions ---

/// Creates a new user.
pub async fn create_user(
    pool: &PgPool,
    username: &str,
    password_hash: &str,
    email: &str,
) -> Result<User> {
    let now = Utc::now();
    let user = sqlx::query_as!(
        User,
        r#"
        INSERT INTO users (username, password_hash, email, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id, username, password_hash, email, created_at, updated_at
        "#,
        username,
        password_hash,
        email,
        now,
        now
    )
    .fetch_one(pool)
    .await?;
    Ok(user)
}

/// Retrieves a user by their ID.
pub async fn get_user_by_id(pool: &PgPool, user_id: i32) -> Result<Option<User>> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE id = $1", user_id)
        .fetch_optional(pool)
        .await
}

/// Retrieves a user by their username.
pub async fn get_user_by_username(pool: &PgPool, username: &str) -> Result<Option<User>> {
    sqlx::query_as!(User, "SELECT * FROM users WHERE username = $1", username)
        .fetch_optional(pool)
        .await
}