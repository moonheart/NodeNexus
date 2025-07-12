use sea_orm::DatabaseConnection; // Replaced PgPool
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};

use super::agent_state::{ConnectedAgents, LiveServerDataCache};
use super::core_services::AgentStreamContext;
use super::handlers::handle_connection;
use crate::db::entities::{performance_disk_usage, performance_metric};
use crate::db::services::BatchCommandManager;
use crate::web::models::websocket_models::WsMessage;
use tokio::sync::broadcast; // Added BatchCommandManager

#[derive(Debug)]
pub struct MyAgentCommService {
    pub connected_agents: Arc<Mutex<ConnectedAgents>>,
    pub db_pool: Arc<DatabaseConnection>, // Changed PgPool to DatabaseConnection
    pub live_server_data_cache: LiveServerDataCache,
    pub ws_data_broadcaster_tx: broadcast::Sender<WsMessage>,
    pub update_trigger_tx: mpsc::Sender<()>,
    pub batch_command_manager: Arc<BatchCommandManager>, // Added BatchCommandManager
    pub metric_sender: mpsc::Sender<(
        performance_metric::Model,
        Vec<performance_disk_usage::Model>,
    )>,
}

impl MyAgentCommService {
    pub fn new(
        connected_agents: Arc<Mutex<ConnectedAgents>>,
        db_pool: Arc<DatabaseConnection>, // Changed PgPool to DatabaseConnection
        live_server_data_cache: LiveServerDataCache,
        ws_data_broadcaster_tx: broadcast::Sender<WsMessage>,
        update_trigger_tx: mpsc::Sender<()>,
        batch_command_manager: Arc<BatchCommandManager>, // Added BatchCommandManager
        metric_sender: mpsc::Sender<(
            performance_metric::Model,
            Vec<performance_disk_usage::Model>,
        )>,
    ) -> Self {
        Self {
            connected_agents,
            db_pool,
            live_server_data_cache,
            ws_data_broadcaster_tx,
            update_trigger_tx,
            batch_command_manager, // Store BatchCommandManager
            metric_sender,
        }
    }
}

#[tonic::async_trait]
impl crate::agent_service::agent_communication_service_server::AgentCommunicationService
    for MyAgentCommService
{
    type EstablishCommunicationStreamStream =
        ReceiverStream<Result<crate::agent_service::MessageToAgent, Status>>;

    async fn establish_communication_stream(
        &self,
        request: Request<Streaming<crate::agent_service::MessageToServer>>,
    ) -> Result<Response<Self::EstablishCommunicationStreamStream>, Status> {
        let context = Arc::new(AgentStreamContext {
            connected_agents: self.connected_agents.clone(),
            db_pool: self.db_pool.clone(),
            ws_data_broadcaster_tx: self.ws_data_broadcaster_tx.clone(),
            update_trigger_tx: self.update_trigger_tx.clone(),
            batch_command_manager: self.batch_command_manager.clone(),
            metric_sender: self.metric_sender.clone(),
        });

        handle_connection(
            request.into_inner(),
            context,
        )
        .await
    }
}
