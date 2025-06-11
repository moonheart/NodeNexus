use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "tasks")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub vps_id_target: Option<i32>,
    pub name: String,
    #[sea_orm(column_name = "type")] // Matches #[serde(rename = "type")]
    pub task_type: String,
    pub schedule_cron: Option<String>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub command_payload: Option<Json>,
    pub ansible_playbook_path: Option<String>,
    pub created_at: ChronoDateTimeUtc,
    pub updated_at: ChronoDateTimeUtc,
    pub last_run_at: Option<ChronoDateTimeUtc>,
    pub next_run_at: Option<ChronoDateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::vps::Entity",
        from = "Column::VpsIdTarget",
        to = "super::vps::Column::Id",
        on_delete = "SetNull", // Assuming if a VPS is deleted, the task's target is set to NULL
        on_update = "Cascade"  // If VPS ID changes, update here
    )]
    Vps, // This relation is for vps_id_target
    // Task might have_many TaskRun
    // #[sea_orm(has_many = "super::task_run::Entity")]
    // TaskRun,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::vps::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Vps.def()
    }
}

// Implement Related for TaskRun when it is created.

impl ActiveModelBehavior for ActiveModel {}