use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "settings")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)] // `key` is not auto-incrementing
    pub key: String,
    #[sea_orm(column_type = "JsonBinary")]
    pub value: Json, // SeaORM maps Json to serde_json::Value
    pub updated_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    // Settings table typically doesn't have relations to other main entities
    // unless there's a specific use case (e.g., user-specific settings linked to user_id).
    // Based on the model, it seems to be global settings.
}

// No explicit Related impls needed if there are no relations.

impl ActiveModelBehavior for ActiveModel {}

// For single primary key entities where `#[sea_orm(primary_key)]` is on the model field,
// an explicit PrimaryKey enum is not strictly needed.
// Removing it to avoid potential conflicts and simplify. SeaORM will infer it.