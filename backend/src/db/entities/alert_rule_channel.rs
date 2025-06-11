use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize}; // Keep Serialize/Deserialize imports

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "alert_rule_channels")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub alert_rule_id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub channel_id: i32,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::alert_rule::Entity",
        from = "Column::AlertRuleId",
        to = "super::alert_rule::Column::Id",
        on_delete = "Cascade",
        on_update = "Cascade"
    )]
    AlertRule,
    #[sea_orm(
        belongs_to = "super::notification_channel::Entity",
        from = "Column::ChannelId",
        to = "super::notification_channel::Column::Id",
        on_delete = "Cascade",
        on_update = "Cascade"
    )]
    NotificationChannel,
}

impl Related<super::alert_rule::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::AlertRule.def()
    }
}

impl Related<super::notification_channel::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::NotificationChannel.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}