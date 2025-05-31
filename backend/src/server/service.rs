use tonic::{Request, Response, Status, Streaming};
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::ReceiverStream;
use std::sync::Arc;
use uuid::Uuid;

use super::agent_state::ConnectedAgents;
use super::handlers::handle_connection;

#[derive(Debug)]
pub struct MyAgentCommService {
    pub connected_agents: Arc<Mutex<ConnectedAgents>>,
}

impl MyAgentCommService {
    pub fn new(connected_agents: Arc<Mutex<ConnectedAgents>>) -> Self {
        Self { connected_agents }
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
        ).await
    }
}