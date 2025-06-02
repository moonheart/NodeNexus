

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::agent_service::AgentConfig;

#[derive(Debug, Clone)]
pub struct AgentState {
    pub agent_id: String,
    pub last_heartbeat_ms: i64,
    pub config: AgentConfig,
    pub vps_db_id: i32,
}

#[derive(Default, Debug)]
pub struct ConnectedAgents {
    pub agents: HashMap<String, AgentState>,
}

impl ConnectedAgents {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::default()))
    }
}