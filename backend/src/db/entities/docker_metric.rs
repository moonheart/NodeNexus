use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize}; // Keep Serialize/Deserialize imports for now, might be needed by other parts or if we re-add

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "docker_metrics")]
pub struct Model {
    // Assuming `docker_metrics` is a hypertable and `time` is part of its primary key
    // or at least a crucial indexing column. SeaORM typically expects a single primary key column
    // by default. If `time` and `container_db_id` form a composite primary key,
    // you'd need to define it in `PrimaryKey` and adjust `auto_increment` if necessary.
    // For simplicity, if there isn't a single auto-incrementing PK, we might need to
    // adjust how `primary_key` is defined or if it's used.
    // Let's assume for now there's an implicit or explicit single PK, or we'll adjust if needed.
    // If `time` + `container_db_id` is the PK, then `#[sea_orm(primary_key, auto_increment = false)]`
    // would be on both `time` and `container_db_id` in the `Column` enum, and `PrimaryKey` would list both.
    // For now, let's assume `time` is the main time column and `container_db_id` is FK.
    // Hypertables often don't have a simple auto-incrementing ID.
    // We will treat `time` and `container_db_id` as simple columns for now and adjust if schema demands.
    // If there's no single primary key in the table, we might need to omit `#[sea_orm(primary_key)]`
    // or define a composite one. Let's assume `time` is the primary time dimension.

    #[sea_orm(primary_key, auto_increment = false)] // Hypertables usually don't auto-increment PKs in the same way
    pub time: ChronoDateTimeUtc,
    #[sea_orm(primary_key, auto_increment = false)] // Part of composite PK
    pub container_db_id: i32, // Foreign key to docker_containers.id
    pub cpu_usage: f64,
    pub mem_usage: f64, // Assuming this will store bytes
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::docker_container::Entity",
        from = "Column::ContainerDbId",
        to = "super::docker_container::Column::Id"
    )]
    DockerContainer,
}

impl Related<super::docker_container::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DockerContainer.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}