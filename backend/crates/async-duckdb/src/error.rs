use duckdb::types::FromSqlError;
use thiserror::Error;

/// The primary error type for `async-duckdb`.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
    /// An error originating from the underlying `duckdb` crate.
    #[error("DuckDB error: {0}")]
    DuckDB(#[from] duckdb::Error),

    /// An error that occurred when converting a SQL value to a Rust type.
    #[error("Failed to convert from SQL: {0}")]
    FromSql(#[from] FromSqlError),

    /// The background worker thread has panicked and is no longer running.
    ///
    /// This error indicates that the connection is no longer usable.
    #[error("Background worker thread has crashed")]
    WorkerCrashed,

    /// The command channel to the background worker is closed.
    ///
    /// This error indicates that the connection is no longer usable.
    #[error("Command channel to worker is closed")]
    ChannelClosed,

    /// An invalid statement ID was provided.
    #[error("Invalid statement ID: {0}")]
    InvalidStatementId(super::worker::StatementId),
}

/// A convenience type alias for a `Result` with `async_duckdb::Error`.
pub type Result<T, E = Error> = std::result::Result<T, E>;