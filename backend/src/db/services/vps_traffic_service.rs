use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set,
    TransactionTrait, IntoActiveModel, QuerySelect, // Added QuerySelect
    // Removed commented out Select and sea_query::LockType
};

use crate::db::entities::vps;

// --- Vps Traffic Service Functions ---

/// Updates VPS traffic statistics after a new performance metric is recorded.
/// This function should be called within the same transaction as saving the performance metric.
pub async fn update_vps_traffic_stats_after_metric(
    txn: &sea_orm::DatabaseTransaction, // Changed
    vps_id: i32,
    new_cumulative_rx: i64,
    new_cumulative_tx: i64,
) -> Result<(), DbErr> { // Changed
    // 1. Get the current Vps traffic stats with exclusive lock
    let vps_model = vps::Entity::find_by_id(vps_id)
        .lock_exclusive() // Equivalent to FOR UPDATE
        .one(txn)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound(format!("VPS with id {} not found", vps_id)))?;

    let last_rx = vps_model.last_processed_cumulative_rx.unwrap_or(0);
    let last_tx = vps_model.last_processed_cumulative_tx.unwrap_or(0);
    let mut current_cycle_rx = vps_model.traffic_current_cycle_rx_bytes.unwrap_or(0);
    let mut current_cycle_tx = vps_model.traffic_current_cycle_tx_bytes.unwrap_or(0);

    // 2. Calculate delta, handling counter resets
    let delta_rx = if new_cumulative_rx >= last_rx {
        new_cumulative_rx - last_rx
    } else {
        new_cumulative_rx // Counter reset
    };

    let delta_tx = if new_cumulative_tx >= last_tx {
        new_cumulative_tx - last_tx
    } else {
        new_cumulative_tx // Counter reset
    };

    // 3. Update cycle usage
    current_cycle_rx += delta_rx;
    current_cycle_tx += delta_tx;

    // 4. Update Vps table
    let mut active_vps: vps::ActiveModel = vps_model.into_active_model();
    active_vps.traffic_current_cycle_rx_bytes = Set(Some(current_cycle_rx));
    active_vps.traffic_current_cycle_tx_bytes = Set(Some(current_cycle_tx));
    active_vps.last_processed_cumulative_rx = Set(Some(new_cumulative_rx));
    active_vps.last_processed_cumulative_tx = Set(Some(new_cumulative_tx));
    active_vps.updated_at = Set(Utc::now());

    active_vps.update(txn).await?;

    Ok(())
}

/// Processes traffic reset for a single VPS if due.
/// Resets current cycle usage, updates last reset time, and calculates the next reset time.
/// Returns Ok(true) if a reset was performed, Ok(false) otherwise.
pub async fn process_vps_traffic_reset(
    db: &DatabaseConnection, // Changed
    vps_id: i32,
) -> Result<bool, DbErr> { // Changed
    let now = Utc::now();

    // Transaction for the whole process
    let txn = db.begin().await?;

    let vps_model_opt = vps::Entity::find_by_id(vps_id)
        .lock_exclusive() // Lock the row for update
        .one(&txn)
        .await?;

    if vps_model_opt.is_none() {
        txn.commit().await?; // Commit as no action was taken on this VPS
        return Ok(false);
    }
    let vps_data = vps_model_opt.unwrap();

    if vps_data.next_traffic_reset_at.is_none() || vps_data.next_traffic_reset_at.unwrap() > now {
        txn.commit().await?; // Commit as no reset is due
        return Ok(false);
    }

    let last_reset_time = vps_data.next_traffic_reset_at.unwrap();

    let new_next_reset_at: Option<DateTime<Utc>>;
    match (
        vps_data.traffic_reset_config_type.as_deref(),
        vps_data.traffic_reset_config_value.as_deref(),
    ) {
        (Some("monthly_day_of_month"), Some(value_str)) => {
            let mut day_of_month: Option<u32> = None;
            let mut time_offset_seconds: i64 = 0;

            for part in value_str.split(',') {
                let kv: Vec<&str> = part.split(':').collect();
                if kv.len() == 2 {
                    match kv[0] {
                        "day" => day_of_month = kv[1].parse().ok(),
                        "time_offset_seconds" => time_offset_seconds = kv[1].parse().unwrap_or(0),
                        _ => {}
                    }
                }
            }

            if let Some(day) = day_of_month {
                let current_reset_naive_date = last_reset_time.date_naive();
                let mut next_month_year = current_reset_naive_date.year();
                let mut next_month_month = current_reset_naive_date.month() + 1;

                if next_month_month > 12 {
                    next_month_month = 1;
                    next_month_year += 1;
                }

                let first_day_of_next_month = NaiveDate::from_ymd_opt(next_month_year, next_month_month, 1).unwrap();
                let days_in_next_month = if next_month_month == 12 {
                    NaiveDate::from_ymd_opt(next_month_year + 1, 1, 1).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(next_month_year, next_month_month + 1, 1).unwrap()
                }
                .signed_duration_since(first_day_of_next_month)
                .num_days() as u32;

                let actual_day = std::cmp::min(day, days_in_next_month);

                if let Some(naive_date_next) = NaiveDate::from_ymd_opt(next_month_year, next_month_month, actual_day) {
                    let naive_datetime_next = naive_date_next.and_hms_opt(0,0,0).unwrap_or(naive_date_next.and_hms_opt(0,0,0).expect("Should be valid time")) + Duration::seconds(time_offset_seconds);
                    new_next_reset_at = Some(DateTime::<Utc>::from_naive_utc_and_offset(naive_datetime_next, Utc));
                } else {
                    new_next_reset_at = None;
                    eprintln!("Error calculating next reset date for monthly_day_of_month for VPS ID {}: Could not form NaiveDate from y/m/d: {}/{}/{}", vps_id, next_month_year, next_month_month, actual_day);
                }
            } else {
                new_next_reset_at = None;
                eprintln!("Invalid traffic_reset_config_value (missing day) for monthly_day_of_month for VPS ID {}", vps_id);
            }
        }
        (Some("fixed_days"), Some(value_str)) => {
            if let Ok(days) = value_str.parse::<i64>() {
                if days > 0 {
                    new_next_reset_at = Some(last_reset_time + Duration::days(days));
                } else {
                    new_next_reset_at = None;
                    eprintln!("Invalid traffic_reset_config_value (days <= 0) for fixed_days for VPS ID {}", vps_id);
                }
            } else {
                new_next_reset_at = None;
                eprintln!("Invalid traffic_reset_config_value (not a number) for fixed_days for VPS ID {}", vps_id);
            }
        }
        _ => {
            new_next_reset_at = None;
            eprintln!("Missing or unknown traffic_reset_config_type or _value for VPS ID {}. Cannot calculate next reset.", vps_id);
        }
    }

    let mut active_vps: vps::ActiveModel = vps_data.into_active_model();
    active_vps.traffic_current_cycle_rx_bytes = Set(Some(0));
    active_vps.traffic_current_cycle_tx_bytes = Set(Some(0));
    active_vps.traffic_last_reset_at = Set(Some(last_reset_time));
    active_vps.next_traffic_reset_at = Set(new_next_reset_at);
    active_vps.updated_at = Set(Utc::now());

    active_vps.update(&txn).await?;

    txn.commit().await?;
    Ok(true)
}

/// Retrieves IDs of VPS that are due for a traffic reset check.
pub async fn get_vps_due_for_traffic_reset(
    db: &DatabaseConnection, // Changed
) -> Result<Vec<i32>, DbErr> { // Changed
    let now = Utc::now();
    let vps_ids: Vec<i32> = vps::Entity::find()
        .select_only()
        .column(vps::Column::Id)
        .filter(vps::Column::TrafficResetConfigType.is_not_null())
        .filter(vps::Column::TrafficResetConfigValue.is_not_null())
        .filter(vps::Column::NextTrafficResetAt.is_not_null())
        .filter(vps::Column::NextTrafficResetAt.lte(now))
        .into_tuple()
        .all(db)
        .await?;
    Ok(vps_ids)
}