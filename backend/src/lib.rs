pub mod agent_service {
    tonic::include_proto!("agent_service");
}

// Removed command_service module as its contents are now part of agent_service
// pub mod command_service {
//     tonic::include_proto!("command_service");
// }

pub mod server;

pub mod agent_modules;
pub mod axum_embed;
pub mod db;
pub mod services;
pub mod web;

pub mod alerting; // Added alerting module
pub mod notifications;
pub mod version;
