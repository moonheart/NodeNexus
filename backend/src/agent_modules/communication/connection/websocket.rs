use crate::agent_service::{MessageToAgent, MessageToServer};
use futures_util::{Sink, Stream};
use prost::Message as ProstMessage;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;
use tonic::Status;
use tracing::{info, warn};

#[derive(Clone)]
pub struct WebSocketStreamAdapter {
    pub ws_stream: Arc<
        Mutex<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
    >,
}

impl Stream for WebSocketStreamAdapter {
    type Item = Result<MessageToAgent, Status>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut stream_guard = match self.ws_stream.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        };
        match Pin::new(&mut *stream_guard).poll_next(cx) {
            Poll::Ready(Some(Ok(WsMessage::Binary(bin)))) => {
                let msg = MessageToAgent::decode(bin.as_ref())
                    .map_err(|e| Status::internal(format!("Protobuf decode error: {e}")));
                Poll::Ready(Some(msg))
            }
            Poll::Ready(Some(Ok(WsMessage::Close(_)))) => {
                info!("WebSocket connection closed by server.");
                Poll::Ready(None)
            }
            Poll::Ready(Some(Err(e))) => {
                warn!("WebSocket receive error: {}", e);
                Poll::Ready(Some(Err(Status::internal(format!("WebSocket error: {e}")))))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
            _ => Poll::Pending,
        }
    }
}

impl Sink<MessageToServer> for WebSocketStreamAdapter {
    type Error = Status;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut stream = match self.ws_stream.try_lock() {
            Ok(s) => s,
            Err(_) => return Poll::Pending,
        };
        Pin::new(&mut *stream)
            .poll_ready(cx)
            .map_err(|e| Status::internal(e.to_string()))
    }

    fn start_send(self: Pin<&mut Self>, item: MessageToServer) -> Result<(), Self::Error> {
        let mut buf = Vec::new();
        item.encode(&mut buf)
            .map_err(|e| Status::internal(format!("Protobuf encode error: {e}")))?;
        let mut stream = self
            .ws_stream
            .try_lock()
            .map_err(|_| Status::unavailable("WebSocket stream is busy, could not send"))?;
        Pin::new(&mut *stream)
            .start_send(WsMessage::Binary(buf.into()))
            .map_err(|e| Status::internal(e.to_string()))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut stream = match self.ws_stream.try_lock() {
            Ok(s) => s,
            Err(_) => return Poll::Pending,
        };
        Pin::new(&mut *stream)
            .poll_flush(cx)
            .map_err(|e| Status::internal(e.to_string()))
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let mut stream = match self.ws_stream.try_lock() {
            Ok(s) => s,
            Err(_) => return Poll::Pending,
        };
        Pin::new(&mut *stream)
            .poll_close(cx)
            .map_err(|e| Status::internal(e.to_string()))
    }
}
