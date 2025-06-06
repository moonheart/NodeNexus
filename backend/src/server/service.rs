use sqlx::PgPool;
use tonic::{Request, Response, Status, Streaming};
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;
use std::sync::Arc;

use super::agent_state::{ConnectedAgents, LiveServerDataCache};
use crate::websocket_models::FullServerListPush;
use tokio::sync::broadcast;
use super::handlers::handle_connection;

#[derive(Debug)]
pub struct MyAgentCommService {
    pub connected_agents: Arc<Mutex<ConnectedAgents>>,
    pub db_pool: Arc<PgPool>,
    pub live_server_data_cache: LiveServerDataCache,
    pub ws_data_broadcaster_tx: broadcast::Sender<Arc<FullServerListPush>>,
}

impl MyAgentCommService {
    pub fn new(
        connected_agents: Arc<Mutex<ConnectedAgents>>,
        db_pool: Arc<PgPool>,
        live_server_data_cache: LiveServerDataCache,
        ws_data_broadcaster_tx: broadcast::Sender<Arc<FullServerListPush>>,
    ) -> Self {
        Self {
            connected_agents,
            db_pool,
            live_server_data_cache,
            ws_data_broadcaster_tx,
        }
    }
}

#[tonic::async_trait]
impl crate::agent_service::agent_communication_service_server::AgentCommunicationService for MyAgentCommService {
    type EstablishCommunicationStreamStream = ReceiverStream<Result<crate::agent_service::MessageToAgent, Status>>;

    async fn establish_communication_stream(
        &self,
        request: Request<Streaming<crate::agent_service::MessageToServer>>,
    ) -> Result<Response<Self::EstablishCommunicationStreamStream>, Status> {
        handle_connection(
            request.into_inner(),
            self.connected_agents.clone(),
            self.db_pool.clone(),
            self.live_server_data_cache.clone(),
            self.ws_data_broadcaster_tx.clone(),
        ).await
    }
}