use chrono::Utc;
use sea_orm::{
    sea_query::OnConflict, ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, DeleteResult,
    EntityTrait, FromQueryResult, InsertResult, IntoActiveModel, JoinType, // Removed ModelTrait
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait, Set, TransactionTrait,
};

use crate::db::entities::{tag, vps, vps_tag}; // Removed prelude::*

// --- Tag Service Functions ---

/// A struct that includes a Tag and its usage count.
#[derive(FromQueryResult, serde::Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TagWithCount {
    pub id: i32,
    pub user_id: i32,
    pub name: String,
    pub color: String,
    pub icon: Option<String>,
    pub url: Option<String>,
    pub is_visible: bool,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub vps_count: i64,
}

/// Creates a new tag for a user.
pub async fn create_tag(
    db: &DatabaseConnection,
    user_id: i32,
    name: &str,
    color: &str,
    icon: Option<&str>,
    url: Option<&str>,
    is_visible: bool,
) -> Result<tag::Model, DbErr> {
    let now = Utc::now();
    let new_tag = tag::ActiveModel {
        user_id: Set(user_id),
        name: Set(name.to_owned()),
        color: Set(color.to_owned()),
        icon: Set(icon.map(|s| s.to_owned())),
        url: Set(url.map(|s| s.to_owned())),
        is_visible: Set(is_visible),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default() // For id
    };
    new_tag.insert(db).await
}

/// Retrieves all tags for a user, including a count of how many VPS use each tag.
pub async fn get_tags_by_user_id_with_count(
    db: &DatabaseConnection,
    user_id: i32,
) -> Result<Vec<TagWithCount>, DbErr> {
    tag::Entity::find()
        .select_only()
        .column(tag::Column::Id)
        .column(tag::Column::UserId)
        .column(tag::Column::Name)
        .column(tag::Column::Color)
        .column(tag::Column::Icon)
        .column(tag::Column::Url)
        .column(tag::Column::IsVisible)
        .column(tag::Column::CreatedAt)
        .column(tag::Column::UpdatedAt)
        .column_as(vps_tag::Column::VpsId.count(), "vps_count")
        .join(JoinType::LeftJoin, crate::db::entities::vps_tag::Relation::Tag.def().rev())
        .filter(tag::Column::UserId.eq(user_id))
        .group_by(tag::Column::Id)
        .group_by(tag::Column::UserId)
        .group_by(tag::Column::Name)
        .group_by(tag::Column::Color)
        .group_by(tag::Column::Icon)
        .group_by(tag::Column::Url)
        .group_by(tag::Column::IsVisible)
        .group_by(tag::Column::CreatedAt)
        .group_by(tag::Column::UpdatedAt)
        .order_by_asc(tag::Column::Name)
        .into_model::<TagWithCount>()
        .all(db)
        .await
}

/// Updates an existing tag.
pub async fn update_tag(
    db: &DatabaseConnection,
    tag_id: i32,
    user_id: i32, // for authorization
    name: &str,
    color: &str,
    icon: Option<&str>,
    url: Option<&str>,
    is_visible: bool,
) -> Result<tag::Model, DbErr> {
    let now = Utc::now();
    let tag_to_update = tag::Entity::find_by_id(tag_id)
        .filter(tag::Column::UserId.eq(user_id))
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound(format!("Tag with id {tag_id} not found for user {user_id}")))?;

    let mut active_tag: tag::ActiveModel = tag_to_update.into_active_model();
    active_tag.name = Set(name.to_owned());
    active_tag.color = Set(color.to_owned());
    active_tag.icon = Set(icon.map(|s| s.to_owned()));
    active_tag.url = Set(url.map(|s| s.to_owned()));
    active_tag.is_visible = Set(is_visible);
    active_tag.updated_at = Set(now);
    active_tag.update(db).await
}

/// Deletes a tag. The ON DELETE CASCADE in the DB will handle vps_tags entries.
pub async fn delete_tag(db: &DatabaseConnection, tag_id: i32, user_id: i32) -> Result<DeleteResult, DbErr> {
    tag::Entity::delete_many()
        .filter(tag::Column::Id.eq(tag_id))
        .filter(tag::Column::UserId.eq(user_id))
        .exec(db)
        .await
}

/// Associates a tag with a VPS. Ignores conflicts if the association already exists.
/// Returns InsertResult which includes rows_affected.
pub async fn add_tag_to_vps(db: &DatabaseConnection, vps_id: i32, tag_id: i32) -> Result<InsertResult<vps_tag::ActiveModel>, DbErr> {
    vps_tag::Entity::insert(vps_tag::ActiveModel {
        vps_id: Set(vps_id),
        tag_id: Set(tag_id),
    })
    .on_conflict(
        OnConflict::columns([vps_tag::Column::VpsId, vps_tag::Column::TagId])
            .do_nothing()
            .to_owned(),
    )
    .exec(db)
    .await
}


/// Removes a tag from a VPS.
pub async fn remove_tag_from_vps(db: &DatabaseConnection, vps_id: i32, tag_id: i32) -> Result<DeleteResult, DbErr> {
    vps_tag::Entity::delete_by_id((vps_id, tag_id)).exec(db).await
}

/// Retrieves all tags for a specific VPS.
pub async fn get_tags_for_vps(db: &DatabaseConnection, vps_id: i32) -> Result<Vec<tag::Model>, DbErr> {
    // Assuming Vps entity has a find_related(tag::Entity) through VpsTag
    // If not, the manual join is correct. Let's use the manual join for clarity as per plan.
    tag::Entity::find()
        .join(JoinType::InnerJoin, crate::db::entities::vps_tag::Relation::Tag.def().rev())
        .filter(vps_tag::Column::VpsId.eq(vps_id))
        .order_by_asc(tag::Column::Name)
        .all(db)
        .await
}

/// Bulk adds/removes tags for a list of VPS.
/// This function performs operations in a single transaction.
pub async fn bulk_update_vps_tags(
    db: &DatabaseConnection,
    user_id: i32, // For authorization
    vps_ids: &[i32],
    add_tag_ids: &[i32],
    remove_tag_ids: &[i32],
) -> Result<(), DbErr> {
    if vps_ids.is_empty() && (add_tag_ids.is_empty() && remove_tag_ids.is_empty()) {
        return Ok(()); // Nothing to do
    }

    let txn = db.begin().await?;

    // Authorize: Ensure the user owns all the VPS they are trying to modify.
    if !vps_ids.is_empty() {
        let owned_vps_count = vps::Entity::find()
            .filter(vps::Column::Id.is_in(vps_ids.to_vec()))
            .filter(vps::Column::UserId.eq(user_id))
            .count(&txn)
            .await?;

        if owned_vps_count != vps_ids.len() as u64 {
            txn.rollback().await?; // Rollback before returning error
            return Err(DbErr::Custom(
                "Authorization failed: User does not own all specified VPS".to_string(),
            ));
        }
    }


    // Bulk add tags
    if !add_tag_ids.is_empty() && !vps_ids.is_empty() {
        let mut batch_inserts = Vec::new();
        for v_id in vps_ids {
            for t_id in add_tag_ids {
                batch_inserts.push(vps_tag::ActiveModel {
                    vps_id: Set(*v_id),
                    tag_id: Set(*t_id),
                });
            }
        }
        if !batch_inserts.is_empty() {
            vps_tag::Entity::insert_many(batch_inserts)
                .on_conflict(
                    OnConflict::columns([vps_tag::Column::VpsId, vps_tag::Column::TagId])
                        .do_nothing()
                        .to_owned(),
                )
                .exec(&txn)
                .await?;
        }
    }

    // Bulk remove tags
    if !remove_tag_ids.is_empty() && !vps_ids.is_empty() {
        vps_tag::Entity::delete_many()
            .filter(vps_tag::Column::VpsId.is_in(vps_ids.to_vec()))
            .filter(vps_tag::Column::TagId.is_in(remove_tag_ids.to_vec()))
            .exec(&txn)
            .await?;
    }

    txn.commit().await?;

    Ok(())
}