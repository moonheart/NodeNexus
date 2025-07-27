use sea_orm::DatabaseConnection;
use std::sync::{mpsc as std_mpsc, Arc}; // Use std::sync::mpsc
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};

use super::agent_state::{ConnectedAgents, LiveServerDataCache};
use super::core_services::AgentStreamContext;
use super::handlers::handle_connection;
use crate::db::duckdb_service::DuckDbPool;
use crate::db::entities::performance_metric;
use crate::web::models::websocket_models::WsMessage;

#[derive(Clone)]
pub struct MyAgentCommService {
    pub connected_agents: Arc<Mutex<ConnectedAgents>>,
    pub db_pool: Arc<DatabaseConnection>,
    pub duckdb_pool: DuckDbPool,
    pub live_server_data_cache: LiveServerDataCache,
    pub ws_data_broadcaster_tx: broadcast::Sender<WsMessage>,
    pub update_trigger_tx: mpsc::Sender<()>,
    pub metric_sender: mpsc::Sender<performance_metric::Model>,
    // The service now holds the sender for DuckDB metrics.
    pub duckdb_metric_sender: std_mpsc::Sender<performance_metric::Model>,
}

impl MyAgentCommService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        connected_agents: Arc<Mutex<ConnectedAgents>>,
        db_pool: Arc<DatabaseConnection>,
        duckdb_pool: DuckDbPool,
        live_server_data_cache: LiveServerDataCache,
        ws_data_broadcaster_tx: broadcast::Sender<WsMessage>,
        update_trigger_tx: mpsc::Sender<()>,
        metric_sender: mpsc::Sender<performance_metric::Model>,
        duckdb_metric_sender: std_mpsc::Sender<performance_metric::Model>,
    ) -> Self {
        Self {
            connected_agents,
            db_pool,
            duckdb_pool,
            live_server_data_cache,
            ws_data_broadcaster_tx,
            update_trigger_tx,
            metric_sender,
            duckdb_metric_sender,
        }
    }
}

#[tonic::async_trait]
impl nodenexus_common::agent_service::agent_communication_service_server::AgentCommunicationService
    for MyAgentCommService
{
    type EstablishCommunicationStreamStream =
        ReceiverStream<Result<nodenexus_common::agent_service::MessageToAgent, Status>>;

    async fn establish_communication_stream(
        &self,
        request: Request<Streaming<nodenexus_common::agent_service::MessageToServer>>,
    ) -> Result<Response<Self::EstablishCommunicationStreamStream>, Status> {
        let context = Arc::new(AgentStreamContext {
            connected_agents: self.connected_agents.clone(),
            db_pool: self.db_pool.clone(),
            duckdb_pool: self.duckdb_pool.clone(),
            ws_data_broadcaster_tx: self.ws_data_broadcaster_tx.clone(),
            update_trigger_tx: self.update_trigger_tx.clone(),
            metric_sender: self.metric_sender.clone(),
            duckdb_metric_sender: self.duckdb_metric_sender.clone(),
        });

        handle_connection(
            request.into_inner(),
            context,
        )
        .await
    }
}
