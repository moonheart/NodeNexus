pub mod agent_service {
    tonic::include_proto!("agent_service");
}

pub mod server;

pub mod db;
pub mod http_server;
pub mod agent_modules;
pub mod websocket_models; // Added websocket_models module
