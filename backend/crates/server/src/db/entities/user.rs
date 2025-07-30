use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub id: i32,
    pub username: String,
    pub password_hash: Option<String>,
    pub role: String,
    pub password_login_disabled: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub theme_mode: String,
    pub active_theme_id: Option<i32>,
    pub language: String,
}
