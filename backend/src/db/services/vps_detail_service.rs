use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, ModelTrait, QueryFilter, QueryOrder,
}; // Removed QuerySelect
// Removed use sea_orm::sea_query::Expr;

use crate::db::entities::{
    performance_disk_usage,
    performance_metric,
    tag,
    vps,
    vps_renewal_info, // Removed vps_tag
};
use crate::web::models::websocket_models::{
    ServerBasicInfo, ServerMetricsSnapshot, ServerWithDetails, Tag as WebsocketTag,
};

// --- Vps Detail Service Functions ---

async fn build_server_with_details(
    db: &DatabaseConnection,
    vps_model: vps::Model,
) -> Result<ServerWithDetails, DbErr> {
    let _vps_id = vps_model.id; // Renamed vps_id

    // Fetch latest performance metric
    let latest_metric_opt: Option<performance_metric::Model> = vps_model
        .find_related(performance_metric::Entity)
        .order_by_desc(performance_metric::Column::Time)
        .one(db)
        .await?;

    let mut latest_metrics_snapshot: Option<ServerMetricsSnapshot> = None;
    // total_disk_used_bytes and total_disk_total_bytes moved inside the if block

    if let Some(latest_metric) = &latest_metric_opt {
        // Fetch disk usages for the latest metric
        let disk_usages: Vec<performance_disk_usage::Model> = latest_metric
            .find_related(performance_disk_usage::Entity)
            .all(db)
            .await?;

        let total_disk_used_bytes: i64 = disk_usages.iter().map(|du| du.used_bytes).sum();
        let total_disk_total_bytes: i64 = disk_usages.iter().map(|du| du.total_bytes).sum();

        latest_metrics_snapshot = Some(ServerMetricsSnapshot {
            time: latest_metric.time,
            cpu_usage_percent: latest_metric.cpu_usage_percent as f32,
            memory_usage_bytes: latest_metric.memory_usage_bytes as u64,
            memory_total_bytes: latest_metric.memory_total_bytes as u64,
            network_rx_instant_bps: Some(latest_metric.network_rx_instant_bps as u64),
            network_tx_instant_bps: Some(latest_metric.network_tx_instant_bps as u64),
            uptime_seconds: Some(latest_metric.uptime_seconds as u64),
            disk_used_bytes: Some(total_disk_used_bytes as u64),
            disk_total_bytes: Some(total_disk_total_bytes as u64),
            disk_io_read_bps: Some(latest_metric.disk_io_read_bps as u64),
            disk_io_write_bps: Some(latest_metric.disk_io_write_bps as u64),
            // Populate newly added fields
            swap_usage_bytes: Some(latest_metric.swap_usage_bytes as u64),
            swap_total_bytes: Some(latest_metric.swap_total_bytes as u64),
            network_rx_cumulative: Some(latest_metric.network_rx_cumulative as u64), // Cumulative
            network_tx_cumulative: Some(latest_metric.network_tx_cumulative as u64), // Cumulative
            total_processes_count: Some(latest_metric.total_processes_count as u32),
            running_processes_count: Some(latest_metric.running_processes_count as u32),
            tcp_established_connection_count: Some(
                latest_metric.tcp_established_connection_count as u32,
            ),
        });
    }

    // Fetch tags
    let tags_models: Vec<tag::Model> = vps_model.find_related(tag::Entity).all(db).await?;
    let ws_tags: Option<Vec<WebsocketTag>> = if !tags_models.is_empty() {
        Some(
            tags_models
                .into_iter()
                .map(|t| WebsocketTag {
                    id: t.id,
                    name: t.name,
                    color: t.color,
                    icon: t.icon,
                    url: t.url,
                    is_visible: t.is_visible,
                })
                .collect(),
        )
    } else {
        None
    };

    // Fetch renewal info
    let renewal_info_opt: Option<vps_renewal_info::Model> = vps_model
        .find_related(vps_renewal_info::Entity)
        .one(db)
        .await?;

    let basic_info = ServerBasicInfo {
        id: vps_model.id,
        user_id: vps_model.user_id,
        name: vps_model.name,
        ip_address: vps_model.ip_address,
        status: vps_model.status,
        agent_version: vps_model.agent_version,
        group: vps_model.group,
        tags: ws_tags,
        config_status: vps_model.config_status,
        last_config_update_at: vps_model.last_config_update_at,
        last_config_error: vps_model.last_config_error,
        traffic_limit_bytes: vps_model.traffic_limit_bytes,
        traffic_billing_rule: vps_model.traffic_billing_rule,
        traffic_current_cycle_rx_bytes: vps_model.traffic_current_cycle_rx_bytes,
        traffic_current_cycle_tx_bytes: vps_model.traffic_current_cycle_tx_bytes,
        traffic_last_reset_at: vps_model.traffic_last_reset_at,
        traffic_reset_config_type: vps_model.traffic_reset_config_type,
        traffic_reset_config_value: vps_model.traffic_reset_config_value,
        next_traffic_reset_at: vps_model.next_traffic_reset_at,
    };

    Ok(ServerWithDetails {
        basic_info,
        latest_metrics: latest_metrics_snapshot,
        os_type: vps_model.os_type,
        created_at: vps_model.created_at,
        metadata: vps_model.metadata,
        renewal_cycle: renewal_info_opt
            .as_ref()
            .and_then(|ri| ri.renewal_cycle.clone()),
        renewal_cycle_custom_days: renewal_info_opt
            .as_ref()
            .and_then(|ri| ri.renewal_cycle_custom_days),
        renewal_price: renewal_info_opt.as_ref().and_then(|ri| ri.renewal_price),
        renewal_currency: renewal_info_opt
            .as_ref()
            .and_then(|ri| ri.renewal_currency.clone()),
        next_renewal_date: renewal_info_opt
            .as_ref()
            .and_then(|ri| ri.next_renewal_date),
        last_renewal_date: renewal_info_opt
            .as_ref()
            .and_then(|ri| ri.last_renewal_date),
        service_start_date: renewal_info_opt
            .as_ref()
            .and_then(|ri| ri.service_start_date),
        payment_method: renewal_info_opt
            .as_ref()
            .and_then(|ri| ri.payment_method.clone()),
        auto_renew_enabled: renewal_info_opt
            .as_ref()
            .and_then(|ri| ri.auto_renew_enabled),
        renewal_notes: renewal_info_opt
            .as_ref()
            .and_then(|ri| ri.renewal_notes.clone()),
        reminder_active: renewal_info_opt.as_ref().and_then(|ri| ri.reminder_active),
    })
}

/// Retrieves all VPS along with their latest metrics and disk usage for cache initialization.
pub async fn get_all_vps_with_details_for_cache(
    db: &DatabaseConnection,
) -> Result<Vec<ServerWithDetails>, DbErr> {
    let all_vps: Vec<vps::Model> = vps::Entity::find()
        .order_by_asc(vps::Column::Id)
        .all(db)
        .await?;
    let mut servers_with_details = Vec::new();

    for vps_model in all_vps {
        servers_with_details.push(build_server_with_details(db, vps_model).await?);
    }

    Ok(servers_with_details)
}

/// Retrieves all VPS for a specific user along with their latest metrics and disk usage.
pub async fn get_all_vps_with_details_for_user(
    db: &DatabaseConnection,
    user_id: i32,
) -> Result<Vec<ServerWithDetails>, DbErr> {
    let user_vps: Vec<vps::Model> = vps::Entity::find()
        .filter(vps::Column::UserId.eq(user_id))
        .order_by_asc(vps::Column::Id)
        .all(db)
        .await?;

    let mut servers_with_details = Vec::new();
    for vps_model in user_vps {
        servers_with_details.push(build_server_with_details(db, vps_model).await?);
    }
    Ok(servers_with_details)
}

/// Retrieves a single VPS with its full details for cache updates.
pub async fn get_vps_with_details_for_cache_by_id(
    db: &DatabaseConnection,
    vps_id_param: i32, // Renamed to avoid conflict
) -> Result<Option<ServerWithDetails>, DbErr> {
    let vps_model_opt: Option<vps::Model> = vps::Entity::find_by_id(vps_id_param).one(db).await?;

    if let Some(vps_model) = vps_model_opt {
        Ok(Some(build_server_with_details(db, vps_model).await?))
    } else {
        Ok(None)
    }
}
