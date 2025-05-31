// 主入口文件


use backend::server::agent_state::ConnectedAgents;
use backend::server::service::MyAgentCommService;
use tonic::transport::Server;
use backend::agent_service::agent_communication_service_server::AgentCommunicationServiceServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50051".parse()?;
    let connected_agents = ConnectedAgents::new(); // This returns Arc<Mutex<ConnectedAgents>>
    let agent_comm_service = MyAgentCommService::new(connected_agents);

    println!("AgentCommunicationService backend listening on {}", addr);

    Server::builder()
        .add_service(AgentCommunicationServiceServer::new(agent_comm_service))
        .serve(addr)
        .await?;

    Ok(())
}
