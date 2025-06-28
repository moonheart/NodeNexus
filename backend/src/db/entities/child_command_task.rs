use crate::db::enums::ChildCommandStatus;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize}; // Added import

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "child_command_tasks")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub child_command_id: Uuid,
    #[sea_orm(indexed)]
    pub batch_command_id: Uuid,
    #[sea_orm(indexed)]
    pub vps_id: i32,
    #[sea_orm(indexed)]
    pub status: ChildCommandStatus, // Changed type from String
    pub exit_code: Option<i32>,
    pub error_message: Option<String>, // Using String for Text, SeaORM handles Text type appropriately
    pub stdout_log_path: Option<String>,
    pub stderr_log_path: Option<String>,
    pub last_output_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    pub agent_started_at: Option<DateTimeUtc>,
    pub agent_completed_at: Option<DateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::batch_command_task::Entity",
        from = "Column::BatchCommandId",
        to = "super::batch_command_task::Column::BatchCommandId"
    )]
    BatchCommandTask,
}

impl Related<super::batch_command_task::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BatchCommandTask.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

// Removed commented out enum definition as it's now in db/enums.rs
