use futures_util::{Sink, Stream, StreamExt};
use sea_orm::DatabaseConnection;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::{Mutex, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};
use tracing::info;

use super::core_services::{self, AgentStream};
use crate::agent_service::{MessageToAgent, MessageToServer};
use crate::server::agent_state::{AgentSender, ConnectedAgents, LiveServerDataCache};
use crate::web::models::websocket_models::WsMessage;
use tokio::sync::broadcast;

// 1. Define the GrpcStreamAdapter
pub struct GrpcStreamAdapter {
    rx: tonic::Streaming<MessageToServer>,
    tx: mpsc::Sender<Result<MessageToAgent, Status>>,
}

// 2. Implement Stream for the adapter
impl Stream for GrpcStreamAdapter {
    type Item = Result<MessageToServer, Status>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.rx.poll_next_unpin(cx)
    }
}

// 3. Implement Sink for the adapter (Corrected)
impl Sink<MessageToAgent> for GrpcStreamAdapter {
    type Error = Status;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // mpsc::Sender is always ready to accept a message until the channel is closed.
        // A more robust implementation could check `is_closed()`, but for this adapter,
        // relying on the error from `try_send` is sufficient.
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: MessageToAgent) -> Result<(), Self::Error> {
        // Use `try_send` which is synchronous and returns an error if the channel is full or closed.
        self.get_mut().tx.try_send(Ok(item)).map_err(|e| match e {
            mpsc::error::TrySendError::Full(_) => {
                Status::unavailable("gRPC stream channel is full.")
            }
            mpsc::error::TrySendError::Closed(_) => {
                Status::unavailable("gRPC stream channel is closed.")
            }
        })
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // mpsc channel does not require flushing.
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // The sender is closed when it's dropped, so we don't need to do anything special here.
        Poll::Ready(Ok(()))
    }
}

impl AgentStream for GrpcStreamAdapter {}

// 4. Simplify the handle_connection function
pub async fn handle_connection(
    in_stream: tonic::Streaming<MessageToServer>,
    connected_agents_arc: Arc<Mutex<ConnectedAgents>>,
    pool: Arc<DatabaseConnection>,
    _live_server_data_cache: LiveServerDataCache,
    ws_data_broadcaster_tx: broadcast::Sender<WsMessage>,
    update_trigger_tx: mpsc::Sender<()>,
    batch_command_manager: Arc<crate::db::services::BatchCommandManager>,
) -> Result<Response<ReceiverStream<Result<MessageToAgent, Status>>>, Status> {
    let (tx_to_agent, rx_from_server) = mpsc::channel(128);
    info!("New gRPC connection stream established. Creating adapter.");

    let adapter = GrpcStreamAdapter {
        rx: in_stream,
        tx: tx_to_agent.clone(),
    };

    let agent_sender = AgentSender::Grpc(tx_to_agent);

    // Spawn the core processing logic with the adapter
    tokio::spawn(async move {
        core_services::process_agent_stream(
            adapter,
            agent_sender,
            connected_agents_arc,
            pool,
            ws_data_broadcaster_tx,
            update_trigger_tx,
            batch_command_manager,
        )
        .await;
    });

    Ok(Response::new(ReceiverStream::from(rx_from_server)))
}
