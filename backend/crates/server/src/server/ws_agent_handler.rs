use axum::{
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use futures_util::{
    sink::Sink,
    stream::{SplitSink, SplitStream, Stream, StreamExt},
};
use prost::Message as ProstMessage;
use std::{
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tokio::sync::Mutex;
use tracing::{info, warn};

use nodenexus_common::agent_service::{MessageToAgent, MessageToServer};
use crate::{
    server::{
        agent_state::AgentSender,
        core_services::{self, AgentStream},
    },
    web::AppState,
};

/// Axum handler for the WebSocket agent connection.
pub async fn ws_agent_handler(
    ws: WebSocketUpgrade,
    State(app_state): State<Arc<AppState>>,
) -> Response {
    info!("New WebSocket agent connection request.");
    ws.on_upgrade(move |socket| handle_socket(socket, app_state))
}

/// Handles the WebSocket connection after the upgrade.
async fn handle_socket(socket: WebSocket, app_state: Arc<AppState>) {
    info!("WebSocket connection upgraded. Creating adapter.");
    let (ws_sender, ws_receiver) = socket.split();

    // The sender needs to be wrapped in Arc<Mutex<>> to be shared.
    let ws_sender_arc = Arc::new(Mutex::new(ws_sender));

    let adapter = WebSocketStreamAdapter {
        receiver: ws_receiver,
        // Clone the Arc for the adapter. The original Arc will be moved into the AgentState.
        sender: ws_sender_arc.clone(),
    };

    // Create the AgentSender enum variant for WebSocket.
    let agent_sender = AgentSender::WebSocket(ws_sender_arc);

    // Spawn the core processing logic.
    // NOTE: We are now passing the generic `agent_sender` to `process_agent_stream`.
    // The `process_agent_stream` function will need to be updated to accept this
    // and perform the agent registration.
    // For now, we pass the adapter and the original AppState components.
    // We will adjust this call after updating process_agent_stream's signature.
    let context = Arc::new(core_services::AgentStreamContext {
        connected_agents: app_state.connected_agents.clone(),
        db_pool: Arc::from(app_state.db_pool.clone()),
        ws_data_broadcaster_tx: app_state.ws_data_broadcaster_tx.clone(),
        update_trigger_tx: app_state.update_trigger_tx.clone(),
        batch_command_manager: app_state.batch_command_manager.clone(),
        metric_sender: app_state.metric_sender.clone(),
    });

    tokio::spawn(async move {
        core_services::process_agent_stream(
            adapter,
            agent_sender,
            context,
        )
        .await;
    });
}

// Adapter to make a WebSocket connection conform to the AgentStream trait.
pub struct WebSocketStreamAdapter {
    receiver: SplitStream<WebSocket>,
    sender: Arc<Mutex<SplitSink<WebSocket, Message>>>,
}

// Implementation of the Stream trait for our adapter.
impl Stream for WebSocketStreamAdapter {
    type Item = Result<MessageToServer, tonic::Status>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match Pin::new(&mut self.receiver).poll_next(cx) {
                Poll::Ready(Some(Ok(Message::Binary(bin)))) => {
                    let msg = MessageToServer::decode(bin.as_ref()).map_err(|e| {
                        tonic::Status::internal(format!("Protobuf decode error: {e}"))
                    });
                    return Poll::Ready(Some(msg));
                }
                Poll::Ready(Some(Ok(Message::Close(_)))) => {
                    info!("WebSocket connection closed by agent.");
                    return Poll::Ready(None);
                }
                Poll::Ready(Some(Err(e))) => {
                    warn!("WebSocket receive error: {}", e);
                    return Poll::Ready(Some(Err(tonic::Status::internal(format!(
                        "WebSocket error: {e}"
                    )))));
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
                // Ignore other message types like Text, Ping, Pong
                Poll::Ready(Some(Ok(_))) => continue,
            }
        }
    }
}

// Implementation of the Sink trait for our adapter.
impl Sink<MessageToAgent> for WebSocketStreamAdapter {
    type Error = tonic::Status;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut sender = self
            .sender
            .try_lock()
            .expect("WebSocket sender lock failed in poll_ready");
        Pin::new(&mut *sender)
            .poll_ready(cx)
            .map_err(|e| tonic::Status::internal(format!("WebSocket sink error: {e}")))
    }

    fn start_send(self: Pin<&mut Self>, item: MessageToAgent) -> Result<(), Self::Error> {
        let mut buf = Vec::new();
        item.encode(&mut buf)
            .map_err(|e| tonic::Status::internal(format!("Protobuf encode error: {e}")))?;

        let mut sender = self
            .sender
            .try_lock()
            .expect("WebSocket sender lock failed in start_send");
        Pin::new(&mut *sender)
            .start_send(Message::Binary(buf.into()))
            .map_err(|e| tonic::Status::internal(format!("WebSocket send error: {e}")))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut sender = self
            .sender
            .try_lock()
            .expect("WebSocket sender lock failed in poll_flush");
        Pin::new(&mut *sender)
            .poll_flush(cx)
            .map_err(|e| tonic::Status::internal(format!("WebSocket flush error: {e}")))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut sender = self
            .sender
            .try_lock()
            .expect("WebSocket sender lock failed in poll_close");
        Pin::new(&mut *sender)
            .poll_close(cx)
            .map_err(|e| tonic::Status::internal(format!("WebSocket close error: {e}")))
    }
}

// Make the adapter conform to our generic AgentStream trait.
impl AgentStream for WebSocketStreamAdapter {}
