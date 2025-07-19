//! Service for managing service monitors and their results.
//!
//! This service provides functions for CRUD operations on service monitors,
//! assigning them to agents/tags, and recording check results.

use crate::db::entities::{
    prelude::*, service_monitor, service_monitor_agent, service_monitor_result,
    service_monitor_tag, vps,
};
use crate::web::models::service_monitor_models::{
    CreateMonitor, ServiceMonitorDetails, UpdateMonitor,
};
use futures::try_join;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    FromQueryResult, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use tracing::error;
use std::collections::{HashMap, HashSet};

use crate::agent_service::{ServiceMonitorResult, ServiceMonitorTask};
use crate::db::entities::vps_tag;
use chrono::{DateTime, Utc};
use chrono::TimeZone;
use sea_orm::sea_query::{Alias, Expr, Func};
use serde::{Deserialize, Serialize};

#[derive(FromQueryResult, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceMonitorPoint {
    pub time: DateTime<Utc>,
    pub monitor_id: i32,
    pub agent_id: i32,
    // is_up is a float because it can represent an average (availability)
    pub is_up: Option<f64>,
    // latency_ms is a float because it can be an average
    pub latency_ms: Option<f64>,
    // The details field is not aggregated, so it's an Option
    pub details: Option<serde_json::Value>,
}

pub async fn create_monitor(
    db: &DatabaseConnection,
    user_id: i32,
    monitor_data: CreateMonitor,
) -> Result<service_monitor::Model, DbErr> {
    let new_monitor = service_monitor::ActiveModel {
        user_id: Set(user_id),
        name: Set(monitor_data.name),
        monitor_type: Set(monitor_data.monitor_type),
        target: Set(monitor_data.target),
        frequency_seconds: Set(monitor_data.frequency_seconds.unwrap_or(60)),
        timeout_seconds: Set(monitor_data.timeout_seconds.unwrap_or(10)),
        is_active: Set(monitor_data.is_active.unwrap_or(true)),
        monitor_config: Set(monitor_data.monitor_config),
        assignment_type: Set(monitor_data
            .assignments
            .assignment_type
            .unwrap_or_else(|| "INCLUSIVE".to_string())),
        ..Default::default()
    };

    let txn = db.begin().await?;

    let saved_monitor = new_monitor.insert(&txn).await?;

    // Handle agent assignments
    if let Some(agent_ids) = monitor_data.assignments.agent_ids {
        if !agent_ids.is_empty() {
            let agent_assignments =
                agent_ids
                    .into_iter()
                    .map(|vps_id| service_monitor_agent::ActiveModel {
                        monitor_id: Set(saved_monitor.id),
                        vps_id: Set(vps_id),
                    });
            ServiceMonitorAgent::insert_many(agent_assignments)
                .exec(&txn)
                .await?;
        }
    }

    // Handle tag assignments
    if let Some(tag_ids) = monitor_data.assignments.tag_ids {
        if !tag_ids.is_empty() {
            let tag_assignments =
                tag_ids
                    .into_iter()
                    .map(|tag_id| service_monitor_tag::ActiveModel {
                        monitor_id: Set(saved_monitor.id),
                        tag_id: Set(tag_id),
                    });
            ServiceMonitorTag::insert_many(tag_assignments)
                .exec(&txn)
                .await?;
        }
    }

    txn.commit().await?;

    Ok(saved_monitor)
}

pub async fn get_monitors_with_details_by_user_id(
    db: &DatabaseConnection,
    user_id: i32,
) -> Result<Vec<ServiceMonitorDetails>, DbErr> {
    // 1. Fetch all monitors for the user
    let monitors = ServiceMonitor::find()
        .filter(service_monitor::Column::UserId.eq(user_id))
        .all(db)
        .await?;

    if monitors.is_empty() {
        return Ok(Vec::new());
    }

    let monitor_ids: Vec<i32> = monitors.iter().map(|m| m.id).collect();

    // 2. Fetch all agent and tag assignments for these monitors in parallel
    let agents_future = ServiceMonitorAgent::find()
        .filter(service_monitor_agent::Column::MonitorId.is_in(monitor_ids.clone()))
        .all(db);

    let tags_future = ServiceMonitorTag::find()
        .filter(service_monitor_tag::Column::MonitorId.is_in(monitor_ids.clone()))
        .all(db);

    let (agent_assignments, tag_assignments) = try_join!(agents_future, tags_future)?;

    // 3. Group assignments by monitor_id for efficient lookup
    let mut agent_map: HashMap<i32, Vec<i32>> = HashMap::new();
    for agent in agent_assignments {
        agent_map
            .entry(agent.monitor_id)
            .or_default()
            .push(agent.vps_id);
    }

    let mut tag_map: HashMap<i32, Vec<i32>> = HashMap::new();
    for tag in tag_assignments {
        tag_map.entry(tag.monitor_id).or_default().push(tag.tag_id);
    }

    // 4. Fetch the latest result for each monitor using a window function for efficiency
    let latest_results: Vec<service_monitor_result::Model> = service_monitor_result::Entity::find()
        .from_raw_sql(sea_orm::Statement::from_sql_and_values(
            db.get_database_backend(),
            r#"
            SELECT DISTINCT ON (monitor_id) *
            FROM service_monitor_results
            WHERE monitor_id = ANY($1)
            ORDER BY monitor_id, "time" DESC
            "#,
            [monitor_ids.clone().into()],
        ))
        .all(db)
        .await?;

    let latest_result_map: HashMap<i32, service_monitor_result::Model> = latest_results
        .into_iter()
        .map(|result| (result.monitor_id, result))
        .collect();

    // 5. Construct the final response models
    let details_list = monitors
        .into_iter()
        .map(|monitor| {
            let monitor_id = monitor.id;
            let last_result = latest_result_map.get(&monitor_id);

            let last_status = last_result.map(|r| if r.is_up { "UP" } else { "DOWN" }.to_string());
            let last_check = last_result.map(|r| r.time.to_rfc3339());
            let status_message = last_result.and_then(|r| {
                r.details.as_ref().and_then(|d| {
                    d.get("message")
                        .and_then(|m| m.as_str())
                        .map(String::from)
                })
            });

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
    db: &DatabaseConnection,
    monitor_id: i32,
) -> Result<Option<ServiceMonitorDetails>, DbErr> {
    // 1. Fetch the monitor by its ID
    let monitor = match ServiceMonitor::find_by_id(monitor_id).one(db).await? {
        Some(m) => m,
        None => return Ok(None),
    };

    // 2. Fetch assignments in parallel
    let agents_future = ServiceMonitorAgent::find()
        .filter(service_monitor_agent::Column::MonitorId.eq(monitor_id))
        .all(db);

    let tags_future = ServiceMonitorTag::find()
        .filter(service_monitor_tag::Column::MonitorId.eq(monitor_id))
        .all(db);

    let (agent_assignments, tag_assignments) = try_join!(agents_future, tags_future)?;

    // 3. Collect IDs
    let agent_ids = agent_assignments.into_iter().map(|a| a.vps_id).collect();
    let tag_ids = tag_assignments.into_iter().map(|t| t.tag_id).collect();
    // 4. Construct the response model
    let latest_result = service_monitor_result::Entity::find()
        .filter(service_monitor_result::Column::MonitorId.eq(monitor_id))
        .order_by_desc(service_monitor_result::Column::Time)
        .one(db)
        .await?;

    let last_status = latest_result
        .as_ref()
        .map(|r| if r.is_up { "UP" } else { "DOWN" }.to_string());
    let last_check = latest_result.as_ref().map(|r| r.time.to_rfc3339());
    let status_message = latest_result.and_then(|r| {
        r.details.as_ref().and_then(|d| {
            d.get("message")
                .and_then(|m| m.as_str())
                .map(String::from)
        })
    });

    // 4. Construct the response model
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
    db: &DatabaseConnection,
    monitor_id: i32,
    user_id: i32,
    payload: UpdateMonitor,
) -> Result<(ServiceMonitorDetails, Vec<i32>), DbErr> {
    // Get the state of assignments *before* the transaction
    let old_vps_ids = get_vps_ids_for_monitor(db, monitor_id).await?;

    let txn = db.begin().await?;

    // 1. Fetch the monitor and authorize the user
    let monitor = ServiceMonitor::find_by_id(monitor_id)
        .filter(service_monitor::Column::UserId.eq(user_id))
        .one(&txn)
        .await?
        .ok_or_else(|| {
            DbErr::RecordNotFound("Monitor not found or permission denied".to_string())
        })?;

    let mut active_monitor: service_monitor::ActiveModel = monitor.into();

    // 2. Update main fields if present
    if let Some(name) = payload.name {
        active_monitor.name = Set(name);
    }
    if let Some(monitor_type) = payload.monitor_type {
        active_monitor.monitor_type = Set(monitor_type);
    }
    if let Some(target) = payload.target {
        active_monitor.target = Set(target);
    }
    if let Some(frequency) = payload.frequency_seconds {
        active_monitor.frequency_seconds = Set(frequency);
    }
    if let Some(timeout) = payload.timeout_seconds {
        active_monitor.timeout_seconds = Set(timeout);
    }
    if let Some(is_active) = payload.is_active {
        active_monitor.is_active = Set(is_active);
    }
    if let Some(config) = payload.monitor_config {
        active_monitor.monitor_config = Set(Some(config));
    }

    // 3. Update the monitor record
    active_monitor.update(&txn).await?;

    // 4. Handle assignments if present
    if let Some(assignments) = payload.assignments {
        if let Some(assignment_type) = assignments.assignment_type {
            let monitor_for_update = ServiceMonitor::find_by_id(monitor_id)
                .one(&txn)
                .await?
                .unwrap();
            let mut active_monitor_for_update: service_monitor::ActiveModel =
                monitor_for_update.into();
            active_monitor_for_update.assignment_type = Set(assignment_type);
            active_monitor_for_update.update(&txn).await?;
        }
        // Clear existing assignments
        ServiceMonitorAgent::delete_many()
            .filter(service_monitor_agent::Column::MonitorId.eq(monitor_id))
            .exec(&txn)
            .await?;
        ServiceMonitorTag::delete_many()
            .filter(service_monitor_tag::Column::MonitorId.eq(monitor_id))
            .exec(&txn)
            .await?;

        // Add new agent assignments
        if let Some(agent_ids) = assignments.agent_ids {
            if !agent_ids.is_empty() {
                let agent_assignments =
                    agent_ids
                        .into_iter()
                        .map(|vps_id| service_monitor_agent::ActiveModel {
                            monitor_id: Set(monitor_id),
                            vps_id: Set(vps_id),
                        });
                ServiceMonitorAgent::insert_many(agent_assignments)
                    .exec(&txn)
                    .await?;
            }
        }

        // Add new tag assignments
        if let Some(tag_ids) = assignments.tag_ids {
            if !tag_ids.is_empty() {
                let tag_assignments =
                    tag_ids
                        .into_iter()
                        .map(|tag_id| service_monitor_tag::ActiveModel {
                            monitor_id: Set(monitor_id),
                            tag_id: Set(tag_id),
                        });
                ServiceMonitorTag::insert_many(tag_assignments)
                    .exec(&txn)
                    .await?;
            }
        }
    }

    txn.commit().await?;

    // 5. Determine the full set of affected agents
    let new_vps_ids = get_vps_ids_for_monitor(db, monitor_id).await?;
    let mut affected_vps_ids = old_vps_ids;
    affected_vps_ids.extend(new_vps_ids);
    affected_vps_ids.sort_unstable();
    affected_vps_ids.dedup();

    // 6. Fetch and return the updated details
    // We call the existing get function to ensure consistency
    let details = get_monitor_details_by_id(db, monitor_id)
        .await?
        .ok_or_else(|| {
            DbErr::RecordNotFound("Failed to fetch updated monitor details.".to_string())
        })?;

    Ok((details, affected_vps_ids))
}

pub async fn delete_monitor(
    db: &DatabaseConnection,
    monitor_id: i32,
    user_id: i32,
) -> Result<sea_orm::DeleteResult, DbErr> {
    ServiceMonitor::delete_many()
        .filter(service_monitor::Column::Id.eq(monitor_id))
        .filter(service_monitor::Column::UserId.eq(user_id))
        .exec(db)
        .await
}
/// Fetches all service monitors assigned to a specific agent (VPS).
/// This is similar to get_tasks_for_agent but returns the full monitor models.
pub async fn get_monitors_for_vps(
    db: &DatabaseConnection,
    vps_id: i32,
) -> Result<Vec<service_monitor::Model>, DbErr> {
    // 1. Get monitor IDs from direct agent assignments
    let direct_monitor_ids_future = ServiceMonitorAgent::find()
        .select_only()
        .column(service_monitor_agent::Column::MonitorId)
        .filter(service_monitor_agent::Column::VpsId.eq(vps_id))
        .into_tuple::<i32>()
        .all(db);

    // 2. Get monitor IDs from tag-based assignments
    let agent_tags_future = VpsTag::find()
        .select_only()
        .column(vps_tag::Column::TagId)
        .filter(vps_tag::Column::VpsId.eq(vps_id))
        .into_tuple::<i32>()
        .all(db);

    let (direct_monitor_ids, agent_tags) = try_join!(direct_monitor_ids_future, agent_tags_future)?;

    let mut tagged_monitor_ids: Vec<i32> = Vec::new();
    if !agent_tags.is_empty() {
        tagged_monitor_ids = ServiceMonitorTag::find()
            .select_only()
            .column(service_monitor_tag::Column::MonitorId)
            .filter(service_monitor_tag::Column::TagId.is_in(agent_tags))
            .into_tuple::<i32>()
            .all(db)
            .await?;
    }

    // 3. Combine and deduplicate monitor IDs
    let mut all_monitor_ids = direct_monitor_ids;
    all_monitor_ids.extend(tagged_monitor_ids);
    all_monitor_ids.sort_unstable();
    all_monitor_ids.dedup();

    if all_monitor_ids.is_empty() {
        return Ok(Vec::new());
    }

    // 4. Fetch all monitors corresponding to the collected IDs
    let monitors = ServiceMonitor::find()
        .filter(service_monitor::Column::Id.is_in(all_monitor_ids))
        .all(db)
        .await?;

    Ok(monitors)
}
/// Fetches all active service monitoring tasks assigned to a specific agent (VPS).
///
/// This function determines the full set of monitors for an agent by considering
/// the `assignment_type` of each monitor:
/// - `INCLUSIVE`: The agent runs the monitor if it's directly assigned or has a matching tag.
/// - `EXCLUSIVE`: The agent runs the monitor if it's NOT directly assigned and does NOT have a matching tag.
///
/// It then fetches the details for these monitors, ensuring only active ones are returned,
/// and transforms them into the gRPC `ServiceMonitorTask` format.
pub async fn get_tasks_for_agent(
    db: &DatabaseConnection,
    vps_id: i32,
) -> Result<Vec<ServiceMonitorTask>, DbErr> {
    let monitors = get_runnable_monitors_for_vps(db, vps_id).await?;
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

/// A helper function that determines which monitors a VPS should be running,
/// considering direct and tag-based assignments with INCLUSIVE/EXCLUSIVE logic.
/// This returns the full monitor models.
pub async fn get_runnable_monitors_for_vps(
    db: &DatabaseConnection,
    vps_id: i32,
) -> Result<Vec<service_monitor::Model>, DbErr> {
    // 1. Get user_id for the vps to scope the monitors
    let vps = Vps::find_by_id(vps_id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound(format!("VPS with ID {vps_id} not found")))?;
    let user_id = vps.user_id;

    // 2. Get all active monitors for the user
    let all_active_monitors = ServiceMonitor::find()
        .filter(service_monitor::Column::UserId.eq(user_id))
        .filter(service_monitor::Column::IsActive.eq(true))
        .all(db)
        .await?;

    if all_active_monitors.is_empty() {
        return Ok(Vec::new());
    }

    let monitor_ids: Vec<i32> = all_active_monitors.iter().map(|m| m.id).collect();

    // 3. Fetch all assignments for these monitors and tags for the current VPS in parallel
    let agent_assignments_future = ServiceMonitorAgent::find()
        .filter(service_monitor_agent::Column::MonitorId.is_in(monitor_ids.clone()))
        .all(db);

    let tag_assignments_future = ServiceMonitorTag::find()
        .filter(service_monitor_tag::Column::MonitorId.is_in(monitor_ids))
        .all(db);

    let vps_tags_future = VpsTag::find()
        .filter(vps_tag::Column::VpsId.eq(vps_id))
        .all(db);

    let (agent_assignments, tag_assignments, vps_tags) =
        try_join!(agent_assignments_future, tag_assignments_future, vps_tags_future)?;

    // 4. Process assignments and tags into efficient lookup structures
    let mut monitor_agent_assignments: HashMap<i32, HashSet<i32>> = HashMap::new();
    for assignment in agent_assignments {
        monitor_agent_assignments
            .entry(assignment.monitor_id)
            .or_default()
            .insert(assignment.vps_id);
    }

    let mut monitor_tag_assignments: HashMap<i32, HashSet<i32>> = HashMap::new();
    for assignment in tag_assignments {
        monitor_tag_assignments
            .entry(assignment.monitor_id)
            .or_default()
            .insert(assignment.tag_id);
    }

    let vps_tag_ids: HashSet<i32> = vps_tags.into_iter().map(|t| t.tag_id).collect();

    // 5. Filter monitors based on assignment logic
    let runnable_monitors = all_active_monitors
        .into_iter()
        .filter(|monitor| {
            let empty_set = HashSet::new();
            let assigned_agents = monitor_agent_assignments
                .get(&monitor.id)
                .unwrap_or(&empty_set);
            let assigned_tags = monitor_tag_assignments
                .get(&monitor.id)
                .unwrap_or(&empty_set);

            let is_directly_assigned = assigned_agents.contains(&vps_id);
            let has_assigned_tag = !vps_tag_ids.is_disjoint(assigned_tags);

            if monitor.assignment_type == "EXCLUSIVE" {
                // For EXCLUSIVE, run if NOT assigned directly and NOT assigned via tag
                !is_directly_assigned && !has_assigned_tag
            } else {
                // For INCLUSIVE, run if assigned directly OR via tag
                is_directly_assigned || has_assigned_tag
            }
        })
        .collect();

    Ok(runnable_monitors)
}

/// Given a monitor ID, finds all VPS IDs that should be running this monitor.
///
/// This is determined by looking at both direct agent assignments and tag-based assignments.
/// It's a helper function to determine which agents are affected by a monitor change.
pub async fn get_vps_ids_for_monitor(
    db: &DatabaseConnection,
    monitor_id: i32,
) -> Result<Vec<i32>, DbErr> {
    let monitor = ServiceMonitor::find_by_id(monitor_id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound(format!("Monitor with ID {monitor_id} not found")))?;

    // Get explicitly assigned/excluded agents and tags
    let assigned_agents_future = ServiceMonitorAgent::find()
        .filter(service_monitor_agent::Column::MonitorId.eq(monitor_id))
        .all(db);
    let assigned_tags_future = ServiceMonitorTag::find()
        .filter(service_monitor_tag::Column::MonitorId.eq(monitor_id))
        .all(db);

    let (assigned_agents, assigned_tags) = try_join!(assigned_agents_future, assigned_tags_future)?;

    let assigned_agent_ids: Vec<i32> = assigned_agents.into_iter().map(|a| a.vps_id).collect();
    let assigned_tag_ids: Vec<i32> = assigned_tags.into_iter().map(|t| t.tag_id).collect();

    let agents_from_tags = if !assigned_tag_ids.is_empty() {
        VpsTag::find()
            .filter(vps_tag::Column::TagId.is_in(assigned_tag_ids))
            .all(db)
            .await?
            .into_iter()
            .map(|vt| vt.vps_id)
            .collect::<Vec<i32>>()
    } else {
        Vec::new()
    };

    let mut combined_assigned_ids = assigned_agent_ids;
    combined_assigned_ids.extend(agents_from_tags);
    combined_assigned_ids.sort_unstable();
    combined_assigned_ids.dedup();

    if monitor.assignment_type == "EXCLUSIVE" {
        let all_agent_ids = Vps::find()
            .select_only()
            .column(vps::Column::Id)
            .into_tuple::<i32>()
            .all(db)
            .await?;

        let excluded_ids_set: std::collections::HashSet<i32> =
            combined_assigned_ids.into_iter().collect();

        let final_agent_ids = all_agent_ids
            .into_iter()
            .filter(|id| !excluded_ids_set.contains(id))
            .collect();

        Ok(final_agent_ids)
    } else {
        // INCLUSIVE mode (default)
        Ok(combined_assigned_ids)
    }
}

pub async fn record_monitor_result(
    db: &DatabaseConnection,
    agent_id: i32, // This is the vps_id
    result: &ServiceMonitorResult,
) -> Result<(), DbErr> {
    let new_result = service_monitor_result::ActiveModel {
        time: Set(chrono::Utc.timestamp_millis_opt(result.timestamp_unix_ms).unwrap()),
        monitor_id: Set(result.monitor_id),
        agent_id: Set(agent_id),
        is_up: Set(result.successful),
        latency_ms: Set(result.response_time_ms),
        details: Set(Some(serde_json::json!({ "message": &result.details }))),
    };

    new_result.insert(db).await?;

    Ok(())
}

pub async fn get_monitor_results_by_id(
    db: &DatabaseConnection,
    monitor_id: i32,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    interval_seconds: Option<i64>,
) -> Result<Vec<ServiceMonitorPoint>, DbErr> {
    let query = service_monitor_result::Entity::find()
        .filter(service_monitor_result::Column::MonitorId.eq(monitor_id))
        .filter(service_monitor_result::Column::Time.gte(start_time))
        .filter(service_monitor_result::Column::Time.lte(end_time));

    if let Some(interval) = interval_seconds {
        // Aggregated query
        let interval_string = format!("{interval} seconds");
        query
            .select_only()
            .column_as(
                Expr::cust(format!("time_bucket('{interval} seconds', time)")),
                "time",
            )
            .column(service_monitor_result::Column::MonitorId)
            .column(service_monitor_result::Column::AgentId)
            .column_as(
                Expr::expr(Func::avg(Expr::col(
                    service_monitor_result::Column::LatencyMs,
                ))).cast_as(Alias::new("double precision")),
                "latency_ms",
            )
            .column_as(
                Expr::cust(
                    "CAST(SUM(CASE WHEN is_up THEN 1 ELSE 0 END) AS REAL) / COUNT(*)",
                ),
                "is_up",
            )
            // Details are not aggregated, so we don't select them here.
            .column_as(Expr::val(serde_json::Value::Null), "details")
            .group_by(Expr::cust("1")) // group by time_bucket
            .group_by(service_monitor_result::Column::MonitorId)
            .group_by(service_monitor_result::Column::AgentId)
            .order_by(Expr::cust("1"), sea_orm::Order::Desc)
            .into_model::<ServiceMonitorPoint>()
            .all(db)
            .await
    } else {
        // Raw data query
        query
            .select_only()
            .column_as(service_monitor_result::Column::Time, "time")
            .column(service_monitor_result::Column::MonitorId)
            .column(service_monitor_result::Column::AgentId)
            .column_as(
                Expr::col(service_monitor_result::Column::LatencyMs).cast_as(sea_orm::sea_query::Alias::new("float")),
                "latency_ms",
            )
            .column_as(
                Expr::cust("CAST(CASE WHEN is_up THEN 1.0 ELSE 0.0 END AS DOUBLE PRECISION)"),
                "is_up",
            )
            .column(service_monitor_result::Column::Details)
            .order_by_desc(service_monitor_result::Column::Time)
            .into_model::<ServiceMonitorPoint>()
            .all(db)
            .await
    }
}

pub async fn get_monitor_results_by_vps_id(
    db: &DatabaseConnection,
    vps_id: i32,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    interval_seconds: Option<i64>,
) -> Result<Vec<ServiceMonitorPoint>, DbErr> {
    // 1. Get all runnable monitor IDs for the given VPS
    let runnable_monitors = get_runnable_monitors_for_vps(db, vps_id).await?;
    if runnable_monitors.is_empty() {
        return Ok(Vec::new());
    }
    let monitor_ids: Vec<i32> = runnable_monitors.into_iter().map(|m| m.id).collect();

    // 2. Build the main query
    let query = service_monitor_result::Entity::find()
        .filter(service_monitor_result::Column::MonitorId.is_in(monitor_ids))
        .filter(service_monitor_result::Column::AgentId.eq(vps_id)) // Results must be from this agent
        .filter(service_monitor_result::Column::Time.gte(start_time))
        .filter(service_monitor_result::Column::Time.lte(end_time));

    // 3. Apply aggregation or select raw data
    if let Some(interval) = interval_seconds {
        // Aggregated query
        let interval_string = format!("{interval} seconds");
        let results = query
            .select_only()
            .column_as(
                Expr::cust(format!("time_bucket('{interval} seconds', time)")),
                "time",
            )
            .column(service_monitor_result::Column::MonitorId)
            .column(service_monitor_result::Column::AgentId)
            .column_as(
                Expr::expr(Func::avg(Expr::col(
                    service_monitor_result::Column::LatencyMs,
                ))).cast_as(Alias::new("double precision")),
                "latency_ms",
            )
            .column_as(
                Expr::cust(
                    "CAST(SUM(CASE WHEN is_up THEN 1 ELSE 0 END) AS REAL) / COUNT(*)",
                ),
                "is_up",
            )
            .column_as(Expr::val(serde_json::Value::Null), "details")
            .group_by(Expr::cust("1")) // group by time_bucket
            .group_by(service_monitor_result::Column::MonitorId)
            .group_by(service_monitor_result::Column::AgentId)
            .order_by(Expr::cust("1"), sea_orm::Order::Desc)
            .into_model::<ServiceMonitorPoint>()
            .all(db)
            .await;

        if let Err(e) = &results {
            error!("Database error in get_monitor_results_by_vps_id (raw): {:?}", e);
        }
        results
    } else {
        // Raw data query
        query
            .select_only()
            .column_as(service_monitor_result::Column::Time, "time")
            .column(service_monitor_result::Column::MonitorId)
            .column(service_monitor_result::Column::AgentId)
            .column_as(
                Expr::col(service_monitor_result::Column::LatencyMs).cast_as(sea_orm::sea_query::Alias::new("float")),
                "latency_ms",
            )
            .column_as(
                Expr::cust("CAST(CASE WHEN is_up THEN 1.0 ELSE 0.0 END AS DOUBLE PRECISION)"),
                "is_up",
            )
            .column(service_monitor_result::Column::Details)
            .order_by_desc(service_monitor_result::Column::Time)
            .into_model::<ServiceMonitorPoint>()
            .all(db)
            .await
    }
}
