use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "service_monitors")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub monitor_type: String,
    pub target: String,
    pub frequency_seconds: i32,
    pub timeout_seconds: i32,
    pub is_active: bool,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub monitor_config: Option<Json>,
    pub created_at: ChronoDateTimeUtc,
    pub updated_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_delete = "Cascade"
    )]
    User,

    #[sea_orm(has_many = "super::service_monitor_result::Entity")]
    ServiceMonitorResult,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::service_monitor_result::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ServiceMonitorResult.def()
    }
}

// Many-to-many relation for directly assigned agents (vps)
impl Related<super::vps::Entity> for Entity {
    fn to() -> RelationDef {
        super::service_monitor_agent::Relation::Vps.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::service_monitor_agent::Relation::ServiceMonitor.def().rev())
    }
}

// Many-to-many relation for tag-based assignments
impl Related<super::tag::Entity> for Entity {
    fn to() -> RelationDef {
        super::service_monitor_tag::Relation::Tag.def()
    }
    fn via() -> Option<RelationDef> {
        Some(super::service_monitor_tag::Relation::ServiceMonitor.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}