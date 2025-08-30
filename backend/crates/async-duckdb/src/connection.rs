use crate::error::{Error, Result};
use crate::worker::{Command, Worker};
use duckdb::Connection as DuckDBConnection;
use flume::Sender;
use std::path::Path;
use std::thread;

/// An asynchronous connection to a DuckDB database.
///
/// This is the main entry point for interacting with a DuckDB database.
/// It is created by calling [`AsyncConnection::open`] or [`AsyncConnection::open_in_memory`].
#[derive(Debug)]
pub struct AsyncConnection {
    cmd_tx: Sender<Command>,
}

use crate::transaction::AsyncTransaction;

impl AsyncConnection {
    /// Begins a new transaction.
    ///
    /// The transaction is started immediately. If the transaction is not committed,
    /// it will be rolled back when it is dropped.
    pub async fn transaction(&self) -> Result<AsyncTransaction> {
        let (responder, rx) = futures_channel::oneshot::channel();
        self.cmd_tx
            .send_async(Command::BeginTransaction { responder })
            .await
            .map_err(|_| Error::ChannelClosed)?;

        let tx_cmd = rx.await.map_err(|_| Error::WorkerCrashed)??;

        Ok(AsyncTransaction {
            tx_cmd,
            committed: false,
        })
    }

    /// Open a new in-memory DuckDB database and connect to it.
    pub async fn open_in_memory() -> Result<Self> {
        Self::open_internal(DuckDBConnection::open_in_memory).await
    }

    /// Open a new or existing DuckDB database file and connect to it.
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_owned();
        Self::open_internal(|| DuckDBConnection::open(path)).await
    }

    /// Internal implementation for opening a connection.
    async fn open_internal<F>(open_fn: F) -> Result<Self>
    where
        F: FnOnce() -> duckdb::Result<DuckDBConnection> + Send + 'static,
    {
        let (cmd_tx, cmd_rx) = flume::unbounded();
        let (init_tx, init_rx) = futures_channel::oneshot::channel();

        thread::spawn(move || {
            let conn_result = open_fn();
            match conn_result {
                Ok(conn) => {
                    if init_tx.send(Ok(())).is_err() {
                        // Receiver was dropped, so the awaiting task is gone.
                        return;
                    }
                    let worker = Worker::new(conn);
                    worker.run(cmd_rx);
                }
                Err(e) => {
                    // Failed to open connection, send the error back.
                    let _ = init_tx.send(Err(Error::DuckDB(e)));
                }
            }
        });

        // Wait for the worker thread to confirm successful initialization.
        init_rx.await.map_err(|_| Error::WorkerCrashed)??;

        Ok(Self { cmd_tx })
    }

    /// Creates a new prepared statement.
    pub async fn prepare<S: Into<String>>(&self, sql: S) -> Result<crate::statement::AsyncStatement<'_>> {
        let (responder, rx) = futures_channel::oneshot::channel();
        self.cmd_tx
            .send_async(Command::Prepare {
                sql: sql.into(),
                responder,
            })
            .await
            .map_err(|_| Error::ChannelClosed)?;

        let stmt_id = rx.await.map_err(|_| Error::WorkerCrashed)??;

        Ok(crate::statement::AsyncStatement {
            id: stmt_id,
            cmd_tx: self.cmd_tx.clone(),
            _marker: std::marker::PhantomData,
        })
    }

    /// Execute a statement that does not return any rows.
    ///
    /// On success, returns the number of rows that were changed or inserted or
    /// deleted.
    pub async fn execute<S, P>(&self, sql: S, params: P) -> Result<usize>
    where
        S: Into<String>,
        P: crate::IntoParams,
    {
        let (responder, rx) = futures_channel::oneshot::channel();
        self.cmd_tx
            .send_async(Command::Execute {
                sql: sql.into(),
                params: params.into_params()?,
                responder,
            })
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::WorkerCrashed)?
    }
}

impl Drop for AsyncConnection {
    fn drop(&mut self) {
        // The channel is closed by dropping the sender.
        // The worker thread will detect this and shut down gracefully.
    }
}