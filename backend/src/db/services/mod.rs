//! The `services` module provides a high-level API for interacting with the database.
//! It encapsulates all the SQL logic and data access patterns, allowing the rest of
//! the application (e.g., HTTP handlers, websocket services) to work with domain models
//! without needing to know about the underlying database schema or queries.
//!
//! This module is organized into sub-modules, each responsible for a specific
//! domain entity (like users, VPS, tags, etc.), promoting separation of concerns.
//! All public functions from these sub-modules are re-exported here for convenient access
//! under the `crate::db::services::` path.

// Declare the sub-modules for each service area.
pub mod performance_service;
pub mod settings_service;
pub mod tag_service;
pub mod user_service;
pub mod vps_core_service; // Added vps_core_service module
pub mod vps_detail_service; // Added vps_detail_service module
pub mod vps_renewal_service; // Added vps_renewal_service module
pub mod vps_traffic_service; // Added vps_traffic_service module
pub mod vps_service; // This now re-exports from the above vps_* modules
pub mod alert_service; // Added alert_service module
pub mod batch_command_service;
pub mod service_monitor_service;

// Re-export all public functions and structs from the sub-modules
// to make them accessible directly under `crate::db::services::*`.
pub use performance_service::*;
pub use settings_service::*;
pub use tag_service::*;
pub use user_service::*;
pub use vps_service::*;
pub use alert_service::*; // Re-export alert_service
pub use batch_command_service::*;
pub use service_monitor_service::*;