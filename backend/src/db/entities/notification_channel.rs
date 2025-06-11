use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "notification_channels")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub channel_type: String, // e.g., "telegram", "webhook"
    pub config: Vec<u8>,      // Encrypted JSON blob
    pub created_at: ChronoDateTimeUtc,
    pub updated_at: ChronoDateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_delete = "Cascade", // If a User is deleted, their notification channels are also deleted
        on_update = "Cascade"
    )]
    User,
    // This represents the link: NotificationChannel has_many AlertRuleChannel entries
    #[sea_orm(has_many = "super::alert_rule_channel::Entity")]
    AlertRuleChannels, // Renamed from AlertRule
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

// Defines how to get AlertRule(s) from a NotificationChannel via AlertRuleChannels
impl Related<super::alert_rule::Entity> for Entity {
    fn to() -> RelationDef {
        // Link from AlertRuleChannel to AlertRule
        super::alert_rule_channel::Relation::AlertRule.def()
    }

    fn via() -> Option<RelationDef> {
        // Path: NotificationChannel -> AlertRuleChannel (AlertRuleChannels)
        Some(super::alert_rule_channel::Relation::NotificationChannel.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}