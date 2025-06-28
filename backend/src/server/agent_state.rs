use std::collections::HashMap;
use crate::websocket_models::ServerWithDetails;
use std::sync::Arc;
use tokio::sync::Mutex;
use crate::agent_service::{AgentConfig, MessageToAgent, TriggerUpdateCheckCommand};
use tokio::sync::mpsc;
use std::fmt;
use tracing::{info, warn};
use crate::agent_service::message_to_agent::Payload;
use futures_util::{Sink, SinkExt};
use std::pin::Pin;
use std::task::{Context, Poll};
use axum::extract::ws::{WebSocket, Message};
use futures_util::stream::SplitSink;
use prost::Message as ProstMessage;

// 1. Define the AgentSender enum
#[derive(Clone)]
pub enum AgentSender {
    Grpc(mpsc::Sender<Result<MessageToAgent, tonic::Status>>),
    WebSocket(Arc<Mutex<SplitSink<WebSocket, Message>>>),
}

// 2. Implement Sink for AgentSender
impl Sink<MessageToAgent> for AgentSender {
    type Error = tonic::Status;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.get_mut() {
            AgentSender::Grpc(sender) => {
                // mpsc::Sender's poll_ready is for reserving a slot, which we don't need
                // to do explicitly when using try_send. We can consider it always ready
                // and let start_send handle the backpressure/closed channel case.
                Poll::Ready(Ok(()))
            }
            AgentSender::WebSocket(sink) => {
                let mut sink = sink.try_lock().unwrap();
                Pin::new(&mut *sink).poll_ready(cx).map_err(|e| tonic::Status::internal(e.to_string()))
            }
        }
    }

    fn start_send(self: Pin<&mut Self>, item: MessageToAgent) -> Result<(), Self::Error> {
        match self.get_mut() {
            AgentSender::Grpc(sender) => {
                sender.try_send(Ok(item)).map_err(|e| tonic::Status::internal(e.to_string()))
            }
            AgentSender::WebSocket(sink) => {
                let mut sink = sink.try_lock().unwrap();
                let mut buf = Vec::new();
                item.encode(&mut buf).unwrap();
                // Corrected: Convert Vec<u8> to Bytes
                Pin::new(&mut *sink).start_send(Message::Binary(buf.into())).map_err(|e| tonic::Status::internal(e.to_string()))
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.get_mut() {
            AgentSender::Grpc(_) => Poll::Ready(Ok(())),
            AgentSender::WebSocket(sink) => {
                let mut sink = sink.try_lock().unwrap();
                Pin::new(&mut *sink).poll_flush(cx).map_err(|e| tonic::Status::internal(e.to_string()))
            }
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        match self.get_mut() {
            AgentSender::Grpc(_) => Poll::Ready(Ok(())),
            AgentSender::WebSocket(sink) => {
                let mut sink = sink.try_lock().unwrap();
                Pin::new(&mut *sink).poll_close(cx).map_err(|e| tonic::Status::internal(e.to_string()))
            }
        }
    }
}

// 3. Update AgentState
#[derive(Clone)]
pub struct AgentState {
    pub agent_id: String,
    pub last_heartbeat_ms: i64,
    pub config: AgentConfig,
    pub vps_db_id: i32,
    pub sender: AgentSender,
}

impl fmt::Debug for AgentState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sender_type = match self.sender {
            AgentSender::Grpc(_) => "Grpc",
            AgentSender::WebSocket(_) => "WebSocket",
        };
        f.debug_struct("AgentState")
            .field("agent_id", &self.agent_id)
            .field("last_heartbeat_ms", &self.last_heartbeat_ms)
            .field("config", &self.config)
            .field("vps_db_id", &self.vps_db_id)
            .field("sender_type", &sender_type)
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

    pub fn find_by_vps_id(&self, vps_id: i32) -> Option<AgentState> {
        self.agents.values().find(|state| state.vps_db_id == vps_id).cloned()
    }

    // 4. Update send_update_check_command
    pub async fn send_update_check_command(&self, vps_id: i32) -> bool {
        if let Some(mut agent_state) = self.find_by_vps_id(vps_id) {
            let command = MessageToAgent {
                server_message_id: chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default() as u64,
                payload: Some(Payload::TriggerUpdateCheck(
                    TriggerUpdateCheckCommand {},
                )),
            };

            match agent_state.sender.send(command).await {
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

pub type LiveServerDataCache = Arc<Mutex<HashMap<i32, ServerWithDetails>>>;