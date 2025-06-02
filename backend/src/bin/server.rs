// 主入口文件
use backend::server::agent_state::ConnectedAgents;
use backend::server::service::MyAgentCommService;
use tonic::transport::Server as TonicServer; // Renamed to avoid conflict if axum::Server is used directly
use backend::agent_service::agent_communication_service_server::AgentCommunicationServiceServer;
use backend::http_server; // Import the new http_server module

use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use dotenv::dotenv;
use std::env;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> { // Added Send + Sync for tokio::spawn
    dotenv().ok(); // Load .env file

    // --- Database Pool Setup ---
    let database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env file");
    let db_pool = PgPoolOptions::new()
        .max_connections(10) // Adjust as needed
        .connect(&database_url)
        .await
        .expect("Failed to create database pool.");
    
    // --- gRPC Server Setup ---
    let grpc_addr: SocketAddr = "0.0.0.0:50051".parse()?;
    let connected_agents = ConnectedAgents::new();
    // If MyAgentCommService needs Arc<Mutex<ConnectedAgents>>, ensure it's cloned if necessary.
    // For now, assuming it takes ownership or a direct reference that's fine for one service instance.
    let agent_comm_service = MyAgentCommService::new(connected_agents.clone(), Arc::from(db_pool.clone()));
    
    let grpc_service = AgentCommunicationServiceServer::new(agent_comm_service);
    let grpc_server_future = TonicServer::builder()
        .add_service(grpc_service)
        .serve(grpc_addr);

    println!("gRPC AgentCommunicationService listening on {}", grpc_addr);

    // --- Axum HTTP Server Setup ---
    let http_addr: SocketAddr = "0.0.0.0:8080".parse()?; // Different port for HTTP
    // Pass the db_pool to the http_server run function
    let http_server_future = http_server::run_http_server(db_pool.clone(), http_addr);

    // Run both servers concurrently
    let grpc_handle = tokio::spawn(async move {
        if let Err(e) = grpc_server_future.await {
            eprintln!("gRPC server error: {}", e);
        }
    });

    let http_handle = tokio::spawn(async move {
        if let Err(e) = http_server_future.await {
            eprintln!("HTTP server error: {}", e);
        }
    });

    let (_grpc_result, _http_result) = tokio::try_join!(grpc_handle, http_handle)?;
    Ok(())
}
