use crate::db::duckdb_service::DuckDbPool;
use crate::db::entities::tag;
use crate::web::error::AppError;
use chrono::Utc;
use duckdb::{params, Row, Result as DuckDbResult};
use serde::Serialize;

#[derive(Serialize, Debug)]
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

fn row_to_tag_model(row: &Row) -> DuckDbResult<tag::Model> {
    Ok(tag::Model {
        id: row.get("id")?,
        user_id: row.get("user_id")?,
        name: row.get("name")?,
        color: row.get("color")?,
        icon: row.get("icon")?,
        url: row.get("url")?,
        is_visible: row.get("is_visible")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn row_to_tag_with_count(row: &Row) -> DuckDbResult<TagWithCount> {
    Ok(TagWithCount {
        id: row.get("id")?,
        user_id: row.get("user_id")?,
        name: row.get("name")?,
        color: row.get("color")?,
        icon: row.get("icon")?,
        url: row.get("url")?,
        is_visible: row.get("is_visible")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        vps_count: row.get("vps_count")?,
    })
}

pub async fn create_tag(
    pool: DuckDbPool,
    user_id: i32,
    name: &str,
    color: &str,
    icon: Option<&str>,
    url: Option<&str>,
    is_visible: bool,
) -> Result<tag::Model, AppError> {
    let conn = pool.get()?;
    let now = Utc::now();
    let new_tag = conn.query_row(
        "INSERT INTO tags (user_id, name, color, icon, url, is_visible, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?) RETURNING *",
        params![user_id, name, color, icon, url, is_visible, now, now],
        row_to_tag_model,
    )?;
    Ok(new_tag)
}

pub async fn get_tags_by_user_id_with_count(
    pool: DuckDbPool,
    user_id: i32,
) -> Result<Vec<TagWithCount>, AppError> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT t.*, COUNT(vt.vps_id) as vps_count
         FROM tags t
         LEFT JOIN vps_tags vt ON t.id = vt.tag_id
         WHERE t.user_id = ?
         GROUP BY t.id, t.user_id, t.name, t.color, t.icon, t.url, t.is_visible, t.created_at, t.updated_at
         ORDER BY t.name ASC",
    )?;
    let tags = stmt
        .query_map(params![user_id], row_to_tag_with_count)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tags)
}

pub async fn update_tag(
    pool: DuckDbPool,
    tag_id: i32,
    user_id: i32,
    name: &str,
    color: &str,
    icon: Option<&str>,
    url: Option<&str>,
    is_visible: bool,
) -> Result<tag::Model, AppError> {
    let conn = pool.get()?;
    let now = Utc::now();
    let res = conn.query_row(
        "UPDATE tags SET name = ?, color = ?, icon = ?, url = ?, is_visible = ?, updated_at = ? WHERE id = ? AND user_id = ? RETURNING *",
        params![name, color, icon, url, is_visible, now, tag_id, user_id],
        row_to_tag_model,
    );
    
    match res {
        Ok(tag) => Ok(tag),
        Err(duckdb::Error::QueryReturnedNoRows) => Err(AppError::NotFound(format!("Tag with id {tag_id} not found for user {user_id}"))),
        Err(e) => Err(e.into()),
    }
}

pub async fn delete_tag(pool: DuckDbPool, tag_id: i32, user_id: i32) -> Result<u64, AppError> {
    let conn = pool.get()?;
    let rows_affected = conn.execute(
        "DELETE FROM tags WHERE id = ? AND user_id = ?",
        params![tag_id, user_id],
    )?;
    if rows_affected == 0 {
        return Err(AppError::NotFound(format!("Tag with id {tag_id} not found for user {user_id}")));
    }
    Ok(rows_affected as u64)
}

pub async fn add_tag_to_vps(
    pool: DuckDbPool,
    vps_id: i32,
    tag_id: i32,
) -> Result<u64, AppError> {
    let conn = pool.get()?;
    let rows_affected = conn.execute(
        "INSERT INTO vps_tags (vps_id, tag_id) VALUES (?, ?) ON CONFLICT (vps_id, tag_id) DO NOTHING",
        params![vps_id, tag_id],
    )?;
    Ok(rows_affected as u64)
}

pub async fn remove_tag_from_vps(
    pool: DuckDbPool,
    vps_id: i32,
    tag_id: i32,
) -> Result<u64, AppError> {
    let conn = pool.get()?;
    let rows_affected = conn.execute(
        "DELETE FROM vps_tags WHERE vps_id = ? AND tag_id = ?",
        params![vps_id, tag_id],
    )?;
    Ok(rows_affected as u64)
}

pub async fn get_tags_for_vps(
    pool: DuckDbPool,
    vps_id: i32,
) -> Result<Vec<tag::Model>, AppError> {
    let conn = pool.get()?;
    let mut stmt = conn.prepare(
        "SELECT t.* FROM tags t
         INNER JOIN vps_tags vt ON t.id = vt.tag_id
         WHERE vt.vps_id = ?
         ORDER BY t.name ASC",
    )?;
    let tags = stmt
        .query_map(params![vps_id], row_to_tag_model)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(tags)
}

pub async fn bulk_update_vps_tags(
    pool: DuckDbPool,
    user_id: i32,
    vps_ids: &[i32],
    add_tag_ids: &[i32],
    remove_tag_ids: &[i32],
) -> Result<(), AppError> {
    if vps_ids.is_empty() || (add_tag_ids.is_empty() && remove_tag_ids.is_empty()) {
        return Ok(());
    }

    let mut conn = pool.get()?;
    let tx = conn.transaction()?;

    // Authorize
    if !vps_ids.is_empty() {
        let params_sql = vps_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("SELECT COUNT(*) FROM vps WHERE id IN ({params_sql}) AND user_id = ?");
        
        let mut params_vec: Vec<&dyn duckdb::ToSql> = vps_ids.iter().map(|id| id as &dyn duckdb::ToSql).collect();
        params_vec.push(&user_id);

        let owned_vps_count: i64 = tx.query_row(&sql, &params_vec[..], |row| row.get(0))?;

        if owned_vps_count != vps_ids.len() as i64 {
            return Err(AppError::Forbidden("User does not own all specified VPS".to_string()));
        }
    }

    // Bulk add
    if !add_tag_ids.is_empty() {
        let mut stmt = tx.prepare("INSERT INTO vps_tags (vps_id, tag_id) VALUES (?, ?) ON CONFLICT (vps_id, tag_id) DO NOTHING")?;
        for v_id in vps_ids {
            for t_id in add_tag_ids {
                stmt.execute(params![v_id, t_id])?;
            }
        }
    }

    // Bulk remove
    if !remove_tag_ids.is_empty() {
        let vps_params_sql = vps_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let tag_params_sql = remove_tag_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("DELETE FROM vps_tags WHERE vps_id IN ({vps_params_sql}) AND tag_id IN ({tag_params_sql})");

        let mut params_vec: Vec<&dyn duckdb::ToSql> = vps_ids.iter().map(|id| id as &dyn duckdb::ToSql).collect();
        let mut tag_params_vec: Vec<&dyn duckdb::ToSql> = remove_tag_ids.iter().map(|id| id as &dyn duckdb::ToSql).collect();
        params_vec.append(&mut tag_params_vec);
        
        tx.execute(&sql, &params_vec[..])?;
    }

    tx.commit()?;
    Ok(())
}