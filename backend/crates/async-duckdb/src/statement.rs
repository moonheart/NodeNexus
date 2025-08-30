use crate::worker::{Command, StatementId};
use flume::Sender;
use std::marker::PhantomData;

/// An asynchronous prepared statement.
///
/// A prepared statement is a template for a SQL query that can be executed
/// multiple times with different parameters.
#[derive(Debug)]
pub struct AsyncStatement<'conn> {
    pub(crate) id: StatementId,
    pub(crate) cmd_tx: Sender<Command>,
    pub(crate) _marker: PhantomData<&'conn ()>,
}

use crate::row::RowStream;

impl<'conn> AsyncStatement<'conn> {
    /// Execute the prepared statement, returning a stream of rows.
    pub async fn query<P>(&mut self, params: P) -> crate::Result<RowStream<'_>>
    where
        P: crate::IntoParams,
    {
        let (responder, rx) = futures_channel::oneshot::channel();
        self.cmd_tx
            .send_async(Command::QueryStatement {
                stmt_id: self.id,
                params: params.into_params()?,
                responder,
            })
            .await
            .map_err(|_| crate::Error::ChannelClosed)?;

        let query_id = rx.await.map_err(|_| crate::Error::WorkerCrashed)??;
        Ok(RowStream::new(query_id, self.cmd_tx.clone()))
    }

    /// Execute the prepared statement.
    ///
    /// On success, returns the number of rows that were changed or inserted or
    /// deleted.
    pub async fn execute<P>(&mut self, params: P) -> crate::Result<usize>
    where
        P: crate::IntoParams,
    {
        let (responder, rx) = futures_channel::oneshot::channel();
        self.cmd_tx
            .send_async(Command::ExecuteStatement {
                stmt_id: self.id,
                params: params.into_params()?,
                responder,
            })
            .await
            .map_err(|_| crate::Error::ChannelClosed)?;
        rx.await.map_err(|_| crate::Error::WorkerCrashed)?
    }
}

impl Drop for AsyncStatement<'_> {
    fn drop(&mut self) {
        // A send error is ignored because the worker thread may have already shut down.
        let _ = self.cmd_tx.send(Command::DropStatement { stmt_id: self.id });
    }
}