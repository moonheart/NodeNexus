use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "performance_metrics")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub time: ChronoDateTimeUtc,
    #[sea_orm(primary_key, auto_increment = false)]
    pub vps_id: i32,
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: i64,
    pub memory_total_bytes: i64,
    pub swap_usage_bytes: i64,
    pub swap_total_bytes: i64,
    pub disk_io_read_bps: i64,
    pub disk_io_write_bps: i64,
    pub total_disk_space_bytes: i64,
    pub used_disk_space_bytes: i64,
    pub network_rx_cumulative: i64,
    pub network_tx_cumulative: i64,
    pub network_rx_instant_bps: i64,
    pub network_tx_instant_bps: i64,
    pub uptime_seconds: i64,
    pub total_processes_count: i32,
    pub running_processes_count: i32,
    pub tcp_established_connection_count: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::vps::Entity",
        from = "Column::VpsId",
        to = "super::vps::Column::Id"
    )]
    Vps,
    // PerformanceMetric might have_many PerformanceDiskUsage and PerformanceNetworkInterfaceStat
}

impl Related<super::vps::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Vps.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
