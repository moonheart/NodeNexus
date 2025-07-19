use chrono::{DateTime, Duration, TimeZone, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    FromQueryResult, Order, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
    sea_query::{Alias, Expr, Query},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value as Json};
use tracing::{debug, error};

use crate::agent_service::{DiskUsage, PerformanceSnapshotBatch};
use crate::db::entities::{performance_metric, vps};
use crate::db::services::vps_traffic_service;

// --- Data Structures for API Response ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceMetricPoint {
    pub time: DateTime<Utc>,
    pub vps_id: i32,
    // Base metrics (from AVG)
    pub cpu_usage_percent: Option<f64>,
    pub memory_usage_bytes: Option<f64>,
    pub memory_total_bytes: Option<f64>,
    pub swap_usage_bytes: Option<f64>,
    pub disk_io_read_bps: Option<f64>,
    pub disk_io_write_bps: Option<f64>,
    pub network_rx_instant_bps: Option<f64>,
    pub network_tx_instant_bps: Option<f64>,
    // Aggregated disk usages
    pub used_disk_space_bytes: Option<f64>,
    pub total_disk_space_bytes: Option<f64>,
}

// --- Data Structures for Query Results ---

#[derive(FromQueryResult, Debug)]
struct UnifiedMetricResult {
    time: DateTime<Utc>,
    vps_id: i32,
    avg_cpu_usage_percent: Option<f64>,
    avg_memory_usage_bytes: Option<f64>,
    max_memory_total_bytes: Option<f64>,
    avg_swap_usage_bytes: Option<f64>,
    avg_disk_io_read_bps: Option<f64>,
    avg_disk_io_write_bps: Option<f64>,
    avg_network_rx_instant_bps: Option<f64>,
    avg_network_tx_instant_bps: Option<f64>,
    avg_used_disk_space_bytes: Option<f64>,
    avg_total_disk_space_bytes: Option<f64>,
}

/// Retrieves performance metrics for a given VPS within a time range,
/// intelligently selecting the appropriate continuous aggregate view.
pub async fn get_performance_metrics_for_vps(
    db: &DatabaseConnection,
    vps_id: i32,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
    interval_seconds: Option<u32>,
) -> Result<Vec<PerformanceMetricPoint>, DbErr> {
    // If no interval is specified, return raw data points.
    if interval_seconds.is_none() {
        debug!("No interval specified, fetching raw performance_metrics.");
        let raw_metrics = performance_metric::Entity::find()
            .filter(performance_metric::Column::VpsId.eq(vps_id))
            .filter(performance_metric::Column::Time.gte(start_time))
            .filter(performance_metric::Column::Time.lte(end_time))
            .order_by_asc(performance_metric::Column::Time)
            .all(db)
            .await?;

        let results = raw_metrics.into_iter().map(|m| PerformanceMetricPoint {
            time: m.time,
            vps_id: m.vps_id,
            cpu_usage_percent: Some(m.cpu_usage_percent),
            memory_usage_bytes: Some(m.memory_usage_bytes as f64),
            memory_total_bytes: Some(m.memory_total_bytes as f64),
            swap_usage_bytes: Some(m.swap_usage_bytes as f64),
            disk_io_read_bps: Some(m.disk_io_read_bps as f64),
            disk_io_write_bps: Some(m.disk_io_write_bps as f64),
            network_rx_instant_bps: Some(m.network_rx_instant_bps as f64),
            network_tx_instant_bps: Some(m.network_tx_instant_bps as f64),
            used_disk_space_bytes: Some(m.used_disk_space_bytes as f64),
            total_disk_space_bytes: Some(m.total_disk_space_bytes as f64),
        }).collect();
        return Ok(results);
    }

    // If an interval is specified, proceed with aggregation.
    let duration = end_time - start_time;
    let interval_secs = interval_seconds.unwrap().max(1); // We know it's Some(t) here.

    let mut query_builder = Query::select();

    let time_bucket_expr = |col: &str| -> String {
        format!("time_bucket(INTERVAL '{interval_secs} seconds', \"{col}\")")
    };

    if duration <= Duration::hours(1) {
        // Query raw data for recent queries (<= 1 hour)
        debug!("Querying raw performance_metrics for recent data with interval.");
        query_builder
            .from(performance_metric::Entity)
            .expr_as(Expr::cust(time_bucket_expr("time")), Alias::new("time"))
            .column(performance_metric::Column::VpsId)
            .expr_as(Expr::cust("AVG(cpu_usage_percent)::double precision"), Alias::new("avg_cpu_usage_percent"))
            .expr_as(Expr::cust("AVG(memory_usage_bytes)::double precision"), Alias::new("avg_memory_usage_bytes"))
            .expr_as(Expr::cust("MAX(memory_total_bytes)::double precision"), Alias::new("max_memory_total_bytes"))
            .expr_as(Expr::cust("AVG(swap_usage_bytes)::double precision"), Alias::new("avg_swap_usage_bytes"))
            .expr_as(Expr::cust("AVG(disk_io_read_bps)::double precision"), Alias::new("avg_disk_io_read_bps"))
            .expr_as(Expr::cust("AVG(disk_io_write_bps)::double precision"), Alias::new("avg_disk_io_write_bps"))
            .expr_as(Expr::cust("AVG(network_rx_instant_bps)::double precision"), Alias::new("avg_network_rx_instant_bps"))
            .expr_as(Expr::cust("AVG(network_tx_instant_bps)::double precision"), Alias::new("avg_network_tx_instant_bps"))
            .expr_as(Expr::cust("AVG(used_disk_space_bytes)::double precision"), Alias::new("avg_used_disk_space_bytes"))
            .expr_as(Expr::cust("AVG(total_disk_space_bytes)::double precision"), Alias::new("avg_total_disk_space_bytes"))
            .and_where(Expr::col(performance_metric::Column::VpsId).eq(vps_id))
            .and_where(Expr::col(performance_metric::Column::Time).gte(start_time))
            .and_where(Expr::col(performance_metric::Column::Time).lte(end_time))
            .add_group_by(vec![Expr::cust("1"), Expr::cust("2")]) // Group by time bucket and vps_id
            .order_by(Alias::new("time"), Order::Asc);
    } else {
        // Query aggregated data for older queries
        let (metric_source, time_col) = if duration <= Duration::days(7) {
            ("performance_metrics_summary_1m", "bucket")
        } else if duration <= Duration::days(30) {
            ("performance_metrics_summary_1h", "bucket")
        } else {
            ("performance_metrics_summary_1d", "bucket")
        };
        debug!(?duration, ?interval_seconds, metric_source, "Choosing aggregated data source for performance query");

        query_builder
            .from(Alias::new(metric_source))
            .expr_as(Expr::cust(time_bucket_expr(time_col)), Alias::new("time"))
            .column(Alias::new("vps_id"))
            .expr_as(Expr::cust("AVG(avg_cpu_usage_percent)::double precision"), Alias::new("avg_cpu_usage_percent"))
            .expr_as(Expr::cust("AVG(avg_memory_usage_bytes)::double precision"), Alias::new("avg_memory_usage_bytes"))
            .expr_as(Expr::cust("MAX(max_memory_total_bytes)::double precision"), Alias::new("max_memory_total_bytes"))
            .expr_as(Expr::cust("AVG(avg_swap_usage_bytes)::double precision"), Alias::new("avg_swap_usage_bytes"))
            .expr_as(Expr::cust("AVG(avg_disk_io_read_bps)::double precision"), Alias::new("avg_disk_io_read_bps"))
            .expr_as(Expr::cust("AVG(avg_disk_io_write_bps)::double precision"), Alias::new("avg_disk_io_write_bps"))
            .expr_as(Expr::cust("AVG(avg_network_rx_instant_bps)::double precision"), Alias::new("avg_network_rx_instant_bps"))
            .expr_as(Expr::cust("AVG(avg_network_tx_instant_bps)::double precision"), Alias::new("avg_network_tx_instant_bps"))
            .expr_as(Expr::cust("AVG(avg_used_disk_space_bytes)::double precision"), Alias::new("avg_used_disk_space_bytes"))
            .expr_as(Expr::cust("AVG(avg_total_disk_space_bytes)::double precision"), Alias::new("avg_total_disk_space_bytes"))
            .and_where(Expr::col("vps_id").eq(vps_id))
            .and_where(Expr::col(time_col).gte(start_time))
            .and_where(Expr::col(time_col).lte(end_time))
            .add_group_by(vec![Expr::cust("1"), Expr::cust("2")]) // Group by time bucket and vps_id
            .order_by(Alias::new("time"), Order::Asc);
    }

    let metric_results = UnifiedMetricResult::find_by_statement(db.get_database_backend().build(&query_builder)).all(db).await?;

    // --- Map results to response struct ---
    let results = metric_results.into_iter().map(|metric_res| {
        PerformanceMetricPoint {
            time: metric_res.time,
            vps_id: metric_res.vps_id,
            cpu_usage_percent: metric_res.avg_cpu_usage_percent,
            memory_usage_bytes: metric_res.avg_memory_usage_bytes,
            memory_total_bytes: metric_res.max_memory_total_bytes,
            swap_usage_bytes: metric_res.avg_swap_usage_bytes,
            disk_io_read_bps: metric_res.avg_disk_io_read_bps,
            disk_io_write_bps: metric_res.avg_disk_io_write_bps,
            network_rx_instant_bps: metric_res.avg_network_rx_instant_bps,
            network_tx_instant_bps: metric_res.avg_network_tx_instant_bps,
            used_disk_space_bytes: metric_res.avg_used_disk_space_bytes,
            total_disk_space_bytes: metric_res.avg_total_disk_space_bytes,
        }
    }).collect();

    Ok(results)
}

/// Retrieves the latest performance metric for a given VPS.
pub async fn get_latest_performance_metric_for_vps(
    db: &DatabaseConnection,
    vps_id: i32,
) -> Result<Option<performance_metric::Model>, DbErr> {
    performance_metric::Entity::find()
        .filter(performance_metric::Column::VpsId.eq(vps_id))
        .order_by_desc(performance_metric::Column::Time)
        .one(db)
        .await
}

/// Saves a batch of performance snapshots for a given VPS.
/// This includes the main metrics, detailed disk usage, and detailed network interface stats.
/// Saves a batch of performance snapshots for a given VPS.
/// This function now saves aggregated disk usage into `performance_metrics`
/// and detailed disk usage into the `vps.metadata` JSONB column.
pub async fn save_performance_snapshot_batch(
    db: &DatabaseConnection,
    vps_id: i32,
    batch: &PerformanceSnapshotBatch,
) -> Result<Vec<performance_metric::Model>, DbErr> {
    if batch.snapshots.is_empty() {
        return Ok(Vec::new());
    }

    let txn = db.begin().await?;
    let mut saved_metrics = Vec::with_capacity(batch.snapshots.len());

    for snapshot in &batch.snapshots {
        let timestamp = Utc
            .timestamp_millis_opt(snapshot.timestamp_unix_ms)
            .single()
            .unwrap_or_else(Utc::now);

        let metric_active_model = performance_metric::ActiveModel {
            time: Set(timestamp),
            vps_id: Set(vps_id),
            cpu_usage_percent: Set(snapshot.cpu_overall_usage_percent as f64),
            memory_usage_bytes: Set(snapshot.memory_usage_bytes as i64),
            memory_total_bytes: Set(snapshot.memory_total_bytes as i64),
            swap_usage_bytes: Set(snapshot.swap_usage_bytes as i64),
            swap_total_bytes: Set(snapshot.swap_total_bytes as i64),
            disk_io_read_bps: Set(snapshot.disk_total_io_read_bytes_per_sec as i64),
            disk_io_write_bps: Set(snapshot.disk_total_io_write_bytes_per_sec as i64),
            // New aggregated disk fields
            total_disk_space_bytes: Set(snapshot.total_disk_space_bytes as i64),
            used_disk_space_bytes: Set(snapshot.used_disk_space_bytes as i64),
            // Network fields
            network_rx_cumulative: Set(snapshot.network_rx_bytes_cumulative as i64),
            network_tx_cumulative: Set(snapshot.network_tx_bytes_cumulative as i64),
            network_rx_instant_bps: Set(snapshot.network_rx_bytes_per_sec as i64),
            network_tx_instant_bps: Set(snapshot.network_tx_bytes_per_sec as i64),
            // Other system stats
            uptime_seconds: Set(snapshot.uptime_seconds as i64),
            total_processes_count: Set(snapshot.total_processes_count as i32),
            running_processes_count: Set(snapshot.running_processes_count as i32),
            tcp_established_connection_count: Set(snapshot.tcp_established_connection_count as i32),
        };
        let metric_model = metric_active_model.insert(&txn).await?;
        saved_metrics.push(metric_model);
    }

    // After saving all metrics, update related VPS data using the LATEST snapshot.
    if let Some(last_snapshot) = batch.snapshots.last() {
        // 1. Update VPS traffic stats
        if let Err(e) = vps_traffic_service::update_vps_traffic_stats_after_metric(
            &txn,
            vps_id,
            last_snapshot.network_rx_bytes_cumulative as i64,
            last_snapshot.network_tx_bytes_cumulative as i64,
        )
        .await
        {
            error!(vps_id, error = %e, "Failed to update VPS traffic stats. Rolling back.");
            txn.rollback().await?;
            return Err(e);
        }

        // 2. Update VPS metadata with the latest detailed disk usage
        if let Err(e) = update_vps_disk_usage_metadata(&txn, vps_id, &last_snapshot.disk_usages).await {
            error!(vps_id, error = %e, "Failed to update VPS disk usage metadata. Rolling back.");
            txn.rollback().await?;
            return Err(e);
        }
    }

    txn.commit().await?;
    Ok(saved_metrics)
}

/// Helper function to update the `metadata` field of a VPS with disk usage details.
async fn update_vps_disk_usage_metadata(
    txn: &impl ConnectionTrait,
    vps_id: i32,
    disk_usages: &[DiskUsage],
) -> Result<(), DbErr> {
    // Find the VPS to get its current metadata
    let vps_model = vps::Entity::find_by_id(vps_id)
        .one(txn)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound(format!("VPS with ID {vps_id} not found")))?;

    // Serialize the detailed disk usage into a JSON value
    let disk_usage_json = json!(disk_usages);

    // Get current metadata or create a new JSON object
    let mut current_metadata = vps_model.metadata.unwrap_or_else(|| Json::Object(Default::default()));

    // Update the 'diskUsage' key within the metadata
    if let Some(obj) = current_metadata.as_object_mut() {
        obj.insert("diskUsage".to_string(), disk_usage_json);
    } else {
        // This case should be rare (if metadata is not an object), but we handle it by creating a new object.
        current_metadata = json!({ "diskUsage": disk_usage_json });
    }

    // Create an active model to update the record
    let vps_active_model = vps::ActiveModel {
        id: Set(vps_id),
        metadata: Set(Some(current_metadata)),
        ..Default::default() // Important: only updates specified fields
    };

    vps::Entity::update(vps_active_model).exec(txn).await?;

    Ok(())
}

/// Retrieves the summary of total and used disk space from the latest performance metric.
/// This now reads directly from the latest `performance_metric` record.
pub async fn get_latest_disk_usage_summary(
    db: &DatabaseConnection,
    vps_id: i32,
) -> Result<Option<(i64, i64)>, DbErr> {
    let latest_metric_opt = performance_metric::Entity::find()
        .filter(performance_metric::Column::VpsId.eq(vps_id))
        .order_by_desc(performance_metric::Column::Time)
        .one(db)
        .await?;

    if let Some(latest_metric) = latest_metric_opt {
        Ok(Some((
            latest_metric.total_disk_space_bytes,
            latest_metric.used_disk_space_bytes,
        )))
    } else {
        Ok(None) // No performance metrics for this VPS
    }
}
