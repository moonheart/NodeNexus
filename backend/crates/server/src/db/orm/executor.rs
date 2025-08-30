use crate::{db::orm::entity::{Entity, FromRow}, web::error::AppError};
use async_trait::async_trait;
use duckdb::ToSql;

#[async_trait]
pub trait Executor {
    async fn execute(&mut self, query: &str, params: Vec<duckdb::types::Value>) -> Result<usize, AppError>;

    async fn execute_with_params_and_return_row<E: Entity + Send>(
        &mut self,
        query: &str,
        params: Vec<duckdb::types::Value>,
    ) -> Result<E, AppError>;

    async fn fetch_optional_one<E: Entity + Send>(
        &mut self,
        query: &str,
        params: Vec<duckdb::types::Value>,
    ) -> Result<Option<E>, AppError>;


    async fn fetch_all<E: Entity + Send>(
        &mut self,
        query: &str,
        params: Vec<duckdb::types::Value>,
    ) -> Result<Vec<E>, AppError>;

    async fn fetch_optional_one_typed<T: FromRow + Send>(
        &mut self,
        query: &str,
        params: Vec<duckdb::types::Value>,
    ) -> Result<Option<T>, AppError>;

    async fn fetch_all_typed<T: FromRow + Send>(
        &mut self,
        query: &str,
        params: Vec<duckdb::types::Value>,
    ) -> Result<Vec<T>, AppError>;
}