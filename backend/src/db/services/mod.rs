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
pub mod vps_service;

// Re-export all public functions and structs from the sub-modules
// to make them accessible directly under `crate::db::services::*`.
pub use performance_service::*;
pub use settings_service::*;
pub use tag_service::*;
pub use user_service::*;
pub use vps_service::*;