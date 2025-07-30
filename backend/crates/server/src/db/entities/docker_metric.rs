use serde::{Deserialize, Serialize}; // Keep Serialize/Deserialize imports for now, might be needed by other parts or if we re-add

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Model {
    pub time: chrono::DateTime<chrono::Utc>,
    pub container_db_id: i32, // Foreign key to docker_containers.id
    pub cpu_usage: f64,
    pub mem_usage: f64, // Assuming this will store bytes
}
