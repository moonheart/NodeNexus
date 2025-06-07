use chrono::Utc;
use sqlx::{FromRow, PgPool, Result};

use crate::db::models::Tag;

// --- Tag Service Functions ---

/// A struct that includes a Tag and its usage count.
#[derive(FromRow, serde::Serialize, Debug)]
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
    pool: &PgPool,
    user_id: i32,
    name: &str,
    color: &str,
    icon: Option<&str>,
    url: Option<&str>,
    is_visible: bool,
) -> Result<Tag> {
    let now = Utc::now();
    sqlx::query_as!(
        Tag,
        r#"
        INSERT INTO tags (user_id, name, color, icon, url, is_visible, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, user_id, name, color, icon, url, is_visible, created_at, updated_at
        "#,
        user_id,
        name,
        color,
        icon,
        url,
        is_visible,
        now,
        now
    )
    .fetch_one(pool)
    .await
}

/// Retrieves all tags for a user, including a count of how many VPS use each tag.
pub async fn get_tags_by_user_id_with_count(
    pool: &PgPool,
    user_id: i32,
) -> Result<Vec<TagWithCount>> {
    sqlx::query_as!(
        TagWithCount,
        r#"
        SELECT
            t.id, t.user_id, t.name, t.color, t.icon, t.url, t.is_visible, t.created_at, t.updated_at,
            COALESCE(COUNT(vt.vps_id), 0) as "vps_count!"
        FROM tags t
        LEFT JOIN vps_tags vt ON t.id = vt.tag_id
        WHERE t.user_id = $1
        GROUP BY t.id
        ORDER BY t.name
        "#,
        user_id
    )
    .fetch_all(pool)
    .await
}

/// Updates an existing tag.
pub async fn update_tag(
    pool: &PgPool,
    tag_id: i32,
    user_id: i32, // for authorization
    name: &str,
    color: &str,
    icon: Option<&str>,
    url: Option<&str>,
    is_visible: bool,
) -> Result<u64> {
    let now = Utc::now();
    let rows_affected = sqlx::query!(
        r#"
        UPDATE tags
        SET name = $1, color = $2, icon = $3, url = $4, is_visible = $5, updated_at = $6
        WHERE id = $7 AND user_id = $8
        "#,
        name,
        color,
        icon,
        url,
        is_visible,
        now,
        tag_id,
        user_id
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows_affected)
}

/// Deletes a tag. The ON DELETE CASCADE in the DB will handle vps_tags entries.
pub async fn delete_tag(pool: &PgPool, tag_id: i32, user_id: i32) -> Result<u64> {
    let rows_affected = sqlx::query!(
        "DELETE FROM tags WHERE id = $1 AND user_id = $2",
        tag_id,
        user_id
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows_affected)
}

/// Associates a tag with a VPS. Ignores conflicts if the association already exists.
pub async fn add_tag_to_vps(pool: &PgPool, vps_id: i32, tag_id: i32) -> Result<u64> {
    let rows_affected = sqlx::query!(
        "INSERT INTO vps_tags (vps_id, tag_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
        vps_id,
        tag_id
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows_affected)
}

/// Removes a tag from a VPS.
pub async fn remove_tag_from_vps(pool: &PgPool, vps_id: i32, tag_id: i32) -> Result<u64> {
    let rows_affected = sqlx::query!(
        "DELETE FROM vps_tags WHERE vps_id = $1 AND tag_id = $2",
        vps_id,
        tag_id
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows_affected)
}

/// Retrieves all tags for a specific VPS.
pub async fn get_tags_for_vps(pool: &PgPool, vps_id: i32) -> Result<Vec<Tag>> {
    sqlx::query_as!(
        Tag,
        r#"
        SELECT t.id, t.user_id, t.name, t.color, t.icon, t.url, t.is_visible, t.created_at, t.updated_at
        FROM tags t
        INNER JOIN vps_tags vt ON t.id = vt.tag_id
        WHERE vt.vps_id = $1
        ORDER BY t.name
        "#,
        vps_id
    )
    .fetch_all(pool)
    .await
}

/// Bulk adds/removes tags for a list of VPS.
/// This function performs operations in a single transaction.
pub async fn bulk_update_vps_tags(
    pool: &PgPool,
    user_id: i32, // For authorization
    vps_ids: &[i32],
    add_tag_ids: &[i32],
    remove_tag_ids: &[i32],
) -> Result<(), sqlx::Error> {
    if vps_ids.is_empty() {
        return Ok(()); // Nothing to do
    }

    let mut tx = pool.begin().await?;

    // Authorize: Ensure the user owns all the VPS they are trying to modify.
    let owned_vps_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM vps WHERE id = ANY($1) AND user_id = $2",
        vps_ids,
        user_id
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(0);

    if owned_vps_count != vps_ids.len() as i64 {
        // Use RowNotFound to signal an authorization failure to the handler.
        return Err(sqlx::Error::RowNotFound);
    }

    // Bulk add tags
    if !add_tag_ids.is_empty() {
        sqlx::query!(
            r#"
            INSERT INTO vps_tags (vps_id, tag_id)
            SELECT vps_id, tag_id
            FROM UNNEST($1::int[]) as vps_id, UNNEST($2::int[]) as tag_id
            ON CONFLICT (vps_id, tag_id) DO NOTHING
            "#,
            vps_ids,
            add_tag_ids
        )
        .execute(&mut *tx)
        .await?;
    }

    // Bulk remove tags
    if !remove_tag_ids.is_empty() {
        sqlx::query!(
            r#"
            DELETE FROM vps_tags
            WHERE vps_id = ANY($1) AND tag_id = ANY($2)
            "#,
            vps_ids,
            remove_tag_ids
        )
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(())
}