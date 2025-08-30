# ORM JOIN Query Support Design

This document outlines the design for adding JOIN query support to the existing ORM module. The design prioritizes a type-safe and ergonomic API, leveraging the existing `orm_macros` to automate implementation details.

## 1. High-Level Goals

-   Introduce a `join` method on the `QueryBuilder`.
-   Support standard JOIN types (`INNER`, `LEFT`, `RIGHT`).
-   Deserialize JOIN query results into flat tuples (e.g., `(User, Post, Comment)`).
-   Ensure the API is type-safe and consistent with the existing ORM design.

## 2. Detailed Design

### 2.1. `QueryBuilder` and `JOIN` Method

The `QueryBuilder` will be enhanced to support joins with a fluent API. The generic parameter `T` will represent a tuple of all entities involved in the query.

**`join` Method Signature:**

```rust
pub fn join<J: Entity>(
    self,
    join_type: JoinType,
    on: impl FnOnce(T::Column, J::Column) -> OnClause,
) -> QueryBuilder<'a, <T as Append<J>>::Result>
where
    T: Append<J>,
```

-   **`JoinType` Enum**: A simple enum for `Inner`, `Left`, `Right`.
-   **`Append` Trait**: A helper trait to flatten the resulting tuple of entities (e.g., from `((A, B), C)` to `(A, B, C)`). This is crucial for an ergonomic API.
-   **`OnClause` Builder**: A type-safe builder for constructing the `ON` clause of the JOIN.

### 2.2. Type-Safe `ON` Clause

To avoid raw strings for join conditions, we will use a builder pattern.

**`OnClause` Struct:**

```rust
pub struct OnClause(String);

impl OnClause {
    pub fn new<L: QualifiedColumn, R: QualifiedColumn>(left: L, right: R) -> Self;
    pub fn and<L: QualifiedColumn, R: QualifiedColumn>(self, left: L, right: R) -> Self;
}
```

**`QualifiedColumn` Trait:**

This trait provides the fully qualified column name (e.g., `"users"."id"`) to prevent ambiguity.

```rust
pub trait QualifiedColumn {
    fn qualified_name(&self) -> String;
}

impl<T: Entity> QualifiedColumn for T::Column {
    fn qualified_name(&self) -> String {
        format!("\"{}\".\"{}\"", T::TABLE_NAME, self.as_str())
    }
}
```

This implementation leverages `T::TABLE_NAME` provided by `orm_macros`, making the design clean and non-intrusive.

### 2.3. Result Deserialization (`FromRow`)

The cornerstone of this design is the ability to deserialize a row with columns from multiple tables into a flat tuple of `Entity` structs.

**`FromRow` Trait:**

```rust
pub trait FromRow: Sized {
    fn from_row(row: &async_duckdb::row::OwnedRow) -> Result<Self, duckdb::Error>;
}
```

**SQL Aliasing Strategy:**

The `QueryBuilder` will generate `SELECT` statements with explicit, prefixed aliases for every column.

*Example SQL:*
```sql
SELECT
  "users"."id" AS "users.id",
  "users"."name" AS "users.name",
  "posts"."id" AS "posts.id",
  "posts"."title" AS "posts.title"
FROM "users"
INNER JOIN "posts" ON "users"."id" = "posts"."user_id"
```

**`FromRow` for Tuples:**

The implementation for tuples will rely on a new `from_row_with_prefix` method on the `Entity` trait.

```rust
// In Entity trait
fn from_row_with_prefix(row: &async_duckdb::row::OwnedRow, prefix: &str) -> Result<Self, duckdb::Error>;

// Example implementation for a tuple
impl<A: Entity, B: Entity> FromRow for (A, B) {
    fn from_row(row: &async_duckdb::row::OwnedRow) -> Result<Self, duckdb::Error> {
        let entity_a = A::from_row_with_prefix(row, &format!("{}.", A::TABLE_NAME))?;
        let entity_b = B::from_row_with_prefix(row, &format!("{}.", B::TABLE_NAME))?;
        Ok((entity_a, entity_b))
    }
}
```

The `from_row_with_prefix` method will be implemented automatically by `orm_macros`.

### 2.4. `Executor` Extension

The `Executor` trait will be extended with generic methods to handle any `FromRow`-compatible type.

```rust
#[async_trait]
pub trait Executor {
    // ... existing methods ...

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
```

### 2.5. `orm_macros` Modifications

The `#[derive(Entity)]` macro will be updated to:
1.  Generate the implementation for `from_row_with_prefix`.
2.  Generate a method that returns a list of all column names for the entity, which is needed by `QueryBuilder` to construct the aliased `SELECT` clause.

## 3. Workflow Diagram

```mermaid
graph TD
    subgraph "Phase 1: Query Building"
        A["User Code: User::select().join::<Post>(...)"] --> B["QueryBuilder<'a, (User, Post)> created"];
        B --> C["build_select_sql generates aliased SQL using Entity::columns()"];
    end

    subgraph "Phase 2: Execution"
        C --> D["Executor::fetch_all_typed executes query"];
        D --> E["Database returns rows with aliased columns (e.g., 'users.id')"];
    end

    subgraph "Phase 3: Deserialization"
        E --> F["FromRow for (User, Post) is called for each row"];
        F --> G["Calls User::from_row_with_prefix(\"users.\")"];
        F --> H["Calls Post::from_row_with_prefix(\"posts.\")"];
        G & H --> I["(User, Post) tuple is constructed"];
    end

    I --> J["Final Result: Vec<(User, Post)>"];
```

## 4. Conclusion

This design provides a comprehensive, type-safe, and ergonomic solution for adding `JOIN` support to the ORM. It balances API elegance with implementation feasibility, leveraging Rust's trait system and procedural macros to create a powerful and maintainable feature.