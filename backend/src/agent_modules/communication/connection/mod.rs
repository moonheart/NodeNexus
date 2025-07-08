use crate::agent_modules::config::AgentCliConfig;
use crate::agent_service::{
    AgentConfig, MessageToAgent, MessageToServer, message_to_agent::Payload as AgentPayload,
    message_to_server::Payload as ServerPayload,
};
use futures_util::{Sink, SinkExt, Stream, StreamExt as FuturesStreamExt};
use std::error::Error;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tonic::Status;
use tracing::{error, info};

// 重新导出子模块
pub mod grpc;
pub mod websocket;

// 使用绝对路径导入子模块内容
use self::grpc::GrpcSink;
use self::websocket::WebSocketStreamAdapter;

pub struct ConnectionHandler {
    pub in_stream: Pin<Box<dyn Stream<Item = Result<MessageToAgent, Status>> + Send + Unpin>>,
    pub tx_to_server: Pin<Box<dyn Sink<MessageToServer, Error = Status> + Send + Unpin>>,
    pub initial_agent_config: AgentConfig,
    pub client_message_id_counter: Arc<AtomicU64>,
}

impl ConnectionHandler {
    pub async fn connect_and_handshake(
        agent_cli_config: &AgentCliConfig,
        initial_message_id_counter_val: u64,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        if agent_cli_config.server_address.starts_with("ws") {
            Self::connect_and_handshake_ws(agent_cli_config, initial_message_id_counter_val).await
        } else {
            Self::connect_and_handshake_grpc(agent_cli_config, initial_message_id_counter_val).await
        }
    }

    async fn connect_and_handshake_ws(
        agent_cli_config: &AgentCliConfig,
        initial_message_id_counter_val: u64,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        info!("Attempting to connect to WebSocket server");
        let base_url = agent_cli_config.server_address.trim_end_matches('/');
        let full_url = if !agent_cli_config.server_address.contains("/ws/agent") {
            format!("{base_url}/ws/agent")
        } else {
            agent_cli_config.server_address.clone()
        };

        info!(url = %full_url, "Connecting to WebSocket URL");
        let (ws_stream, _) = tokio_tungstenite::connect_async(&full_url).await?;
        info!("Successfully connected to WebSocket endpoint.");

        let mut adapter = WebSocketStreamAdapter {
            ws_stream: Arc::new(Mutex::new(ws_stream)),
        };

        let handshake_payload = super::handshake::create_handshake_payload().await;
        let client_message_id_counter = Arc::new(AtomicU64::new(initial_message_id_counter_val));
        let handshake_msg_id = client_message_id_counter.fetch_add(1, Ordering::SeqCst);

        let handshake_msg = MessageToServer {
            client_message_id: handshake_msg_id,
            payload: Some(ServerPayload::AgentHandshake(handshake_payload)),
            vps_db_id: agent_cli_config.vps_id,
            agent_secret: agent_cli_config.agent_secret.clone(),
        };

        adapter
            .send(handshake_msg)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?;
        info!("Handshake message sent to server. Waiting for response...");

        if let Some(Ok(response_msg)) = adapter.next().await {
            if let Some(AgentPayload::ServerHandshakeAck(ack)) = response_msg.payload {
                if ack.authentication_successful {
                    info!("Authenticated successfully via WebSocket.");
                    Ok(Self {
                        in_stream: Box::pin(adapter.clone()),
                        tx_to_server: Box::pin(adapter),
                        initial_agent_config: ack.initial_config.unwrap_or_default(),
                        client_message_id_counter,
                    })
                } else {
                    let err_msg = format!(
                        "Authentication failed: {}. This is a critical error. Agent will not retry automatically for auth failures.",
                        ack.error_message
                    );
                    error!(error_message = %err_msg, "Handshake authentication failed.");
                    Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        ack.error_message,
                    )) as Box<dyn Error + Send + Sync>)
                }
            } else {
                error!("Unexpected first message from server (not HandshakeAck).");
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Unexpected first message from server",
                )) as Box<dyn Error + Send + Sync>)
            }
        } else {
            error!("Server closed stream during handshake.");
            Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "Server closed stream during handshake",
            )) as Box<dyn Error + Send + Sync>)
        }
    }

    async fn connect_and_handshake_grpc(
        agent_cli_config: &AgentCliConfig,
        initial_message_id_counter_val: u64,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        info!("Attempting to connect to gRPC server");

        let tls = tonic::transport::ClientTlsConfig::new().with_native_roots();

        let channel =
            tonic::transport::Endpoint::from_shared(agent_cli_config.server_address.clone())?
                .tls_config(tls)?
                .http2_keep_alive_interval(std::time::Duration::from_secs(10))
                .keep_alive_timeout(std::time::Duration::from_secs(30))
                .keep_alive_while_idle(true)
                .connect()
                .await
                .map_err(|e| {
                    error!(error = %e, "Failed to connect to gRPC endpoint with TLS.");
                    Box::new(e) as Box<dyn Error + Send + Sync>
                })?;

        let mut client = crate::agent_service::agent_communication_service_client::AgentCommunicationServiceClient::new(channel);
        info!("Successfully connected to gRPC endpoint.");
        let (tx_to_server, rx_for_stream) = mpsc::channel(128);

        let stream_response_future = client.establish_communication_stream(
            tokio_stream::wrappers::ReceiverStream::new(rx_for_stream),
        );

        info!("Continue without wait for establish_communication_stream result.");

        let handshake_payload = super::handshake::create_handshake_payload().await;

        let client_message_id_counter = Arc::new(AtomicU64::new(initial_message_id_counter_val));
        let handshake_msg_id = client_message_id_counter.fetch_add(1, Ordering::SeqCst);

        tx_to_server
            .send(MessageToServer {
                client_message_id: handshake_msg_id,
                payload: Some(ServerPayload::AgentHandshake(handshake_payload)),
                vps_db_id: agent_cli_config.vps_id,
                agent_secret: agent_cli_config.agent_secret.clone(),
            })
            .await
            .map_err(|e| {
                error!(error = %e, "Failed to send handshake message.");
                Box::new(e) as Box<dyn Error + Send + Sync>
            })?;

        info!("Handshake message sent to server. Waiting for response...");

        let mut in_stream = stream_response_future
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error + Send + Sync>)?
            .into_inner();

        match in_stream.next().await {
            Some(Ok(response_msg)) => {
                if let Some(AgentPayload::ServerHandshakeAck(ack)) = response_msg.payload {
                    if ack.authentication_successful {
                        info!("Authenticated successfully via gRPC.");
                        let grpc_sink = GrpcSink { tx: tx_to_server };
                        Ok(Self {
                            in_stream: Box::pin(in_stream),
                            tx_to_server: Box::pin(grpc_sink),
                            initial_agent_config: ack.initial_config.unwrap_or_default(),
                            client_message_id_counter,
                        })
                    } else {
                        let err_msg = format!(
                            "Authentication failed: {}. This is a critical error. Agent will not retry automatically for auth failures.",
                            ack.error_message
                        );
                        error!(error_message = %err_msg, "Handshake authentication failed.");
                        Err(Box::new(std::io::Error::new(
                            std::io::ErrorKind::PermissionDenied,
                            ack.error_message,
                        )) as Box<dyn Error + Send + Sync>)
                    }
                } else {
                    error!("Unexpected first message from server (not HandshakeAck).");
                    Err(Box::new(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Unexpected first message from server",
                    )) as Box<dyn Error + Send + Sync>)
                }
            }
            Some(Err(status)) => {
                error!(?status, "Error receiving handshake response.");
                Err(Box::new(status) as Box<dyn Error + Send + Sync>)
            }
            None => {
                error!("Server closed stream during handshake.");
                Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::ConnectionAborted,
                    "Server closed stream during handshake",
                )) as Box<dyn Error + Send + Sync>)
            }
        }
    }

    pub fn split_for_tasks(
        mut self,
    ) -> (
        Pin<Box<dyn Stream<Item = Result<MessageToAgent, Status>> + Send + Unpin>>,
        mpsc::Sender<MessageToServer>,
        Arc<AtomicU64>,
        AgentConfig,
    ) {
        let (tx, mut rx) = mpsc::channel(128);

        tokio::spawn(async move {
            while let Some(item) = rx.recv().await {
                if self.tx_to_server.send(item).await.is_err() {
                    error!("Failed to send message to server through sink.");
                    break;
                }
            }
        });

        (
            self.in_stream,
            tx,
            self.client_message_id_counter,
            self.initial_agent_config,
        )
    }

    pub fn get_id_provider_closure(
        counter: Arc<AtomicU64>,
    ) -> impl Fn() -> u64 + Send + Sync + Clone + 'static {
        move || counter.fetch_add(1, Ordering::SeqCst)
    }
}
