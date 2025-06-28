use crate::agent_service::MessageToServer;
use futures_util::Sink;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;
use tonic::Status;

pub struct GrpcSink {
    pub tx: mpsc::Sender<MessageToServer>,
}

impl Sink<MessageToServer> for GrpcSink {
    type Error = Status;

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: MessageToServer) -> Result<(), Self::Error> {
        self.get_mut()
            .tx
            .try_send(item)
            .map_err(|e| Status::internal(e.to_string()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}
