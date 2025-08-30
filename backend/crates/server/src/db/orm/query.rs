use super::entity::{Column, Entity, Order, QualifiedColumn, FromRow};
use crate::db::orm::executor::Executor;
use crate::web::error::AppError;
use duckdb::{types::Value, ToSql};
use std::marker::PhantomData;

fn to_sql_value(v: &dyn ToSql) -> duckdb::Result<Value> {
    let value = v.to_sql()?;
    Ok(match value {
        duckdb::types::ToSqlOutput::Borrowed(v) => v.to_owned(),
        duckdb::types::ToSqlOutput::Owned(v) => v,
        _ => unreachable!(),
    })
}

enum ConditionValue {
    Single(Box<dyn ToSql + Send>),
    List(Vec<Box<dyn ToSql + Send>>),
}

enum FilterOp {
    Equals,
    In,
    LessOrEqual,
}

pub enum JoinType {
    Inner,
    Left,
    Right,
}

impl JoinType {
    fn as_str(&self) -> &'static str {
        match self {
            JoinType::Inner => "INNER JOIN",
            JoinType::Left => "LEFT JOIN",
            JoinType::Right => "RIGHT JOIN",
        }
    }
}

pub struct OnClause(String);

impl OnClause {
    pub fn new<L: Column, R: Column>(left: L, left_table: &'static str, right: R, right_table: &'static str) -> Self {
        Self(format!("{} = {}", left.qualified_name(left_table), right.qualified_name(right_table)))
    }

    pub fn and<L: Column, R: Column>(mut self, left: L, left_table: &'static str, right: R, right_table: &'static str) -> Self {
        self.0.push_str(&format!(" AND {} = {}", left.qualified_name(left_table), right.qualified_name(right_table)));
        self
    }
}

pub trait Append<T> {
    type Result;
}

impl<A: Entity, B: Entity> Append<B> for A {
    type Result = (A, B);
}

impl<A: Entity, B: Entity, C: Entity> Append<C> for (A, B) {
    type Result = (A, B, C);
}

pub struct QueryBuilder<'a, T> {
    conditions: Vec<(String, FilterOp, ConditionValue)>,
    ordering: Vec<(String, Order)>,
    joins: Vec<String>,
    _marker: PhantomData<&'a T>,
}

impl<'a, T: 'static> QueryBuilder<'a, T> {
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
            ordering: Vec::new(),
            joins: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn filter<C: Column>(mut self, column: C, table: &'static str, value: impl ToSql + Send + 'static) -> Self {
        self.conditions.push((
            column.qualified_name(table),
            FilterOp::Equals,
            ConditionValue::Single(Box::new(value)),
        ));
        self
    }

    pub fn order_by<C: Column>(mut self, column: C, table: &'static str, direction: Order) -> Self {
        self.ordering.push((column.qualified_name(table), direction));
        self
    }

    fn build_select_sql(&self) -> String {
        // This part needs to be heavily modified to support tuples of entities
        // For now, this is a placeholder
        let mut sql = format!("SELECT * FROM DUMMY_TABLE");
        if !self.joins.is_empty() {
            sql.push_str(" ");
            sql.push_str(&self.joins.join(" "));
        }
        if !self.conditions.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.build_where_clause());
        }
        if !self.ordering.is_empty() {
            let order_clauses = self
                .ordering
                .iter()
                .map(|(col, dir)| format!("{} {}", col, dir.as_str()))
                .collect::<Vec<_>>()
                .join(", ");
            sql.push_str(" ORDER BY ");
            sql.push_str(&order_clauses);
        }
        sql
    }

    fn build_where_clause(&self) -> String {
        self.conditions
            .iter()
            .map(|(col, op, val)| match op {
                FilterOp::Equals => format!("{} = ?", col),
                FilterOp::LessOrEqual => format!("{} <= ?", col),
                FilterOp::In => {
                    if let ConditionValue::List(vals) = val {
                        if vals.is_empty() {
                            "1=0".to_string()
                        } else {
                            let placeholders =
                                vals.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
                            format!("{} IN ({})", col, placeholders)
                        }
                    } else {
                        panic!("IN operator requires a list of values");
                    }
                }
            })
            .collect::<Vec<_>>()
            .join(" AND ")
    }

    fn into_params(self) -> duckdb::Result<Vec<Value>> {
        self.conditions
            .into_iter()
            .flat_map(|(_, _, val)| match val {
                ConditionValue::Single(v) => vec![v],
                ConditionValue::List(vs) => vs,
            })
            .map(|v| to_sql_value(v.as_ref()))
            .collect()
    }

    pub async fn one<E: Executor + Send>(self, exec: &mut E) -> Result<Option<T>, AppError>
    where
        T: FromRow + Send,
    {
        let sql = self.build_select_sql();
        let params = self.into_params()?;
        exec.fetch_optional_one_typed(&sql, params).await
    }

    pub async fn all<E: Executor + Send>(self, exec: &mut E) -> Result<Vec<T>, AppError>
    where
        T: FromRow + Send,
    {
        let sql = self.build_select_sql();
        let params = self.into_params()?;
        exec.fetch_all_typed(&sql, params).await
    }

    pub async fn delete<E: Executor + Send>(self, exec: &mut E) -> Result<u64, AppError> {
        // This needs to be implemented properly
        Ok(0)
    }
}