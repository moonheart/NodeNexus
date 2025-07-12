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
