use crate::worker::{Command, QueryId};
use crate::{Error, Result};
use duckdb::types::{FromSql, Value, ValueRef};
use flume::Sender;
use futures_channel::oneshot;
use futures_util::stream::Stream;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use std::collections::HashMap;
use std::sync::Arc;

/// An owned row of data.
#[derive(Debug, Clone)]
pub struct OwnedRow {
    pub(crate) values: Vec<Value>,
    pub(crate) column_map: Arc<HashMap<String, usize>>,
}

impl OwnedRow {
    /// Get the value of a column in the row.
    pub fn get<'a, I, T>(&'a self, idx: I) -> Result<T>
    where
        I: RowIndex + 'a,
        T: FromSql,
    {
        idx.get(self)
    }
}

pub trait RowIndex {
    fn get<T: FromSql>(&self, row: &OwnedRow) -> Result<T>;
}

impl RowIndex for usize {
    fn get<T: FromSql>(&self, row: &OwnedRow) -> Result<T> {
        let value_ref: ValueRef<'_> = (&row.values[*self]).into();
        Ok(<T as FromSql>::column_result(value_ref)?)
    }
}

impl RowIndex for &str {
    fn get<T: FromSql>(&self, row: &OwnedRow) -> Result<T> {
        let idx = row.column_map.get(*self).ok_or(Error::DuckDB(duckdb::Error::InvalidColumnName(self.to_string())))?;
        row.get(*idx)
    }
}

/// The state of the RowStream.
enum StreamState {
    /// The stream is idle and ready to fetch the next row.
    Idle,
    /// The stream is currently fetching the next row.
    Fetching(oneshot::Receiver<Result<Option<OwnedRow>>>),
}

/// A stream of rows from a query.
///
/// This stream is created by calling [`AsyncConnection::query`] or
/// [`AsyncStatement::query`].
#[must_use = "streams do nothing unless polled"]
pub struct RowStream<'conn> {
    query_id: QueryId,
    cmd_tx: Sender<Command>,
    state: StreamState,
    _marker: PhantomData<&'conn ()>,
}

impl<'conn> RowStream<'conn> {
    pub(crate) fn new(query_id: QueryId, cmd_tx: Sender<Command>) -> Self {
        Self {
            query_id,
            cmd_tx,
            state: StreamState::Idle,
            _marker: PhantomData,
        }
    }
}

impl Stream for RowStream<'_> {
    type Item = Result<OwnedRow>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            match self.state {
                StreamState::Idle => {
                    let (responder, rx) = oneshot::channel();
                    let cmd = Command::FetchRow {
                        query_id: self.query_id,
                        responder,
                    };

                    if self.cmd_tx.send(cmd).is_err() {
                        return Poll::Ready(Some(Err(Error::ChannelClosed)));
                    }
                    self.state = StreamState::Fetching(rx);
                    // Continue the loop to poll the new future immediately.
                }
                StreamState::Fetching(ref mut rx) => {
                    return match Pin::new(rx).poll(cx) {
                        Poll::Ready(Ok(Ok(Some(row)))) => {
                            self.state = StreamState::Idle;
                            Poll::Ready(Some(Ok(row)))
                        }
                        Poll::Ready(Ok(Ok(None))) => {
                            self.state = StreamState::Idle;
                            Poll::Ready(None)
                        }
                        Poll::Ready(Ok(Err(e))) => {
                            self.state = StreamState::Idle;
                            Poll::Ready(Some(Err(e)))
                        }
                        Poll::Ready(Err(_)) => {
                            self.state = StreamState::Idle;
                            Poll::Ready(Some(Err(Error::WorkerCrashed)))
                        }
                        Poll::Pending => Poll::Pending,
                    };
                }
            }
        }
    }
}

impl Drop for RowStream<'_> {
    fn drop(&mut self) {
        let _ = self.cmd_tx.send(Command::DropQuery {
            query_id: self.query_id,
        });
    }
}