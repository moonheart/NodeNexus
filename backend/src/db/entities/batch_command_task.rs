use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use crate::db::enums::BatchCommandStatus; // Added import

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "batch_command_tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub batch_command_id: Uuid,
    pub original_request_payload: Json,
    #[sea_orm(indexed)]
    pub status: BatchCommandStatus, // Changed type from String
    pub execution_alias: Option<String>,
    #[sea_orm(indexed)]
    pub user_id: String, // Assuming user_id is a string, adjust if it's a Uuid or other type
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    pub completed_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::child_command_task::Entity")]
    ChildCommandTask,
}

impl Related<super::child_command_task::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ChildCommandTask.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

// Removed commented out enum definition as it's now in db/enums.rs