use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub id: i32,
    pub user_id: i32,
    pub command_text: String,
    pub working_directory: String,
    pub created_at: chrono::DateTime<chrono::FixedOffset>,
}
