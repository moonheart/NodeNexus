use serde::{Deserialize, Serialize}; // Keep Serialize/Deserialize imports

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub vps_id: i32,
    pub tag_id: i32,
}
