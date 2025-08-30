use crate::db::orm::entity::{Entity, FromRow};
use crate::db::orm::executor::Executor;
use crate::web::error::AppError;
use async_duckdb::pool::Pool;
use async_duckdb::Manager;
use async_trait::async_trait;
use deadpool::managed;
use futures_util::TryStreamExt;

pub struct Transaction {
    conn: managed::Object<Manager>,
    tx: Option<async_duckdb::AsyncTransaction>,
}

impl Transaction {
    pub async fn new(pool: &Pool) -> Result<Self, AppError> {
        let conn = pool.get().await?;
        let tx = conn.transaction().await?;
        Ok(Self {
            conn,
            tx: Some(tx),
        })
    }

    pub async fn commit(mut self) -> Result<(), AppError> {
        if let Some(tx) = self.tx.take() {
            tx.commit().await?;
        }
        Ok(())
    }

    pub async fn rollback(mut self) -> Result<(), AppError> {
        if let Some(tx) = self.tx.take() {
            tx.rollback().await?;
        }
        Ok(())
    }
}

impl Drop for Transaction {
    fn drop(&mut self) {
        if let Some(tx) = self.tx.take() {
            tokio::spawn(async move {
                if let Err(e) = tx.rollback().await {
                    tracing::error!("Failed to rollback transaction: {}", e);
                }
            });
        }
    }
}

#[async_trait]
impl Executor for Transaction {
    async fn execute(&mut self, query: &str, params: Vec<duckdb::types::Value>) -> Result<usize, AppError> {
        let tx = self.tx.as_mut().expect("Transaction is missing");
        let rows_affected = tx.execute(query, params).await?;
        Ok(rows_affected)
    }

    async fn execute_with_params_and_return_row<E: Entity + Send>(
        &mut self,
        query: &str,
        params: Vec<duckdb::types::Value>,
    ) -> Result<E, AppError> {
        let tx = self.tx.as_mut().expect("Transaction is missing");
        let mut stmt = self.conn.prepare(query).await?;
        let mut rows = stmt.query(params).await?;
        let row = rows.try_next().await?.ok_or_else(|| AppError::NotFound("Entity not found".to_string()))?;
        Ok(E::from_row(&row)?)
    }

    async fn fetch_optional_one<E: Entity + Send>(
        &mut self,
        query: &str,
        params: Vec<duckdb::types::Value>,
    ) -> Result<Option<E>, AppError> {
        let tx = self.tx.as_mut().expect("Transaction is missing");
        let mut stmt = self.conn.prepare(query).await?;
        let mut rows = stmt.query(params).await?;
        if let Some(row) = rows.try_next().await? {
            Ok(Some(E::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    async fn fetch_all<E: Entity + Send>(
        &mut self,
        query: &str,
        params: Vec<duckdb::types::Value>,
    ) -> Result<Vec<E>, AppError> {
        let tx = self.tx.as_mut().expect("Transaction is missing");
        let mut stmt = self.conn.prepare(query).await?;
        let rows = stmt.query(params).await?;
        let rows: Vec<_> = rows.try_collect().await?;
        rows.iter().map(|row| E::from_row(row).map_err(AppError::from)).collect()
    }

    async fn fetch_optional_one_typed<T: FromRow + Send>(
        &mut self,
        query: &str,
        params: Vec<duckdb::types::Value>,
    ) -> Result<Option<T>, AppError> {
        let mut stmt = self.conn.prepare(query).await?;
        let mut rows = stmt.query(params).await?;
        if let Some(row) = rows.try_next().await? {
            Ok(Some(T::from_row(&row)?))
        } else {
            Ok(None)
        }
    }

    async fn fetch_all_typed<T: FromRow + Send>(
        &mut self,
        query: &str,
        params: Vec<duckdb::types::Value>,
    ) -> Result<Vec<T>, AppError> {
        let mut stmt = self.conn.prepare(query).await?;
        let rows = stmt.query(params).await?;
        let rows: Vec<_> = rows.try_collect().await?;
        rows.iter().map(|row| T::from_row(row).map_err(AppError::from)).collect()
    }
}