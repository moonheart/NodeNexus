use crate::agent_service::AgentConfig;
use crate::agent_service::Heartbeat;
use crate::agent_service::MessageToServer;
use crate::agent_service::message_to_server::Payload as ServerPayload;
use chrono::Utc;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error};

pub async fn heartbeat_loop(
    tx_to_server: mpsc::Sender<MessageToServer>,
    shared_agent_config: Arc<RwLock<AgentConfig>>,
    agent_id: String,
    id_provider: impl Fn() -> u64 + Send + Sync + 'static,
    vps_db_id: i32,
    agent_secret: String,
) {
    loop {
        let interval_duration = {
            let config = shared_agent_config.read().unwrap();
            let seconds = config.heartbeat_interval_seconds;
            if seconds > 0 { seconds } else { 30 }
        };

        debug!(interval_seconds = interval_duration, "Heartbeat task tick.");
        tokio::time::sleep(Duration::from_secs(interval_duration as u64)).await;

        let heartbeat_payload = Heartbeat {
            timestamp_unix_ms: Utc::now().timestamp_millis(),
        };
        let msg_id = id_provider();
        if let Err(e) = tx_to_server
            .send(MessageToServer {
                client_message_id: msg_id,
                payload: Some(ServerPayload::Heartbeat(heartbeat_payload)),
                vps_db_id,
                agent_secret: agent_secret.clone(),
            })
            .await
        {
            error!(error = %e, "Failed to send heartbeat. Exiting heartbeat task.");
            break;
        }
    }
}
