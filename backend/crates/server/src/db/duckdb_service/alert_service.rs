use chrono::Utc;
use duckdb::{params, Connection, Result as DuckDbResult, ToSql};
use std::collections::HashMap;
use tokio::task;

use crate::db::duckdb_service::DuckDbPool;
use crate::db::entities::alert_rule;
use crate::db::models::AlertRule;
use crate::web::error::AppError;
use crate::web::models::alert_models::{CreateAlertRuleRequest, UpdateAlertRuleRequest};

pub async fn create_alert_rule(
    pool: DuckDbPool,
    user_id: i32,
    payload: CreateAlertRuleRequest,
) -> Result<AlertRule, AppError> {
    task::spawn_blocking(move || {
        let mut conn = pool.get().map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let tx = conn.transaction().map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let cooldown_seconds = payload.cooldown_seconds.unwrap_or(300);
        let now = Utc::now();

        let new_rule_model = {
            let vps_id_val = payload.vps_id;
            let id: i32 = tx.query_row(
                "INSERT INTO alert_rules (user_id, name, vps_id, metric_type, threshold, comparison_operator, duration_seconds, cooldown_seconds, is_active, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
                params![
                    user_id,
                    payload.name,
                    vps_id_val,
                    payload.metric_type,
                    payload.threshold,
                    payload.comparison_operator,
                    payload.duration_seconds,
                    cooldown_seconds,
                    true, // is_active
                    now,
                    now,
                ],
                |row| row.get(0)
            ).map_err(|e| AppError::DatabaseError(e.to_string()))?;

            alert_rule::Model {
                id,
                user_id,
                name: payload.name,
                vps_id: payload.vps_id,
                metric_type: payload.metric_type,
                threshold: payload.threshold,
                comparison_operator: payload.comparison_operator,
                duration_seconds: payload.duration_seconds,
                is_active: true,
                last_triggered_at: None,
                cooldown_seconds,
                created_at: now,
                updated_at: now,
            }
        };

        let mut notification_channel_ids_to_link = Vec::new();
        if let Some(channel_ids) = payload.notification_channel_ids {
            if !channel_ids.is_empty() {
                link_channels_to_rule(&tx, new_rule_model.id, &channel_ids)?;
                notification_channel_ids_to_link = channel_ids;
            }
        }

        tx.commit().map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(AlertRule {
            id: new_rule_model.id,
            user_id: new_rule_model.user_id,
            name: new_rule_model.name,
            vps_id: new_rule_model.vps_id,
            metric_type: new_rule_model.metric_type,
            threshold: new_rule_model.threshold,
            comparison_operator: new_rule_model.comparison_operator,
            duration_seconds: new_rule_model.duration_seconds,
            notification_channel_ids: Some(notification_channel_ids_to_link),
            is_active: new_rule_model.is_active,
            last_triggered_at: new_rule_model.last_triggered_at,
            cooldown_seconds: new_rule_model.cooldown_seconds,
            created_at: new_rule_model.created_at,
            updated_at: new_rule_model.updated_at,
        })
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
}

fn link_channels_to_rule(
    tx: &duckdb::Transaction,
    rule_id: i32,
    channel_ids: &[i32],
) -> Result<(), AppError> {
    if channel_ids.is_empty() {
        return Ok(());
    }
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO alert_rule_channels (alert_rule_id, channel_id) VALUES (?, ?)",
    ).map_err(|e| AppError::DatabaseError(e.to_string()))?;

    for &channel_id in channel_ids {
        stmt.execute(params![rule_id, channel_id]).map_err(|e| AppError::DatabaseError(e.to_string()))?;
    }
    Ok(())
}

fn row_to_alert_rule_model(row: &duckdb::Row<'_>) -> DuckDbResult<alert_rule::Model> {
    Ok(alert_rule::Model {
        id: row.get(0)?,
        user_id: row.get(1)?,
        name: row.get(2)?,
        vps_id: row.get(3)?,
        metric_type: row.get(4)?,
        threshold: row.get(5)?,
        comparison_operator: row.get(6)?,
        duration_seconds: row.get(7)?,
        is_active: row.get(8)?,
        last_triggered_at: row.get(9)?,
        cooldown_seconds: row.get(10)?,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

pub async fn get_all_alert_rules_for_user(
    pool: DuckDbPool,
    user_id: i32,
) -> Result<Vec<AlertRule>, AppError> {
    task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT * FROM alert_rules WHERE user_id = ? ORDER BY name ASC")
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let rule_models = stmt
            .query_map(params![user_id], row_to_alert_rule_model)
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if rule_models.is_empty() {
            return Ok(Vec::new());
        }

        let rule_ids: Vec<i32> = rule_models.iter().map(|r| r.id).collect();
        let mut channels_map = get_linked_channels_for_rules_sync(&conn, &rule_ids)?;

        let full_rules = rule_models
            .into_iter()
            .map(|rule_model| AlertRule {
                notification_channel_ids: channels_map.remove(&rule_model.id),
                id: rule_model.id,
                user_id: rule_model.user_id,
                name: rule_model.name,
                vps_id: rule_model.vps_id,
                metric_type: rule_model.metric_type,
                threshold: rule_model.threshold,
                comparison_operator: rule_model.comparison_operator,
                duration_seconds: rule_model.duration_seconds,
                is_active: rule_model.is_active,
                last_triggered_at: rule_model.last_triggered_at,
                cooldown_seconds: rule_model.cooldown_seconds,
                created_at: rule_model.created_at,
                updated_at: rule_model.updated_at,
            })
            .collect();

        Ok(full_rules)
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
}

pub async fn get_alert_rule_by_id_for_user(
    pool: DuckDbPool,
    rule_id: i32,
    user_id: i32,
) -> Result<AlertRule, AppError> {
    task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT * FROM alert_rules WHERE id = ? AND user_id = ?")
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let rule_model = stmt
            .query_row(params![rule_id, user_id], row_to_alert_rule_model)
            .map_err(|e| {
                if let duckdb::Error::QueryReturnedNoRows = e {
                    AppError::NotFound("Alert rule not found".to_string())
                } else {
                    AppError::DatabaseError(e.to_string())
                }
            })?;

        let channel_ids = get_linked_channel_ids_sync(&conn, rule_model.id)?;
        Ok(AlertRule {
            id: rule_model.id,
            user_id: rule_model.user_id,
            name: rule_model.name,
            vps_id: rule_model.vps_id,
            metric_type: rule_model.metric_type,
            threshold: rule_model.threshold,
            comparison_operator: rule_model.comparison_operator,
            duration_seconds: rule_model.duration_seconds,
            notification_channel_ids: Some(channel_ids),
            is_active: rule_model.is_active,
            last_triggered_at: rule_model.last_triggered_at,
            cooldown_seconds: rule_model.cooldown_seconds,
            created_at: rule_model.created_at,
            updated_at: rule_model.updated_at,
        })
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
}

fn get_linked_channel_ids_sync(conn: &Connection, rule_id: i32) -> Result<Vec<i32>, AppError> {
    let mut stmt = conn
        .prepare("SELECT channel_id FROM alert_rule_channels WHERE alert_rule_id = ?")
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let channel_ids = stmt
        .query_map(params![rule_id], |row| row.get(0))
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .collect::<Result<Vec<i32>, _>>()
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    Ok(channel_ids)
}

fn get_linked_channels_for_rules_sync(
    conn: &Connection,
    rule_ids: &[i32],
) -> Result<HashMap<i32, Vec<i32>>, AppError> {
    if rule_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let params_sql = rule_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
    let sql = format!(
        "SELECT alert_rule_id, channel_id FROM alert_rule_channels WHERE alert_rule_id IN ({})",
        params_sql
    );

    let mut params_vec: Vec<&dyn ToSql> = Vec::new();
    for id in rule_ids {
        params_vec.push(id);
    }

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    let mut map: HashMap<i32, Vec<i32>> = HashMap::new();
    let rows = stmt
        .query_map(&params_vec[..], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    for row in rows {
        let (rule_id, channel_id): (i32, i32) =
            row.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        map.entry(rule_id).or_default().push(channel_id);
    }

    Ok(map)
}

pub async fn update_alert_rule(
    pool: DuckDbPool,
    rule_id: i32,
    user_id: i32,
    payload: UpdateAlertRuleRequest,
) -> Result<AlertRule, AppError> {
    let pool_clone = pool.clone();
    task::spawn_blocking(move || {
        let mut conn = pool_clone.get().map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let tx = conn.transaction().map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut set_clauses: Vec<String> = Vec::new();
        let mut params_vec: Vec<&dyn ToSql> = Vec::new();

        if let Some(name) = &payload.name {
            set_clauses.push("name = ?".to_string());
            params_vec.push(name);
        }
        if let Some(vps_id) = &payload.vps_id {
            set_clauses.push("vps_id = ?".to_string());
            params_vec.push(vps_id);
        }
        if let Some(metric_type) = &payload.metric_type {
            set_clauses.push("metric_type = ?".to_string());
            params_vec.push(metric_type);
        }
        if let Some(threshold) = &payload.threshold {
            set_clauses.push("threshold = ?".to_string());
            params_vec.push(threshold);
        }
        if let Some(comparison_operator) = &payload.comparison_operator {
            set_clauses.push("comparison_operator = ?".to_string());
            params_vec.push(comparison_operator);
        }
        if let Some(duration_seconds) = &payload.duration_seconds {
            set_clauses.push("duration_seconds = ?".to_string());
            params_vec.push(duration_seconds);
        }
        if let Some(cooldown_seconds) = &payload.cooldown_seconds {
            set_clauses.push("cooldown_seconds = ?".to_string());
            params_vec.push(cooldown_seconds);
        }

        if !set_clauses.is_empty() {
            let now = Utc::now();
            set_clauses.push("updated_at = ?".to_string());
            params_vec.push(&now);

            let sql = format!(
                "UPDATE alert_rules SET {} WHERE id = ? AND user_id = ?",
                set_clauses.join(", ")
            );
            
            let mut final_params = params_vec;
            final_params.push(&rule_id);
            final_params.push(&user_id);

            let num_updated = tx.execute(&sql, &final_params[..]).map_err(|e| AppError::DatabaseError(e.to_string()))?;

            if num_updated == 0 {
                return Err(AppError::NotFound("Alert rule not found or not owned by user".to_string()));
            }
        }

        if let Some(channel_ids) = &payload.notification_channel_ids {
            tx.execute("DELETE FROM alert_rule_channels WHERE alert_rule_id = ?", params![rule_id])
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;
            if !channel_ids.is_empty() {
                link_channels_to_rule(&tx, rule_id, channel_ids)?;
            }
        }

        tx.commit().map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))??;

    get_alert_rule_by_id_for_user(pool, rule_id, user_id).await
}

pub async fn delete_alert_rule(pool: DuckDbPool, rule_id: i32, user_id: i32) -> Result<(), AppError> {
    task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let rows_affected = conn.execute(
            "DELETE FROM alert_rules WHERE id = ? AND user_id = ?",
            params![rule_id, user_id],
        ).map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if rows_affected == 0 {
            Err(AppError::NotFound(
                "Alert rule not found or not owned by user".to_string(),
            ))
        } else {
            Ok(())
        }
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
}

pub async fn get_all_active_rules_for_evaluation(
    pool: DuckDbPool,
) -> Result<Vec<alert_rule::Model>, AppError> {
    task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT * FROM alert_rules WHERE is_active = true ORDER BY id ASC")
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        stmt.query_map([], row_to_alert_rule_model)
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| AppError::DatabaseError(e.to_string()))
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
}

pub async fn update_alert_rule_last_triggered(
    pool: DuckDbPool,
    rule_id: i32,
    user_id: i32,
) -> Result<(), AppError> {
    task::spawn_blocking(move || {
        let conn = pool.get().map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let rows_affected = conn.execute(
            "UPDATE alert_rules SET last_triggered_at = ?, updated_at = ? WHERE id = ? AND user_id = ?",
            params![Utc::now(), Utc::now(), rule_id, user_id],
        )
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if rows_affected == 0 {
            Err(AppError::NotFound("Alert rule not found or not owned by user".to_string()))
        } else {
            Ok(())
        }
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))?
}

pub async fn update_alert_rule_status(
    pool: DuckDbPool,
    rule_id: i32,
    user_id: i32,
    is_active: bool,
) -> Result<AlertRule, AppError> {
    let pool_clone = pool.clone();
    task::spawn_blocking(move || {
        let conn = pool_clone.get().map_err(|e| AppError::DatabaseError(e.to_string()))?;
        let rows_affected = conn.execute(
            "UPDATE alert_rules SET is_active = ?, updated_at = ? WHERE id = ? AND user_id = ?",
            params![is_active, Utc::now(), rule_id, user_id],
        ).map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if rows_affected == 0 {
            return Err(AppError::NotFound(
                "Alert rule not found or not owned by user".to_string(),
            ));
        }
        Ok(())
    })
    .await
    .map_err(|e| AppError::InternalServerError(e.to_string()))??;

    get_alert_rule_by_id_for_user(pool, rule_id, user_id).await
}