use chrono::Utc;
use sqlx::{PgPool, Result};
use super::models::{User, Vps};

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

// --- Vps Service Functions ---

/// Creates a new VPS entry.
pub async fn create_vps(
    pool: &PgPool,
    user_id: i32,
    name: &str,
    ip_address: &str,
    os_type: Option<String>,
    agent_secret: &str,
    status: &str,
    metadata: Option<serde_json::Value>,
) -> Result<Vps> {
    let now = Utc::now();
    let vps = sqlx::query_as!(
        Vps,
        r#"
        INSERT INTO vps (user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING id, user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at
        "#,
        user_id,
        name,
        ip_address,
        os_type,
        agent_secret,
        status,
        metadata,
        now,
        now
    )
    .fetch_one(pool)
    .await?;
    Ok(vps)
}

/// Retrieves a VPS by its ID.
pub async fn get_vps_by_id(pool: &PgPool, vps_id: i32) -> Result<Option<Vps>> {
    sqlx::query_as!(Vps, "SELECT * FROM vps WHERE id = $1", vps_id)
        .fetch_optional(pool)
        .await
}

/// Retrieves all VPS entries for a given user.
pub async fn get_vps_by_user_id(pool: &PgPool, user_id: i32) -> Result<Vec<Vps>> {
    sqlx::query_as!(Vps, "SELECT * FROM vps WHERE user_id = $1 ORDER BY created_at DESC", user_id)
        .fetch_all(pool)
        .await
}

/// Updates the status of a VPS.
pub async fn update_vps_status(pool: &PgPool, vps_id: i32, status: &str) -> Result<u64> {
    let now = Utc::now();
    let rows_affected = sqlx::query!(
        "UPDATE vps SET status = $1, updated_at = $2 WHERE id = $3",
        status,
        now,
        vps_id
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows_affected)
}

// TODO: Implement service functions for other models:
// - PerformanceMetric (batch insert, query by time range)
// - DockerContainer (create, update, get by vps_id)
// - DockerMetric (batch insert, query by time range)
// - Task (create, update, get, list)
// - TaskRun (create, update, get by task_id)
// - AlertRule (create, update, get, list)
// - AlertEvent (create, list by rule_id/vps_id)
// - VpsMonthlyTraffic (upsert, get)