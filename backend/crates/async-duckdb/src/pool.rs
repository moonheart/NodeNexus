//! Connection pool for async-duckdb using `deadpool`.
use crate::{AsyncConnection, Error};
use deadpool::managed::{self, RecycleResult};
use std::path::{Path, PathBuf};

/// The configuration for a DuckDB connection.
#[derive(Debug, Clone)]
pub enum DuckDBConfig {
    /// Open a database at the given path.
    Path(PathBuf),
    /// Open an in-memory database.
    InMemory,
}

/// The manager for `deadpool`.
#[derive(Debug)]
pub struct Manager {
    config: DuckDBConfig,
}

impl Manager {
    /// Creates a new `Manager` with the given configuration.
    pub fn new(config: DuckDBConfig) -> Self {
        Self { config }
    }
}

impl managed::Manager for Manager {
    type Type = AsyncConnection;
    type Error = Error;

    async fn create(&self) -> Result<Self::Type, Self::Error> {
        match &self.config {
            DuckDBConfig::Path(path) => AsyncConnection::open(path).await,
            DuckDBConfig::InMemory => AsyncConnection::open_in_memory().await,
        }
    }

    async fn recycle(&self, conn: &mut Self::Type, _: &managed::Metrics) -> RecycleResult<Self::Error> {
        match conn.execute("PRAGMA version", Vec::new()).await {
            Ok(_) => Ok(()),
            Err(e) => Err(managed::RecycleError::Backend(e)),
        }
    }

    fn detach(&self, _conn: &mut Self::Type) {}
}

/// A type alias for a `deadpool` pool of `async-duckdb` connections.
pub type Pool = managed::Pool<Manager>;