use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub id: i32,
    pub vps_id: i32,
    pub container_id_on_host: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub created_at_on_host: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    // Note: `labels` and `mounts` from the original proto could be added here
    // as a JsonBinary column if they are stored in the database.
    // Example:
    // pub labels: Option<Json>,
    // pub mounts: Option<Json>,
}
