use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "service_monitor_agents")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub monitor_id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub vps_id: i32,
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
        from = "Column::VpsId",
        to = "super::vps::Column::Id",
        on_delete = "Cascade",
        on_update = "Cascade"
    )]
    Vps,
}

impl ActiveModelBehavior for ActiveModel {}
