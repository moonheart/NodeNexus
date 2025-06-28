use chrono::Utc;
use sea_orm::{
    prelude::Expr, ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    IntoActiveModel, QueryFilter, Set, UpdateResult,
};

use crate::db::entities::{prelude::Vps, setting, vps}; // Assuming prelude exports Vps entity

// --- Settings Service Functions ---

/// Retrieves a setting by its key.
pub async fn get_setting(
    db: &DatabaseConnection,
    key: &str,
) -> Result<Option<setting::Model>, DbErr> {
    setting::Entity::find_by_id(key.to_owned()).one(db).await
}

/// Creates or updates a setting.
pub async fn update_setting(
    db: &DatabaseConnection,
    key: &str,
    value: &serde_json::Value,
) -> Result<setting::ActiveModel, DbErr> {
    let now = Utc::now();
    let active_setting = setting::ActiveModel {
        key: Set(key.to_owned()),
        value: Set(value.clone()),
        updated_at: Set(now),
    };
    // .save() handles INSERT ON CONFLICT DO UPDATE for entities with a primary key
    active_setting.save(db).await
}

/// Updates a VPS's config override field.
pub async fn update_vps_config_override(
    db: &DatabaseConnection,
    vps_id: i32,
    user_id: i32, // For authorization
    config_override: &serde_json::Value,
) -> Result<UpdateResult, DbErr> {
    let now = Utc::now();
    Vps::update_many()
        .col_expr(vps::Column::AgentConfigOverride, Expr::value(sea_orm::Value::Json(Some(Box::new(config_override.clone())))))
        .col_expr(vps::Column::UpdatedAt, Expr::value(sea_orm::Value::ChronoDateTimeUtc(Some(Box::new(now)))))
        .filter(vps::Column::Id.eq(vps_id))
        .filter(vps::Column::UserId.eq(user_id))
        .exec(db)
        .await
}

/// Updates the config status of a VPS.
pub async fn update_vps_config_status(
    db: &DatabaseConnection,
    vps_id: i32,
    status: &str,
    error: Option<&str>,
) -> Result<UpdateResult, DbErr> {
    let now = Utc::now();
    let vps_model = vps::Entity::find_by_id(vps_id).one(db).await?;

    if let Some(vps_model) = vps_model {
        let mut active_vps: vps::ActiveModel = vps_model.into_active_model();
        active_vps.config_status = Set(status.to_owned()); // Corrected: Removed Some()
        active_vps.last_config_error = Set(error.map(|e| e.to_owned()));
        active_vps.last_config_update_at = Set(Some(now));
        active_vps.updated_at = Set(now);
        active_vps.update(db).await
        .map_err(|e| {
            DbErr::Custom(format!("Failed to update VPS config status: {e}"))
        })
        .map(|_res| {
            UpdateResult {
                rows_affected: 1
            }
        })
        
    } else {
        // Or handle as an error: Err(DbErr::RecordNotFound(format!("VPS with id {} not found", vps_id)))
        Ok(UpdateResult { rows_affected: 0 })
    }
}