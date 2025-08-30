//! An asynchronous wrapper for the `duckdb` crate.
//!
//! This crate provides an asynchronous API for interacting with a DuckDB database.
//! It is designed to be used with modern async runtimes like Tokio and async-std.
//!
//! # Architecture
//!
//! The core of this crate is a background worker thread that manages a synchronous
//! `duckdb::Connection`. All database operations are sent to this worker thread
//! via an asynchronous channel, and the results are returned via a one-shot channel.
//! This design ensures that blocking FFI calls to the DuckDB C API do not block
//! the async runtime.
//!
//! # Example
//!
//! ```rust,no_run
//! use async_duckdb::{AsyncConnection, Result};
//! use duckdb::params;
//! use futures_util::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let conn = AsyncConnection::open_in_memory().await?;
//!
//!     conn.execute(
//!         "CREATE TABLE person (id INTEGER, name TEXT, data BLOB)",
//!         params![],
//!     )
//!     .await?;
//!
//!     let tx = conn.transaction().await?;
//!     tx.execute(
//!         "INSERT INTO person (name) VALUES (?)",
//!         params!["John Doe"],
//!     )
//!     .await?;
//!     tx.commit().await?;
//!
//!     let mut stmt = conn.prepare("SELECT name FROM person").await?;
//!     let mut rows = stmt.query(params![]).await?;
//!
//!     while let Some(row) = rows.next().await {
//!         let row = row?;
//!         let name: String = row.get(0)?;
//!         println!("Found person: {}", name);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # Connection Pool
//!
//! This crate provides a connection pool implementation for `deadpool`.
//!
//! ```rust,no_run
//! use async_duckdb::pool::{self, DuckDBConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!    let config = DuckDBConfig::Path("my-database.db".into());
//!    let manager = pool::Manager::new(config);
//!    let pool = pool::Pool::builder(manager).max_size(16).build().unwrap();
//!
//!    let mut conn = pool.get().await.unwrap();
//!    conn.execute("CREATE TABLE foo (id INTEGER)", []).await.unwrap();
//! }
//! ```

pub mod error;
mod worker;
pub mod connection;
pub mod statement;
pub mod row;
pub mod transaction;
pub mod pool;

pub use connection::AsyncConnection;
pub use statement::AsyncStatement;
pub use row::OwnedRow;
pub use transaction::AsyncTransaction;
pub use pool::{DuckDBConfig, Manager, Pool};
use duckdb::types::{ToSqlOutput, Value};
use duckdb::ToSql;
pub use error::{Error, Result};

/// A trait for types that can be converted into a `Vec<duckdb::types::Value>`.
///
/// This is used to ergonomically pass parameters to execute methods.
pub trait IntoParams {
    fn into_params(self) -> Result<Vec<Value>>;
}

fn to_sql_to_value(p: &dyn ToSql) -> Result<Value> {
    match p.to_sql().map_err(Error::DuckDB)? {
        ToSqlOutput::Owned(v) => Ok(v),
        ToSqlOutput::Borrowed(v_ref) => Ok(v_ref.to_owned()),
        _ => Err(Error::DuckDB(duckdb::Error::ToSqlConversionFailure(
            "unsupported ToSqlOutput variant".into(),
        ))),
    }
}

// Implement IntoParams for the type that `params!` macro produces.
impl<'a> IntoParams for &'a [&'a dyn ToSql] {
    fn into_params(self) -> Result<Vec<Value>> {
        let mut params = Vec::with_capacity(self.len());
        for p in self {
            params.push(to_sql_to_value(*p)?);
        }
        Ok(params)
    }
}

impl IntoParams for Vec<Value> {
    fn into_params(self) -> Result<Vec<Value>> {
        Ok(self)
    }
}
