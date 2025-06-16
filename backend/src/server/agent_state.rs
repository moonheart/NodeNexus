

use std::collections::HashMap;
use crate::websocket_models::ServerWithDetails;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::agent_service::{AgentConfig, MessageToAgent, TriggerUpdateCheckCommand};
use tokio::sync::mpsc;
use std::fmt;
use tracing::{info, warn};
use crate::agent_service::message_to_agent::Payload;

#[derive(Clone)]
pub struct AgentState {
    pub agent_id: String,
    pub last_heartbeat_ms: i64,
    pub config: AgentConfig,
    pub vps_db_id: i32,
    pub sender: mpsc::Sender<Result<MessageToAgent, tonic::Status>>,
}

impl fmt::Debug for AgentState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AgentState")
            .field("agent_id", &self.agent_id)
            .field("last_heartbeat_ms", &self.last_heartbeat_ms)
            .field("config", &self.config)
            .field("vps_db_id", &self.vps_db_id)
            .field("sender", &"mpsc::Sender<...>") // Don't print the sender itself
            .finish()
    }
}

#[derive(Default, Debug)]
pub struct ConnectedAgents {
    pub agents: HashMap<String, AgentState>,
}

impl ConnectedAgents {
    pub fn new() -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self::default()))
    }

    /// Finds an agent's state by their VPS database ID.
    /// This requires iterating through the values, so it's O(n) on the number of connected agents.
    pub fn find_by_vps_id(&self, vps_id: i32) -> Option<AgentState> {
        self.agents.values().find(|state| state.vps_db_id == vps_id).cloned()
    }

    /// Sends a command to an agent to trigger an update check.
    pub async fn send_update_check_command(&self, vps_id: i32) -> bool {
        if let Some(agent_state) = self.find_by_vps_id(vps_id) {
            let command = MessageToAgent {
                server_message_id: chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default() as u64,
                payload: Some(Payload::TriggerUpdateCheck(
                    TriggerUpdateCheckCommand {},
                )),
            };

            match agent_state.sender.send(Ok(command)).await {
                Ok(_) => {
                    info!(vps_id, "Successfully sent TriggerUpdateCheckCommand to agent.");
                    true
                }
                Err(e) => {
                    warn!(vps_id, error = %e, "Failed to send TriggerUpdateCheckCommand to agent, channel closed.");
                    false
                }
            }
        } else {
            warn!(vps_id, "Could not send TriggerUpdateCheckCommand: agent not found in connected list.");
            false
        }
    }
}

// Cache for live server data including basic info and latest metrics
pub type LiveServerDataCache = Arc<Mutex<HashMap<i32, ServerWithDetails>>>;