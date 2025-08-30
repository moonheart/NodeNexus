use crate::worker::TransactionCommand;
use crate::{Error, IntoParams, Result};
use flume::Sender;
use futures_channel::oneshot;
use std::marker::PhantomData;

/// An asynchronous transaction.
///
/// A transaction is a sequence of operations performed as a single logical
/// unit of work.
///
/// If the transaction is not committed, it will be rolled back when it is
/// dropped.
#[derive(Debug)]
pub struct AsyncTransaction {
    pub(crate) tx_cmd: Sender<TransactionCommand>,
    pub(crate) committed: bool,
}

impl AsyncTransaction {
    /// Commits the transaction.
    pub async fn commit(mut self) -> Result<()> {
        let (responder, rx) = oneshot::channel();
        self.tx_cmd
            .send_async(TransactionCommand::Commit { responder })
            .await
            .map_err(|_| Error::ChannelClosed)?;
        self.committed = true;
        rx.await.map_err(|_| Error::WorkerCrashed)?
    }

    /// Rolls back the transaction.
    ///
    /// This is equivalent to dropping the transaction, but is more explicit.
    pub async fn rollback(mut self) -> Result<()> {
        let (responder, rx) = oneshot::channel();
        self.tx_cmd
            .send_async(TransactionCommand::Rollback { responder })
            .await
            .map_err(|_| Error::ChannelClosed)?;
        self.committed = true; // Prevent double-rollback in Drop
        rx.await.map_err(|_| Error::WorkerCrashed)?
    }

    /// Execute a statement that does not return any rows.
    ///
    /// On success, returns the number of rows that were changed or inserted or
    /// deleted.
    pub async fn execute<S, P>(&self, sql: S, params: P) -> Result<usize>
    where
        S: Into<String>,
        P: IntoParams,
    {
        let (responder, rx) = oneshot::channel();
        self.tx_cmd
            .send_async(TransactionCommand::Execute {
                sql: sql.into(),
                params: params.into_params()?,
                responder,
            })
            .await
            .map_err(|_| Error::ChannelClosed)?;
        rx.await.map_err(|_| Error::WorkerCrashed)?
    }
}

impl Drop for AsyncTransaction {
    fn drop(&mut self) {
        if !self.committed {
            let (responder, _) = oneshot::channel();
            // We don't care about the result, just that we tried to roll back.
            let _ = self.tx_cmd.send(TransactionCommand::Rollback { responder });
        }
    }
}