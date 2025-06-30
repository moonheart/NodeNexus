use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    #[sea_orm(unique)] // Assuming username is unique
    pub username: String,
    pub password_hash: Option<String>,
    pub role: String,
    pub password_login_disabled: bool,
    pub created_at: ChronoDateTimeUtc,
    pub updated_at: ChronoDateTimeUtc,
    pub theme_mode: String,
    pub active_theme_id: Option<Uuid>,
    pub language: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::vps::Entity")]
    Vps,

    #[sea_orm(has_many = "super::tag::Entity")]
    Tags,

    #[sea_orm(has_many = "super::user_identity_provider::Entity")]
    UserIdentityProviders,

    #[sea_orm(has_many = "super::theme::Entity")]
    Themes,
}

impl Related<super::vps::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Vps.def()
    }
}

impl Related<super::theme::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Themes.def()
    }
}

impl Related<super::tag::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Tags.def()
    }
}

impl Related<super::user_identity_provider::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserIdentityProviders.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
