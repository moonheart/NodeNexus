use crate::db::orm::entity::Entity;
use orm_macros::Entity;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Entity)]
#[entity(table_name = "vps_tags")]
pub struct Model {
    #[entity(primary_key)]
    pub vps_id: i32,
    #[entity(primary_key)]
    pub tag_id: i32,
}
