use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "service_monitor_tags")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub monitor_id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub tag_id: i32,
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
        belongs_to = "super::tag::Entity",
        from = "Column::TagId",
        to = "super::tag::Column::Id",
        on_delete = "Cascade",
        on_update = "Cascade"
    )]
    Tag,
}

impl ActiveModelBehavior for ActiveModel {}
