use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize}; // Keep Serialize/Deserialize imports

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "vps_monthly_traffic")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub vps_id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub month: ChronoDate, // Corresponds to DATE type in SQL
    pub total_rx: i64,
    pub total_tx: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::vps::Entity",
        from = "Column::VpsId",
        to = "super::vps::Column::Id",
        on_delete = "Cascade", // If a VPS is deleted, its monthly traffic data is also deleted
        on_update = "Cascade"
    )]
    Vps,
}

impl Related<super::vps::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Vps.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
