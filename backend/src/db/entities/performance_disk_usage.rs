use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "performance_disk_usages")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub performance_metric_id: i32,
    pub mount_point: String,
    pub used_bytes: i64,
    pub total_bytes: i64,
    pub fstype: Option<String>,
    pub usage_percent: f64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::performance_metric::Entity",
        from = "Column::PerformanceMetricId",
        to = "super::performance_metric::Column::Id",
        on_delete = "Cascade", // If a PerformanceMetric is deleted, its disk usages are also deleted
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