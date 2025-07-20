use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "alert_events")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub rule_id: i32,
    pub vps_id: i32,
    pub trigger_time: ChronoDateTimeUtc,
    pub resolve_time: Option<ChronoDateTimeUtc>,
    #[sea_orm(column_type = "Text", nullable)] // Assuming details can be large
    pub details: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::alert_rule::Entity",
        from = "Column::RuleId",
        to = "super::alert_rule::Column::Id",
        on_delete = "Cascade", // If an AlertRule is deleted, its events are also deleted
        on_update = "Cascade"
    )]
    AlertRule,
    #[sea_orm(
        belongs_to = "super::vps::Entity",
        from = "Column::VpsId",
        to = "super::vps::Column::Id",
        on_delete = "Cascade", // If a VPS is deleted, its alert events are also deleted
        on_update = "Cascade"
    )]
    Vps,
}

impl Related<super::alert_rule::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AlertRule.def()
    }
}

impl Related<super::vps::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Vps.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
