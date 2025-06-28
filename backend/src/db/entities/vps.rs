use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "vps")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub user_id: i32, // Foreign key to User
    pub name: String,
    pub ip_address: Option<String>,
    pub os_type: Option<String>,
    pub agent_secret: String,
    pub agent_version: Option<String>,
    pub status: String,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub metadata: Option<Json>,
    pub created_at: ChronoDateTimeUtc,
    pub updated_at: ChronoDateTimeUtc,
    pub group: Option<String>, // Matches #[serde(rename = "group")] but SeaORM uses struct field name
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub agent_config_override: Option<Json>,
    pub config_status: String,
    pub last_config_update_at: Option<ChronoDateTimeUtc>,
    pub last_config_error: Option<String>,
    pub traffic_limit_bytes: Option<i64>,
    pub traffic_billing_rule: Option<String>,
    pub traffic_current_cycle_rx_bytes: Option<i64>,
    pub traffic_current_cycle_tx_bytes: Option<i64>,
    pub last_processed_cumulative_rx: Option<i64>,
    pub last_processed_cumulative_tx: Option<i64>,
    pub traffic_last_reset_at: Option<ChronoDateTimeUtc>,
    pub traffic_reset_config_type: Option<String>,
    pub traffic_reset_config_value: Option<String>,
    pub next_traffic_reset_at: Option<ChronoDateTimeUtc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::performance_metric::Entity")] // Added HasMany relation
    PerformanceMetrics, // Renamed for clarity (plural)

    #[sea_orm(has_one = "super::vps_renewal_info::Entity")]
    VpsRenewalInfo,

    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id",
        on_delete = "Cascade", // If a User is deleted, their VPS are also deleted
        on_update = "Cascade"
    )]
    User,
}

impl Related<super::tag::Entity> for Entity {
    fn to() -> RelationDef {
        super::vps_tag::Relation::Tag.def() // Corrected: Link from VpsTag to Tag
    }
    fn via() -> Option<RelationDef> {
        Some(super::vps_tag::Relation::Vps.def().rev())
    }
}

impl Related<super::vps_renewal_info::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::VpsRenewalInfo.def()
    }
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::performance_metric::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::PerformanceMetrics.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
