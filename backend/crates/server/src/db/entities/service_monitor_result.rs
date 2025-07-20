use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[sea_orm(table_name = "service_monitor_results")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_name = "time")]
    pub time: ChronoDateTimeUtc,
    #[sea_orm(primary_key, auto_increment = false)]
    pub monitor_id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub agent_id: i32,
    pub is_up: bool,
    #[sea_orm(nullable)]
    pub latency_ms: Option<i32>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub details: Option<Json>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::service_monitor::Entity",
        from = "Column::MonitorId",
        to = "super::service_monitor::Column::Id",
        on_delete = "Cascade",
        on_update = "Cascade"
    )]
    ServiceMonitor,

    #[sea_orm(
        belongs_to = "super::vps::Entity",
        from = "Column::AgentId",
        to = "super::vps::Column::Id",
        on_delete = "Cascade",
        on_update = "Cascade"
    )]
    Vps,
}

impl Related<super::service_monitor::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ServiceMonitor.def()
    }
}

impl Related<super::vps::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Vps.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
