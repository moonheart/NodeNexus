use super::query::QueryBuilder;
use crate::db::orm::executor::Executor;
use crate::web::error::AppError;
use duckdb::ToSql;
use async_trait::async_trait;
use std::error::Error as StdError;
use duckdb::types::Type;

pub trait Column: Copy {
    fn as_str(&self) -> &'static str;
}

#[derive(Debug, Clone, Copy)]
pub enum Order {
    Asc,
    Desc,
}

impl Order {
    pub fn as_str(&self) -> &'static str {
        match self {
            Order::Asc => "ASC",
            Order::Desc => "DESC",
        }
    }
}

#[async_trait]
pub trait Entity: Sized + FromRow + Send + Sync + Clone + serde::de::DeserializeOwned + 'static {
    type PrimaryKey: Send + Sync;
    type Column: Column;

    const TABLE_NAME: &'static str;
    
    fn from_row_with_prefix(row: &async_duckdb::row::OwnedRow, prefix: &str) -> Result<Self, duckdb::Error>;
    fn columns() -> Vec<&'static str>;

    async fn insert<E: Executor + Send>(self, exec: &mut E) -> Result<Self, AppError>;

    async fn update<E: Executor + Send>(self, exec: &mut E) -> Result<Self, AppError>;

    async fn delete<E: Executor + Send>(exec: &mut E, id: Self::PrimaryKey) -> Result<u64, AppError>;

    async fn delete_many<E: Executor + Send>(builder: QueryBuilder<'_, Self>, exec: &mut E) -> Result<u64, AppError>;

    async fn find_by_id<E: Executor + Send>(
        exec: &mut E,
        id: Self::PrimaryKey,
    ) -> Result<Option<Self>, AppError>;

    fn select<'a>() -> QueryBuilder<'a, Self> {
        QueryBuilder::new()
    }
}

pub trait FromRow: Sized {
    fn from_row(row: &async_duckdb::row::OwnedRow) -> Result<Self, duckdb::Error>;
}

pub fn json_from_row<'a, T: serde::de::DeserializeOwned>(row: &'a async_duckdb::row::OwnedRow, col_name: &str) -> Result<T, duckdb::Error> {
    let value: duckdb::types::Value = row.get(col_name).map_err(|e| duckdb::Error::FromSqlConversionFailure(0, Type::Any, Box::new(e)))?;
    match value {
        duckdb::types::Value::Text(s) => {
            serde_json::from_str(&s).map_err(|e| duckdb::Error::FromSqlConversionFailure(0, Type::Any, Box::new(e)))
        },
        _ => Err(duckdb::Error::FromSqlConversionFailure(0, Type::Any, "Expected a string for JSON deserialization".into()))
    }
}

pub trait QualifiedColumn: Column {
    fn qualified_name(&self, table_name: &'static str) -> String;
}

impl<C: Column> QualifiedColumn for C {
    fn qualified_name(&self, table_name: &'static str) -> String {
        format!("\"{}\".\"{}\"", table_name, self.as_str())
    }
}
