use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "performance_network_interface_stats")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub performance_metric_id: i32,
    pub interface_name: String,
    pub rx_bytes_per_sec: i64,
    pub tx_bytes_per_sec: i64,
    pub rx_packets_per_sec: i64,
    pub tx_packets_per_sec: i64,
    pub rx_errors_total_cumulative: i64,
    pub tx_errors_total_cumulative: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::performance_metric::Entity",
        from = "Column::PerformanceMetricId",
        to = "super::performance_metric::Column::Id",
        on_delete = "Cascade", // If a PerformanceMetric is deleted, its network stats are also deleted
        on_update = "Cascade"
    )]
    PerformanceMetric,
}

impl Related<super::performance_metric::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PerformanceMetric.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
