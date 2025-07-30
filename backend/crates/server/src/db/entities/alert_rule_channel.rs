use serde::{Deserialize, Serialize}; // Keep Serialize/Deserialize imports

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub alert_rule_id: i32,
    pub channel_id: i32,
}
