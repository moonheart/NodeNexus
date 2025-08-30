use crate::Result;
use duckdb::{types::Value, Connection};
use flume::{Receiver, Sender};
use futures_channel::oneshot;
use std::collections::HashMap;
use std::sync::Arc;

// ID types
pub type StatementId = usize;
pub type QueryId = usize;

/// A command sent to the background worker thread.
pub(crate) enum Command {
    /// Execute a query that does not return rows.
    Execute {
        sql: String,
        params: Vec<Value>,
        responder: oneshot::Sender<Result<usize>>,
    },
    /// Prepare a statement.
    Prepare {
        sql: String,
        responder: oneshot::Sender<Result<StatementId>>,
    },
    /// Execute a prepared statement.
    ExecuteStatement {
        stmt_id: StatementId,
        params: Vec<Value>,
        responder: oneshot::Sender<Result<usize>>,
    },
    /// Drop a prepared statement.
    DropStatement {
        stmt_id: StatementId,
    },
    /// Start a query that returns rows.
    QueryStatement {
        stmt_id: StatementId,
        params: Vec<Value>,
        responder: oneshot::Sender<Result<QueryId>>,
    },
    /// Fetch a row from a query.
    FetchRow {
        query_id: QueryId,
        responder: oneshot::Sender<Result<Option<crate::row::OwnedRow>>>,
    },
    /// Drop a query.
    DropQuery {
        query_id: QueryId,
    },
    /// Begin a transaction.
    BeginTransaction {
        responder: oneshot::Sender<Result<Sender<TransactionCommand>>>,
    },
}

/// A command sent to the background worker thread for a transaction.
pub(crate) enum TransactionCommand {
    /// Execute a query that does not return rows.
    Execute {
        sql: String,
        params: Vec<Value>,
        responder: oneshot::Sender<Result<usize>>,
    },
    /// Commit the transaction.
    Commit {
        responder: oneshot::Sender<Result<()>>,
    },
    /// Rollback the transaction.
    Rollback {
        responder: oneshot::Sender<Result<()>>,
    },
}

/// The background worker that executes synchronous DuckDB operations.
pub(crate) struct Worker {
    conn: Connection,
    statements: HashMap<StatementId, duckdb::Statement<'static>>,
    queries: HashMap<QueryId, (duckdb::Rows<'static>, Arc<HashMap<String, usize>>)>,
    next_stmt_id: StatementId,
    next_query_id: QueryId,
}

impl Worker {
    /// Creates a new worker.
    pub(crate) fn new(conn: Connection) -> Self {
        Self {
            conn,
            statements: HashMap::new(),
            queries: HashMap::new(),
            next_stmt_id: 0,
            next_query_id: 0,
        }
    }

    /// Runs the worker's command loop.
    pub(crate) fn run(mut self, rx: flume::Receiver<Command>) {
        while let Ok(cmd) = rx.recv() {
            match cmd {
                Command::Execute {
                    sql,
                    params,
                    responder,
                } => {
                    let params_as_ref: Vec<&dyn duckdb::ToSql> =
                        params.iter().map(|p| p as &dyn duckdb::ToSql).collect();
                    let result = self.conn.execute(&sql, params_as_ref.as_slice()).map_err(From::from);
                    let _ = responder.send(result);
                }
                Command::Prepare { sql, responder } => {
                    let result = self.conn.prepare(&sql).map(|stmt| {
                        let id = self.next_stmt_id;
                        self.next_stmt_id += 1;
                        // SAFETY: The statement is stored in the same struct as the connection.
                        // It will not outlive the connection.
                        let static_stmt = unsafe { std::mem::transmute::<duckdb::Statement<'_>, duckdb::Statement<'_>>(stmt) };
                        self.statements.insert(id, static_stmt);
                        id
                    });
                    let _ = responder.send(result.map_err(From::from));
                }
                Command::ExecuteStatement {
                    stmt_id,
                    params,
                    responder,
                } => {
                    let result = self
                        .statements
                        .get_mut(&stmt_id)
                        .ok_or(crate::Error::InvalidStatementId(stmt_id))
                        .and_then(|stmt| {
                            let params_as_ref: Vec<&dyn duckdb::ToSql> =
                                params.iter().map(|p| p as &dyn duckdb::ToSql).collect();
                            stmt.execute(params_as_ref.as_slice()).map_err(From::from)
                        });
                    let _ = responder.send(result);
                }
                Command::DropStatement { stmt_id } => {
                    self.statements.remove(&stmt_id);
                }
                Command::QueryStatement {
                    stmt_id,
                    params,
                    responder,
                } => {
                    let result = self
                        .statements
                        .get_mut(&stmt_id)
                        .ok_or(crate::Error::InvalidStatementId(stmt_id))
                        .and_then(|stmt| {
                            let params_as_ref: Vec<&dyn duckdb::ToSql> =
                                params.iter().map(|p| p as &dyn duckdb::ToSql).collect();
                            stmt.query(params_as_ref.as_slice()).map_err(Into::into)
                        })
                        .map(|mut rows| {
                            let column_map = Arc::new(
                                rows.as_ref()
                                    .unwrap()
                                    .column_names()
                                    .into_iter()
                                    .enumerate()
                                    .map(|(i, name)| (name.to_string(), i))
                                    .collect(),
                            );
                            let id = self.next_query_id;
                            self.next_query_id += 1;
                            // SAFETY: The rows are stored in the same struct as the connection.
                            // SAFETY: This is safe for the same reason as above. The `Rows` object
                            // is tied to the lifetime of the `Statement` and `Connection`, all of
                            // which are owned by the `Worker`. It will be dropped when the `Worker`
                            // is dropped.
                            let static_rows = unsafe { std::mem::transmute::<duckdb::Rows<'_>, duckdb::Rows<'static>>(rows) };
                            self.queries.insert(id, (static_rows, column_map));
                            id
                        });
                    let _ = responder.send(result);
                }
                Command::FetchRow {
                    query_id,
                    responder,
                } => {
                    let result = match self.queries.get_mut(&query_id) {
                        Some((rows, column_map)) => match rows.next() {
                            Ok(Some(row)) => {
                                let stmt = row.as_ref();
                                let values = (0..stmt.column_count())
                                    .map(|i| row.get_ref_unwrap(i).to_owned())
                                    .collect();
                                Ok(Some(crate::row::OwnedRow {
                                    values,
                                    column_map: column_map.clone(),
                                }))
                            }
                            Ok(None) => Ok(None),
                            Err(e) => Err(e.into()),
                        },
                        None => Ok(None), // Query not found, treat as end of stream.
                    };
                    let _ = responder.send(result);
                }
                Command::DropQuery { query_id } => {
                    self.queries.remove(&query_id);
                }
                Command::BeginTransaction { responder } => {
                    let (tx, rx) = flume::unbounded();
                    match self.conn.execute_batch("BEGIN TRANSACTION") {
                        Ok(_) => {
                            let _ = responder.send(Ok(tx));
                            self.handle_transaction(rx);
                        }
                        Err(e) => {
                            let _ = responder.send(Err(e.into()));
                        }
                    }
                }
            }
        }
    }

    /// Handles commands for a single transaction.
    fn handle_transaction(&mut self, rx: flume::Receiver<TransactionCommand>) {
        while let Ok(cmd) = rx.recv() {
            match cmd {
                TransactionCommand::Execute {
                    sql,
                    params,
                    responder,
                } => {
                    let params_as_ref: Vec<&dyn duckdb::ToSql> =
                        params.iter().map(|p| p as &dyn duckdb::ToSql).collect();
                    let result = self.conn.execute(&sql, params_as_ref.as_slice());
                    let _ = responder.send(result.map_err(Into::into));
                }
                TransactionCommand::Commit { responder } => {
                    let result = self.conn.execute_batch("COMMIT");
                    let _ = responder.send(result.map_err(Into::into));
                    return; // Exit transaction loop
                }
                TransactionCommand::Rollback { responder } => {
                    let result = self.conn.execute_batch("ROLLBACK");
                    let _ = responder.send(result.map_err(Into::into));
                    return; // Exit transaction loop
                }
            }
        }
        // Channel closed without commit/rollback, so we roll back.
        let _ = self.conn.execute_batch("ROLLBACK");
        // Receiver is disconnected, which means the AsyncConnection is dropped.
        // The worker can now shut down. The connection will be closed by the Drop impl of `duckdb::Connection`.
    }
}