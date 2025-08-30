use crate::{
    db::{orm::{entity::{Entity, FromRow}, executor::Executor}},
    web::error::AppError,
};
use async_duckdb::Manager;
use async_trait::async_trait;
use deadpool::managed;
use futures_util::TryStreamExt;

#[async_trait]
impl Executor for managed::Object<Manager> {
    async fn execute(&mut self, query: &str, params: Vec<duckdb::types::Value>) -> Result<usize, AppError> {
        let conn = self;
        let mut stmt = conn.prepare(query).await?;
        let rows_affected = stmt.execute(params).await?;
        Ok(rows_affected)
    }

    async fn execute_with_params_and_return_row<E: Entity + Send>(
        &mut self,
        query: &str,
        params: Vec<duckdb::types::Value>,
    ) -> Result<E, AppError> {
        let conn = self;
        let mut stmt = conn.prepare(query).await?;
        let mut rows = stmt.query(params).await?;
        let row = rows.try_next().await?.ok_or_else(|| AppError::NotFound("Entity not found".to_string()))?;
        Ok(E::from_row(&row)?)
    }

    async fn fetch_optional_one<E: Entity + Send>(
        &mut self,
        query: &str,
        params: Vec<duckdb::types::Value>,
    ) -> Result<Option<E>, AppError> {
        self.fetch_optional_one_typed(query, params).await
    }

    async fn fetch_all<E: Entity + Send>(
        &mut self,
        query: &str,
        params: Vec<duckdb::types::Value>,
    ) -> Result<Vec<E>, AppError> {
        self.fetch_all_typed(query, params).await
    }

    async fn fetch_optional_one_typed<T: FromRow + Send>(
        &mut self,
        query: &str,
        params: Vec<duckdb::types::Value>,
    ) -> Result<Option<T>, AppError> {
        let conn = self;
        let mut stmt = conn.prepare(query).await?;
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
        let conn = self;
        let mut stmt = conn.prepare(query).await?;
        let rows = stmt.query(params).await?;
        let rows: Vec<_> = rows.try_collect().await?;
        rows.iter().map(|row| T::from_row(row).map_err(AppError::from)).collect()
    }
}