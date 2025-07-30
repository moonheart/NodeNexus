use serde::{Deserialize, Serialize}; // Keep Serialize/Deserialize imports

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub vps_id: i32,
    pub month: chrono::NaiveDate, // Corresponds to DATE type in SQL
    pub total_rx: i64,
    pub total_tx: i64,
}
