use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "docker_containers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub vps_id: i32,
    pub container_id_on_host: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub created_at_on_host: Option<ChronoDateTimeUtc>,
    pub created_at: ChronoDateTimeUtc,
    pub updated_at: ChronoDateTimeUtc,
    // Note: `labels` and `mounts` from the original proto could be added here
    // as a JsonBinary column if they are stored in the database.
    // Example:
    // #[sea_orm(column_type = "JsonBinary", nullable)]
    // pub labels: Option<Json>,
    // #[sea_orm(column_type = "JsonBinary", nullable)]
    // pub mounts: Option<Json>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::vps::Entity",
        from = "Column::VpsId",
        to = "super::vps::Column::Id"
    )]
    Vps,
    // DockerContainer might have_many DockerMetric
    // This will be defined later.
    // Example:
    // #[sea_orm(has_many = "super::docker_metric::Entity")]
    // DockerMetric,
}

impl Related<super::vps::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Vps.def()
    }
}

// Implement Related for DockerMetric when it is created.

impl ActiveModelBehavior for ActiveModel {}
