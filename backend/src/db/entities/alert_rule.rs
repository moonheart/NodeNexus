use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "alert_rules")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub vps_id: Option<i32>,
    pub metric_type: String,
    pub threshold: f64,
    pub comparison_operator: String,
    pub duration_seconds: i32,
    pub is_active: bool,
    pub last_triggered_at: Option<ChronoDateTimeUtc>,
    pub cooldown_seconds: i32,
    pub created_at: ChronoDateTimeUtc,
    pub updated_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_delete = "Cascade",
        on_update = "Cascade"
    )]
    User,
    #[sea_orm(
        belongs_to = "super::vps::Entity",
        from = "Column::VpsId",
        to = "super::vps::Column::Id",
        on_delete = "SetNull", // If VPS is deleted, set vps_id to NULL
        on_update = "Cascade"
    )]
    Vps,
    #[sea_orm(has_many = "super::alert_event::Entity")]
    AlertEvent
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::vps::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Vps.def()
    }
}

impl Related<super::alert_event::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AlertEvent.def()
    }
}

impl Related<super::notification_channel::Entity> for Entity {
    fn to() -> RelationDef {
        super::alert_rule_channel::Relation::NotificationChannel.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::alert_rule_channel::Relation::AlertRule.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}