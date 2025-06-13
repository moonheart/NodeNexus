pub mod agent_service {
    tonic::include_proto!("agent_service");
}

// Removed command_service module as its contents are now part of agent_service
// pub mod command_service {
//     tonic::include_proto!("command_service");
// }

pub mod server;

pub mod db;
pub mod http_server;
pub mod agent_modules;
pub mod websocket_models; // Added websocket_models module

pub mod notifications;
pub mod alerting; // Added alerting module
