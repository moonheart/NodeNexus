use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "vps_renewal_info")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)] // vps_id is PK and FK
    pub vps_id: i32,
    pub renewal_cycle: Option<String>,
    pub renewal_cycle_custom_days: Option<i32>,
    pub renewal_price: Option<f64>,
    pub renewal_currency: Option<String>,
    pub next_renewal_date: Option<ChronoDateTimeUtc>,
    pub last_renewal_date: Option<ChronoDateTimeUtc>,
    pub service_start_date: Option<ChronoDateTimeUtc>,
    pub payment_method: Option<String>,
    pub auto_renew_enabled: Option<bool>,
    #[sea_orm(column_type = "Text", nullable)]
    pub renewal_notes: Option<String>,
    pub reminder_active: Option<bool>,
    pub last_reminder_generated_at: Option<ChronoDateTimeUtc>,
    pub created_at: ChronoDateTimeUtc,
    pub updated_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::vps::Entity",
        from = "Column::VpsId",
        to = "super::vps::Column::Id",
        on_delete = "Cascade", // If a VPS is deleted, its renewal info is also deleted
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

// For single primary key entities where `#[sea_orm(primary_key)]` is on the model field,
// an explicit PrimaryKey enum is not strictly needed unless for specific advanced use cases.
// Removing it to avoid potential conflicts and simplify. SeaORM will infer it.
