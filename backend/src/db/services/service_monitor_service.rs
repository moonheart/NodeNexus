//! Service for managing service monitors and their results.
//!
//! This service provides functions for CRUD operations on service monitors,
//! assigning them to agents/tags, and recording check results.

use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, ModelTrait, QueryFilter, QueryOrder, Set, TransactionTrait};
use crate::db::entities::{prelude::*, service_monitor, service_monitor_agent, service_monitor_tag, vps};
use crate::http_server::models::service_monitor_models::{CreateMonitor, ServiceMonitorDetails, ServiceMonitorResultDetails, UpdateMonitor};
use std::collections::HashMap;
use futures::try_join;

use crate::agent_service::{ServiceMonitorResult, ServiceMonitorTask};
use crate::db::entities::{service_monitor_result, vps_tag};
use sea_orm::QuerySelect;
use chrono::{DateTime, TimeZone, Utc};
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
            ..Default::default()
        };

        let txn = db.begin().await?;

        let saved_monitor = new_monitor.insert(&txn).await?;

        // Handle agent assignments
        if let Some(agent_ids) = monitor_data.assignments.agent_ids {
            if !agent_ids.is_empty() {
                let agent_assignments = agent_ids.into_iter().map(|vps_id| {
                    service_monitor_agent::ActiveModel {
                        monitor_id: Set(saved_monitor.id),
                        vps_id: Set(vps_id),
                    }
                });
                ServiceMonitorAgent::insert_many(agent_assignments).exec(&txn).await?;
            }
        }

        // Handle tag assignments
        if let Some(tag_ids) = monitor_data.assignments.tag_ids {
            if !tag_ids.is_empty() {
                let tag_assignments = tag_ids.into_iter().map(|tag_id| {
                    service_monitor_tag::ActiveModel {
                        monitor_id: Set(saved_monitor.id),
                        tag_id: Set(tag_id),
                    }
                });
                ServiceMonitorTag::insert_many(tag_assignments).exec(&txn).await?;
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
        agent_map.entry(agent.monitor_id).or_default().push(agent.vps_id);
    }

    let mut tag_map: HashMap<i32, Vec<i32>> = HashMap::new();
    for tag in tag_assignments {
        tag_map.entry(tag.monitor_id).or_default().push(tag.tag_id);
    }

    // 4. Construct the final response models
    let details_list = monitors
        .into_iter()
        .map(|monitor| {
            let monitor_id = monitor.id;
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
        .ok_or_else(|| DbErr::RecordNotFound("Monitor not found or permission denied".to_string()))?;

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
                let agent_assignments = agent_ids.into_iter().map(|vps_id| {
                    service_monitor_agent::ActiveModel {
                        monitor_id: Set(monitor_id),
                        vps_id: Set(vps_id),
                    }
                });
                ServiceMonitorAgent::insert_many(agent_assignments).exec(&txn).await?;
            }
        }

        // Add new tag assignments
        if let Some(tag_ids) = assignments.tag_ids {
            if !tag_ids.is_empty() {
                let tag_assignments = tag_ids.into_iter().map(|tag_id| {
                    service_monitor_tag::ActiveModel {
                        monitor_id: Set(monitor_id),
                        tag_id: Set(tag_id),
                    }
                });
                ServiceMonitorTag::insert_many(tag_assignments).exec(&txn).await?;
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
        .ok_or_else(|| DbErr::RecordNotFound("Failed to fetch updated monitor details.".to_string()))?;

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
/// Fetches all active service monitoring tasks assigned to a specific agent (VPS).
///
/// This function determines the full set of monitors for an agent by combining:
/// 1. Monitors directly assigned to the agent's `vps_id`.
/// 2. Monitors assigned to tags that the agent possesses.
///
/// It then fetches the details for these monitors, ensuring only active ones are returned,
/// and transforms them into the gRPC `ServiceMonitorTask` format.
pub async fn get_tasks_for_agent(
    db: &DatabaseConnection,
    vps_id: i32,
) -> Result<Vec<ServiceMonitorTask>, DbErr> {
    // 1. Get monitor IDs from direct agent assignments
    let direct_monitor_ids_future = ServiceMonitorAgent::find()
        .select_only()
        .column(service_monitor_agent::Column::MonitorId)
        .filter(service_monitor_agent::Column::VpsId.eq(vps_id))
        .into_tuple::<i32>()
        .all(db);

    // 2. Get monitor IDs from tag-based assignments
    // First, find all tags for the given vps_id
    let agent_tags_future = VpsTag::find()
        .select_only()
        .column(vps_tag::Column::TagId)
        .filter(vps_tag::Column::VpsId.eq(vps_id))
        .into_tuple::<i32>()
        .all(db);

    let (direct_monitor_ids, agent_tags) = try_join!(direct_monitor_ids_future, agent_tags_future)?;

    let mut tagged_monitor_ids: Vec<i32> = Vec::new();
    if !agent_tags.is_empty() {
        // Then, find all monitors associated with those tags
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

    // 4. Fetch all active monitors corresponding to the collected IDs
    let monitors = ServiceMonitor::find()
        .filter(service_monitor::Column::Id.is_in(all_monitor_ids))
        .filter(service_monitor::Column::IsActive.eq(true))
        .all(db)
        .await?;

    // 5. Map the database models to the gRPC task models
    let tasks = monitors
        .into_iter()
        .map(|monitor| ServiceMonitorTask {
            monitor_id: monitor.id,
            name: monitor.name,
            monitor_type: monitor.monitor_type,
            target: monitor.target,
            frequency_seconds: monitor.frequency_seconds,
            // Safely handle optional JSON, defaulting to an empty JSON object string
            monitor_config_json: monitor
                .monitor_config
                .as_ref()
                .map_or_else(|| "{}".to_string(), |json| json.to_string()),
            timeout_seconds: monitor.timeout_seconds,
        })
        .collect();

    Ok(tasks)
}
/// Given a monitor ID, finds all VPS IDs that should be running this monitor.
///
/// This is determined by looking at both direct agent assignments and tag-based assignments.
/// It's a helper function to determine which agents are affected by a monitor change.
pub async fn get_vps_ids_for_monitor(
    db: &DatabaseConnection,
    monitor_id: i32,
) -> Result<Vec<i32>, DbErr> {
    // 1. Get VPS IDs from direct assignments
    let direct_vps_ids_future = ServiceMonitorAgent::find()
        .select_only()
        .column(service_monitor_agent::Column::VpsId)
        .filter(service_monitor_agent::Column::MonitorId.eq(monitor_id))
        .into_tuple::<i32>()
        .all(db);

    // 2. Get VPS IDs from tag-based assignments
    // First, find all tags for the given monitor
    let monitor_tags_future = ServiceMonitorTag::find()
        .select_only()
        .column(service_monitor_tag::Column::TagId)
        .filter(service_monitor_tag::Column::MonitorId.eq(monitor_id))
        .into_tuple::<i32>()
        .all(db);

    let (direct_vps_ids, monitor_tags) = try_join!(direct_vps_ids_future, monitor_tags_future)?;

    let mut tagged_vps_ids: Vec<i32> = Vec::new();
    if !monitor_tags.is_empty() {
        // Then, find all VPSs that have those tags
        tagged_vps_ids = VpsTag::find()
            .select_only()
            .column(vps_tag::Column::VpsId)
            .filter(vps_tag::Column::TagId.is_in(monitor_tags))
            .into_tuple::<i32>()
            .all(db)
            .await?;
    }

    // 3. Combine and deduplicate VPS IDs
    let mut all_vps_ids = direct_vps_ids;
    all_vps_ids.extend(tagged_vps_ids);
    all_vps_ids.sort_unstable();
    all_vps_ids.dedup();

    Ok(all_vps_ids)
}

pub async fn record_monitor_result(
    db: &DatabaseConnection,
    agent_id: i32, // This is the vps_id
    result: &ServiceMonitorResult,
) -> Result<(), DbErr> {
    let new_result = service_monitor_result::ActiveModel {
        time: Set(Utc.timestamp_millis_opt(result.timestamp_unix_ms).unwrap()),
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
    start_time: Option<DateTime<Utc>>,
    end_time: Option<DateTime<Utc>>,
    limit: Option<u64>,
) -> Result<Vec<ServiceMonitorResultDetails>, DbErr> {
    let mut query = service_monitor_result::Entity::find()
        .filter(service_monitor_result::Column::MonitorId.eq(monitor_id));

    if let Some(start) = start_time {
        query = query.filter(service_monitor_result::Column::Time.gte(start));
    }
    if let Some(end) = end_time {
        query = query.filter(service_monitor_result::Column::Time.lte(end));
    }

    if let Some(limit_val) = limit {
        query = query.limit(limit_val);
    }

    let results = query
        .order_by_desc(service_monitor_result::Column::Time)
        .all(db)
        .await?;

    if results.is_empty() {
        return Ok(Vec::new());
    }

    // 2. Collect all unique agent IDs from the results
    let agent_ids: Vec<i32> = results.iter().map(|r| r.agent_id).collect::<std::collections::HashSet<_>>().into_iter().collect();

    // 3. Fetch the names for these agents
    let agents = Vps::find()
        .filter(vps::Column::Id.is_in(agent_ids))
        .all(db)
        .await?;

    // 4. Create a map from agent_id to agent_name for quick lookup
    let agent_name_map: HashMap<i32, String> = agents.into_iter().map(|a| (a.id, a.name)).collect();

    // 5. Construct the final detailed response
    let result_details = results
        .into_iter()
        .map(|result| {
            let agent_name = agent_name_map.get(&result.agent_id).cloned().unwrap_or_else(|| "Unknown Agent".to_string());
            ServiceMonitorResultDetails {
                time: result.time.to_rfc3339(),
                monitor_id: result.monitor_id,
                agent_id: result.agent_id,
                agent_name,
                is_up: result.is_up,
                latency_ms: result.latency_ms,
                details: result.details,
            }
        })
        .collect();

    Ok(result_details)
}