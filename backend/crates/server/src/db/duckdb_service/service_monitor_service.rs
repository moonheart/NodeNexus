//! Service for managing service monitors and their results.
//!
//! This service provides functions for CRUD operations on service monitors,
//! assigning them to agents/tags, and recording check results.

use crate::db::duckdb_service::DuckDbPool;
use crate::db::entities::{
    service_monitor,
};
use crate::web::error::AppError;
use crate::web::models::service_monitor_models::{
    CreateMonitor, ServiceMonitorDetails, UpdateMonitor,
};
use chrono::{DateTime, Utc};
use nodenexus_common::agent_service::{ServiceMonitorResult, ServiceMonitorTask};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceMonitorPoint {
    pub time: DateTime<Utc>,
    pub monitor_id: i32,
    pub agent_id: i32,
    pub is_up: Option<f64>,
    pub latency_ms: Option<f64>,
    pub details: Option<serde_json::Value>,
}

use duckdb::{params, params_from_iter, OptionalExt, Result as DuckDbResult, Row};
use crate::db::duckdb_service::json_from_row;

// A helper function to generate `(?, ?, ...)` placeholder strings for `IN` clauses.
fn repeat_vars(count: usize) -> String {
    if count == 0 {
        // Return a value that will result in an empty set, but is valid SQL.
        return "()".to_string();
    }
    let mut s = "?,".repeat(count);
    s.pop(); // Remove the trailing comma
    format!("({s})")
}

fn row_to_monitor_model(row: &Row) -> DuckDbResult<service_monitor::Model> {
    Ok(service_monitor::Model {
        id: row.get("id")?,
        user_id: row.get("user_id")?,
        name: row.get("name")?,
        monitor_type: row.get("monitor_type")?,
        target: row.get("target")?,
        frequency_seconds: row.get("frequency_seconds")?,
        timeout_seconds: row.get("timeout_seconds")?,
        is_active: row.get("is_active")?,
        monitor_config: json_from_row(row, "monitor_config")?,
        assignment_type: row.get("assignment_type")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub async fn create_monitor(
    pool: DuckDbPool,
    user_id: i32,
    monitor_data: CreateMonitor,
) -> Result<service_monitor::Model, AppError> {
    let mut conn = pool.get()?;
    let tx = conn.transaction()?;

    let monitor_config_str = serde_json::to_string(&monitor_data.monitor_config)?;
    let assignment_type = monitor_data.assignments.assignment_type.unwrap_or_else(|| "INCLUSIVE".to_string());

    let saved_monitor: service_monitor::Model = tx.query_row(
        "INSERT INTO service_monitors (user_id, name, monitor_type, target, frequency_seconds, timeout_seconds, is_active, monitor_config, assignment_type)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING *",
        params![
            user_id,
            monitor_data.name,
            monitor_data.monitor_type,
            monitor_data.target,
            monitor_data.frequency_seconds.unwrap_or(60),
            monitor_data.timeout_seconds.unwrap_or(10),
            monitor_data.is_active.unwrap_or(true),
            monitor_config_str,
            assignment_type,
        ],
        row_to_monitor_model,
    )?;

    if let Some(agent_ids) = monitor_data.assignments.agent_ids {
        if !agent_ids.is_empty() {
            let mut stmt = tx.prepare("INSERT INTO service_monitor_agents (monitor_id, vps_id) VALUES (?, ?)")?;
            for vps_id in agent_ids {
                stmt.execute(params![saved_monitor.id, vps_id])?;
            }
        }
    }

    if let Some(tag_ids) = monitor_data.assignments.tag_ids {
        if !tag_ids.is_empty() {
            let mut stmt = tx.prepare("INSERT INTO service_monitor_tags (monitor_id, tag_id) VALUES (?, ?)")?;
            for tag_id in tag_ids {
                stmt.execute(params![saved_monitor.id, tag_id])?;
            }
        }
    }

    tx.commit()?;
    Ok(saved_monitor)
}

pub async fn get_monitors_with_details_by_user_id(
    pool: DuckDbPool,
    user_id: i32,
) -> Result<Vec<ServiceMonitorDetails>, AppError> {
    let conn = pool.get()?;

    // 1. Fetch all monitors for the user
    let monitors: Vec<service_monitor::Model> = conn
        .prepare("SELECT * FROM service_monitors WHERE user_id = ?")?
        .query_map(params![user_id], row_to_monitor_model)?
        .collect::<Result<Vec<_>, _>>()?;

    if monitors.is_empty() {
        return Ok(Vec::new());
    }

    let monitor_ids: Vec<i32> = monitors.iter().map(|m| m.id).collect();
    
    if monitor_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = repeat_vars(monitor_ids.len());

    // 2. Fetch all agent and tag assignments for these monitors
    let agent_sql = format!("SELECT monitor_id, vps_id FROM service_monitor_agents WHERE monitor_id IN {placeholders}");
    let agent_assignments: Vec<(i32, i32)> = conn
        .prepare(&agent_sql)?
        .query_map(params_from_iter(monitor_ids.iter()), |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    let tag_sql = format!("SELECT monitor_id, tag_id FROM service_monitor_tags WHERE monitor_id IN {placeholders}");
    let tag_assignments: Vec<(i32, i32)> = conn
        .prepare(&tag_sql)?
        .query_map(params_from_iter(monitor_ids.iter()), |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    // 3. Group assignments by monitor_id
    let mut agent_map: std::collections::HashMap<i32, Vec<i32>> = std::collections::HashMap::new();
    for (monitor_id, vps_id) in agent_assignments {
        agent_map.entry(monitor_id).or_default().push(vps_id);
    }

    let mut tag_map: std::collections::HashMap<i32, Vec<i32>> = std::collections::HashMap::new();
    for (monitor_id, tag_id) in tag_assignments {
        tag_map.entry(monitor_id).or_default().push(tag_id);
    }

    // 4. Fetch the latest result for each monitor
    let latest_results_sql = format!(
        "
        SELECT monitor_id, is_up, time, details->>'message' as details
        FROM (
            SELECT *, ROW_NUMBER() OVER(PARTITION BY monitor_id ORDER BY time DESC) as rn
            FROM service_monitor_results
            WHERE monitor_id IN {placeholders}
        )
        WHERE rn = 1
        "
    );
    let latest_results: Vec<(i32, bool, DateTime<Utc>, Option<String>)> = conn
        .prepare(&latest_results_sql)?
        .query_map(params_from_iter(monitor_ids.iter()), |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let latest_result_map: std::collections::HashMap<i32, (bool, DateTime<Utc>, Option<String>)> = latest_results
        .into_iter()
        .map(|(monitor_id, is_up, time, details)| (monitor_id, (is_up, time, details)))
        .collect();

    // 5. Construct the final response models
    let details_list = monitors
        .into_iter()
        .map(|monitor| {
            let monitor_id = monitor.id;
            let last_result = latest_result_map.get(&monitor_id);

            let last_status = last_result.map(|(is_up, _, _)| if *is_up { "UP" } else { "DOWN" }.to_string());
            let last_check = last_result.map(|(_, time, _)| time.to_rfc3339());
            let status_message = last_result.and_then(|(_, _, details)| details.clone());

            ServiceMonitorDetails {
                id: monitor.id,
                user_id: monitor.user_id,
                name: monitor.name,
                monitor_type: monitor.monitor_type,
                target: monitor.target,
                frequency_seconds: monitor.frequency_seconds,
                timeout_seconds: monitor.timeout_seconds,
                is_active: monitor.is_active,
                monitor_config: monitor.monitor_config.unwrap_or_default(),
                created_at: monitor.created_at.to_rfc3339(),
                updated_at: monitor.updated_at.to_rfc3339(),
                agent_ids: agent_map.get(&monitor_id).cloned().unwrap_or_default(),
                tag_ids: tag_map.get(&monitor_id).cloned().unwrap_or_default(),
                assignment_type: monitor.assignment_type,
                last_status,
                last_check,
                status_message,
            }
        })
        .collect();

    Ok(details_list)
}

pub async fn get_monitor_details_by_id(
    pool: DuckDbPool,
    monitor_id: i32,
) -> Result<Option<ServiceMonitorDetails>, AppError> {
    let conn = pool.get()?;

    // 1. Fetch the monitor
    let monitor: service_monitor::Model = match conn.query_row(
        "SELECT * FROM service_monitors WHERE id = ?",
        params![monitor_id],
        row_to_monitor_model,
    ) {
        Ok(monitor) => monitor,
        Err(duckdb::Error::QueryReturnedNoRows) => return Ok(None),
        Err(e) => return Err(e.into()),
    };

    // 2. Fetch agent assignments
    let agent_ids: Vec<i32> = conn
        .prepare("SELECT vps_id FROM service_monitor_agents WHERE monitor_id = ?")?
        .query_map(params![monitor_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    // 3. Fetch tag assignments
    let tag_ids: Vec<i32> = conn
        .prepare("SELECT tag_id FROM service_monitor_tags WHERE monitor_id = ?")?
        .query_map(params![monitor_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    // 4. Fetch the latest result
    let latest_result: Option<(bool, DateTime<Utc>, Option<String>)> = conn
        .query_row(
            "
            SELECT is_up, time, details->>'message' as details
            FROM service_monitor_results
            WHERE monitor_id = ?
            ORDER BY time DESC
            LIMIT 1
            ",
            params![monitor_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;

    // 5. Construct the final response model
    let last_status = latest_result.as_ref().map(|(is_up, _, _)| if *is_up { "UP" } else { "DOWN" }.to_string());
    let last_check = latest_result.as_ref().map(|(_, time, _)| time.to_rfc3339());
    let status_message = latest_result.and_then(|(_, _, details)| details);

    let details = ServiceMonitorDetails {
        id: monitor.id,
        user_id: monitor.user_id,
        name: monitor.name,
        monitor_type: monitor.monitor_type,
        target: monitor.target,
        frequency_seconds: monitor.frequency_seconds,
        timeout_seconds: monitor.timeout_seconds,
        is_active: monitor.is_active,
        monitor_config: monitor.monitor_config.unwrap_or_default(),
        created_at: monitor.created_at.to_rfc3339(),
        updated_at: monitor.updated_at.to_rfc3339(),
        agent_ids,
        tag_ids,
        assignment_type: monitor.assignment_type,
        last_status,
        last_check,
        status_message,
    };

    Ok(Some(details))
}

pub async fn update_monitor(
    pool: DuckDbPool,
    monitor_id: i32,
    user_id: i32,
    payload: UpdateMonitor,
) -> Result<(ServiceMonitorDetails, Vec<i32>), AppError> {
    let pool_clone = pool.clone();
    let blocking_task = tokio::task::spawn_blocking(move || {
        let mut conn = pool_clone.get()?;
        
        // Get the state of assignments *before* the transaction
        let old_agent_ids: Vec<i32> = conn
            .prepare("SELECT vps_id FROM service_monitor_agents WHERE monitor_id = ?")?
            .query_map(params![monitor_id], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        let tx = conn.transaction()?;

        // Fetch the monitor to ensure it exists and belongs to the user
        if let Err(duckdb::Error::QueryReturnedNoRows) = tx.query_row::<(), _, _>(
            "SELECT 1 FROM service_monitors WHERE id = ? AND user_id = ?",
            params![monitor_id, user_id],
            |_| Ok(()),
        ) {
            return Err(AppError::NotFound("Monitor not found or permission denied".to_string()));
        }

        // Dynamically build the UPDATE statement
        let mut set_clauses: Vec<String> = Vec::new();
        let mut params_vec: Vec<duckdb::types::Value> = Vec::new();

        macro_rules! add_param {
            ($field:expr, $name:expr) => {
                if let Some(value) = $field {
                    set_clauses.push(format!("{} = ?", $name));
                    params_vec.push(duckdb::types::Value::from(value.clone()));
                }
            };
        }

        add_param!(&payload.name, "name");
        add_param!(&payload.monitor_type, "monitor_type");
        add_param!(&payload.target, "target");
        add_param!(&payload.frequency_seconds, "frequency_seconds");
        add_param!(&payload.timeout_seconds, "timeout_seconds");
        add_param!(&payload.is_active, "is_active");

        let monitor_config_str = if let Some(config) = &payload.monitor_config {
            Some(serde_json::to_string(config)?)
        } else {
            None
        };
        if let Some(config_str) = &monitor_config_str {
            set_clauses.push("monitor_config = ?".to_string());
            params_vec.push(duckdb::types::Value::from(config_str.clone()));
        }

        if let Some(assignments) = &payload.assignments {
            if let Some(assignment_type) = &assignments.assignment_type {
                set_clauses.push("assignment_type = ?".to_string());
                params_vec.push(duckdb::types::Value::from(assignment_type.clone()));
            }
        }
        
        let now = Utc::now();
        set_clauses.push("updated_at = ?".to_string());
        params_vec.push(duckdb::types::Value::from(now.timestamp_micros()));

        if !set_clauses.is_empty() {
            let sql = format!(
                "UPDATE service_monitors SET {} WHERE id = ? AND user_id = ?",
                set_clauses.join(", ")
            );
            let mut final_params: Vec<&dyn duckdb::ToSql> = params_vec.iter().map(|p| p as &dyn duckdb::ToSql).collect();
            final_params.push(&monitor_id);
            final_params.push(&user_id);
            tx.execute(&sql, &final_params[..])?;
        }

        let mut new_agent_ids = Vec::new();
        if let Some(assignments) = payload.assignments {
            tx.execute("DELETE FROM service_monitor_agents WHERE monitor_id = ?", params![monitor_id])?;
            tx.execute("DELETE FROM service_monitor_tags WHERE monitor_id = ?", params![monitor_id])?;

            if let Some(agent_ids) = assignments.agent_ids {
                new_agent_ids = agent_ids;
                if !new_agent_ids.is_empty() {
                    let mut stmt = tx.prepare("INSERT INTO service_monitor_agents (monitor_id, vps_id) VALUES (?, ?)")?;
                    for vps_id in &new_agent_ids {
                        stmt.execute(params![monitor_id, vps_id])?;
                    }
                }
            }

            if let Some(tag_ids) = assignments.tag_ids {
                if !tag_ids.is_empty() {
                    let mut stmt = tx.prepare("INSERT INTO service_monitor_tags (monitor_id, tag_id) VALUES (?, ?)")?;
                    for tag_id in tag_ids {
                        stmt.execute(params![monitor_id, tag_id])?;
                    }
                }
            }
        }

        tx.commit()?;
        
        let mut affected_vps_ids = old_agent_ids;
        affected_vps_ids.extend(new_agent_ids.iter().cloned());
        affected_vps_ids.sort_unstable();
        affected_vps_ids.dedup();

        Ok(affected_vps_ids)
    });

    let affected_vps_ids = blocking_task.await.map_err(|e| AppError::InternalServerError(e.to_string()))??;

    let updated_details = get_monitor_details_by_id(pool.clone(), monitor_id)
        .await?
        .ok_or_else(|| AppError::NotFound("Monitor not found after update".to_string()))?;

    Ok((updated_details, affected_vps_ids))
}

pub async fn delete_monitor(
    pool: DuckDbPool,
    monitor_id: i32,
    user_id: i32,
) -> Result<u64, AppError> {
    let conn = pool.get()?;
    let rows_affected = conn.execute(
        "DELETE FROM service_monitors WHERE id = ? AND user_id = ?",
        params![monitor_id, user_id],
    )?;
    Ok(rows_affected as u64)
}

pub async fn get_monitors_for_vps(
    pool: DuckDbPool,
    vps_id: i32,
) -> Result<Vec<service_monitor::Model>, AppError> {
    let conn = pool.get()?;

    let direct_monitor_ids: Vec<i32> = conn
        .prepare("SELECT monitor_id FROM service_monitor_agents WHERE vps_id = ?")?
        .query_map(params![vps_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    let agent_tags: Vec<i32> = conn
        .prepare("SELECT tag_id FROM vps_tags WHERE vps_id = ?")?
        .query_map(params![vps_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut tagged_monitor_ids: Vec<i32> = Vec::new();
    if !agent_tags.is_empty() {
        let placeholders = repeat_vars(agent_tags.len());
        let sql = format!("SELECT monitor_id FROM service_monitor_tags WHERE tag_id IN {placeholders}");
        tagged_monitor_ids = conn
            .prepare(&sql)?
            .query_map(params_from_iter(agent_tags.iter()), |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
    }

    let mut all_monitor_ids = direct_monitor_ids;
    all_monitor_ids.extend(tagged_monitor_ids);
    all_monitor_ids.sort_unstable();
    all_monitor_ids.dedup();

    if all_monitor_ids.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = repeat_vars(all_monitor_ids.len());
    let sql = format!("SELECT * FROM service_monitors WHERE id IN {placeholders}");
    let monitors = conn
        .prepare(&sql)?
        .query_map(params_from_iter(all_monitor_ids.iter()), row_to_monitor_model)?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(monitors)
}

pub async fn get_runnable_monitors_for_vps(
    pool: DuckDbPool,
    vps_id: i32,
) -> Result<Vec<service_monitor::Model>, AppError> {
    let conn = pool.get()?;

    let user_id: i32 = conn.query_row(
        "SELECT user_id FROM vps WHERE id = ?",
        params![vps_id],
        |row| row.get(0),
    )?;

    let all_active_monitors: Vec<service_monitor::Model> = conn
        .prepare("SELECT * FROM service_monitors WHERE user_id = ? AND is_active = TRUE")?
        .query_map(params![user_id], row_to_monitor_model)?
        .collect::<Result<Vec<_>, _>>()?;

    if all_active_monitors.is_empty() {
        return Ok(Vec::new());
    }

    let monitor_ids: Vec<i32> = all_active_monitors.iter().map(|m| m.id).collect();
    let placeholders = repeat_vars(monitor_ids.len());

    let agent_assignments: Vec<(i32, i32)> = conn
        .prepare(&format!("SELECT monitor_id, vps_id FROM service_monitor_agents WHERE monitor_id IN {placeholders}"))?
        .query_map(params_from_iter(monitor_ids.iter()), |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    let tag_assignments: Vec<(i32, i32)> = conn
        .prepare(&format!("SELECT monitor_id, tag_id FROM service_monitor_tags WHERE monitor_id IN {placeholders}"))?
        .query_map(params_from_iter(monitor_ids.iter()), |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;

    let vps_tags: Vec<i32> = conn
        .prepare("SELECT tag_id FROM vps_tags WHERE vps_id = ?")?
        .query_map(params![vps_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut monitor_agent_assignments: HashMap<i32, HashSet<i32>> = HashMap::new();
    for (monitor_id, agent_id) in agent_assignments {
        monitor_agent_assignments.entry(monitor_id).or_default().insert(agent_id);
    }

    let mut monitor_tag_assignments: HashMap<i32, HashSet<i32>> = HashMap::new();
    for (monitor_id, tag_id) in tag_assignments {
        monitor_tag_assignments.entry(monitor_id).or_default().insert(tag_id);
    }

    let vps_tag_ids: HashSet<i32> = vps_tags.into_iter().collect();

    let runnable_monitors = all_active_monitors
        .into_iter()
        .filter(|monitor| {
            let empty_set = HashSet::new();
            let assigned_agents = monitor_agent_assignments.get(&monitor.id).unwrap_or(&empty_set);
            let assigned_tags = monitor_tag_assignments.get(&monitor.id).unwrap_or(&empty_set);

            let is_directly_assigned = assigned_agents.contains(&vps_id);
            let has_assigned_tag = !vps_tag_ids.is_disjoint(assigned_tags);

            if monitor.assignment_type == "EXCLUSIVE" {
                !is_directly_assigned && !has_assigned_tag
            } else {
                is_directly_assigned || has_assigned_tag
            }
        })
        .collect();

    Ok(runnable_monitors)
}

pub async fn get_tasks_for_agent(
    pool: DuckDbPool,
    vps_id: i32,
) -> Result<Vec<ServiceMonitorTask>, AppError> {
    let monitors = get_runnable_monitors_for_vps(pool, vps_id).await?;
    let tasks = monitors
        .into_iter()
        .map(|monitor| ServiceMonitorTask {
            monitor_id: monitor.id,
            name: monitor.name,
            monitor_type: monitor.monitor_type,
            target: monitor.target,
            frequency_seconds: monitor.frequency_seconds,
            monitor_config_json: monitor
                .monitor_config
                .as_ref()
                .map_or_else(|| "{}".to_string(), |json| json.to_string()),
            timeout_seconds: monitor.timeout_seconds,
        })
        .collect();
    Ok(tasks)
}

pub async fn get_vps_ids_for_monitor(
    pool: DuckDbPool,
    monitor_id: i32,
) -> Result<Vec<i32>, AppError> {
    let conn = pool.get()?;
    let monitor: service_monitor::Model = conn.query_row(
        "SELECT * FROM service_monitors WHERE id = ?",
        params![monitor_id],
        row_to_monitor_model,
    )?;

    let assigned_agent_ids: Vec<i32> = conn
        .prepare("SELECT vps_id FROM service_monitor_agents WHERE monitor_id = ?")?
        .query_map(params![monitor_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    let assigned_tag_ids: Vec<i32> = conn
        .prepare("SELECT tag_id FROM service_monitor_tags WHERE monitor_id = ?")?
        .query_map(params![monitor_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;

    let agents_from_tags = if !assigned_tag_ids.is_empty() {
        let placeholders = repeat_vars(assigned_tag_ids.len());
        let sql = format!("SELECT vps_id FROM vps_tags WHERE tag_id IN {placeholders}");
        conn.prepare(&sql)?
            .query_map(params_from_iter(assigned_tag_ids.iter()), |row| row.get::<_, i32>(0))?
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };

    let mut combined_assigned_ids: Vec<i32> = assigned_agent_ids;
    combined_assigned_ids.extend(agents_from_tags);
    combined_assigned_ids.sort_unstable();
    combined_assigned_ids.dedup();

    if monitor.assignment_type == "EXCLUSIVE" {
        let all_agent_ids: Vec<i32> = conn
            .prepare("SELECT id FROM vps")?
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;

        let excluded_ids_set: HashSet<i32> = combined_assigned_ids.into_iter().collect();

        let final_agent_ids = all_agent_ids
            .into_iter()
            .filter(|id| !excluded_ids_set.contains(id))
            .collect();
        Ok(final_agent_ids)
    } else {
        Ok(combined_assigned_ids)
    }
}

pub async fn record_monitor_result(
    pool: DuckDbPool,
    agent_id: i32, // This is the vps_id
    result: &ServiceMonitorResult,
) -> Result<(), AppError> {
    let conn = pool.get()?;
    let details_str = serde_json::to_string(&serde_json::json!({ "message": &result.details }))?;
    conn.execute(
        "INSERT INTO service_monitor_results (time, monitor_id, agent_id, is_up, latency_ms, details)
         VALUES (?, ?, ?, ?, ?, ?)",
        params![
            result.timestamp_unix_ms,
            result.monitor_id,
            agent_id,
            result.successful,
            result.response_time_ms,
            details_str,
        ],
    )?;
    Ok(())
}

fn row_to_service_monitor_point(row: &Row) -> DuckDbResult<ServiceMonitorPoint> {
    Ok(ServiceMonitorPoint {
        time: row.get("time")?,
        monitor_id: row.get("monitor_id")?,
        agent_id: row.get("agent_id")?,
        is_up: row.get("is_up")?,
        latency_ms: row.get("latency_ms")?,
        details: json_from_row(row, "details")?,
    })
}

pub async fn get_monitor_results_by_id(
    pool: DuckDbPool,
    monitor_id: i32,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    interval_seconds: Option<i64>,
) -> Result<Vec<ServiceMonitorPoint>, AppError> {
    let conn = pool.get()?;
    
    if let Some(interval) = interval_seconds {
        let sql = format!(
            "SELECT
                time_bucket(INTERVAL '{interval}' SECONDS, time) as time,
                monitor_id,
                agent_id,
                AVG(latency_ms)::DOUBLE as latency_ms,
                CAST(SUM(CASE WHEN is_up THEN 1 ELSE 0 END) AS REAL) / COUNT(*) as is_up,
                NULL as details
             FROM service_monitor_results
             WHERE monitor_id = ? AND time >= ? AND time <= ?
             GROUP BY 1, 2, 3
             ORDER BY 1 DESC"
        );
        let points = conn
            .prepare(&sql)?
            .query_map(params![monitor_id, start_time, end_time], row_to_service_monitor_point)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(points)
    } else {
        let points = conn
            .prepare(
                "SELECT time, monitor_id, agent_id, latency_ms, is_up, details
                 FROM service_monitor_results
                 WHERE monitor_id = ? AND time >= ? AND time <= ?
                 ORDER BY time DESC",
            )?
            .query_map(params![monitor_id, start_time, end_time], row_to_service_monitor_point)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(points)
    }
}

pub async fn get_monitor_results_by_vps_id(
    pool: DuckDbPool,
    vps_id: i32,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    interval_seconds: Option<i64>,
) -> Result<Vec<ServiceMonitorPoint>, AppError> {
    let conn = pool.get()?;
    let runnable_monitors = get_runnable_monitors_for_vps(pool.clone(), vps_id).await?;
    if runnable_monitors.is_empty() {
        return Ok(Vec::new());
    }
    let monitor_ids: Vec<i32> = runnable_monitors.into_iter().map(|m| m.id).collect();
    let placeholders = repeat_vars(monitor_ids.len());

    if let Some(interval) = interval_seconds {
        let sql = format!(
            "SELECT
                time_bucket(INTERVAL '{interval}' SECONDS, time) as time,
                monitor_id,
                agent_id,
                AVG(latency_ms)::DOUBLE as latency_ms,
                CAST(SUM(CASE WHEN is_up THEN 1 ELSE 0 END) AS REAL) / COUNT(*) as is_up,
                NULL as details
             FROM service_monitor_results
             WHERE monitor_id IN {placeholders} AND agent_id = ? AND time >= ? AND time <= ?
             GROUP BY 1, 2, 3
             ORDER BY 1 DESC"
        );
        let mut params: Vec<&dyn duckdb::ToSql> = monitor_ids.iter().map(|id| id as &dyn duckdb::ToSql).collect();
        params.push(&vps_id);
        params.push(&start_time);
        params.push(&end_time);

        let points = conn
            .prepare(&sql)?
            .query_map(params.as_slice(), row_to_service_monitor_point)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(points)
    } else {
        let sql = format!(
            "SELECT time, monitor_id, agent_id, latency_ms, is_up, details
             FROM service_monitor_results
             WHERE monitor_id IN {placeholders} AND agent_id = ? AND time >= ? AND time <= ?
             ORDER BY time DESC"
        );
        let mut params: Vec<&dyn duckdb::ToSql> = monitor_ids.iter().map(|id| id as &dyn duckdb::ToSql).collect();
        params.push(&vps_id);
        params.push(&start_time);
        params.push(&end_time);

        let points = conn
            .prepare(&sql)?
            .query_map(params.as_slice(), row_to_service_monitor_point)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(points)
    }
}