use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use sqlx::{PgPool, Result};

use crate::db::models::Vps;

// --- Vps Traffic Service Functions ---

/// Updates VPS traffic statistics after a new performance metric is recorded.
/// This function should be called within the same transaction as saving the performance metric.
pub async fn update_vps_traffic_stats_after_metric(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    vps_id: i32,
    new_cumulative_rx: i64,
    new_cumulative_tx: i64,
) -> Result<()> {
    // 1. Get the current Vps traffic stats
    let vps = sqlx::query_as!(
        Vps,
        r#"
        SELECT 
            id, user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at, "group",
            agent_config_override, config_status, last_config_update_at, last_config_error,
            traffic_limit_bytes, traffic_billing_rule, traffic_current_cycle_rx_bytes, traffic_current_cycle_tx_bytes,
            last_processed_cumulative_rx, last_processed_cumulative_tx, traffic_last_reset_at,
            traffic_reset_config_type, traffic_reset_config_value, next_traffic_reset_at
        FROM vps WHERE id = $1 FOR UPDATE
        "#,
        vps_id
    )
    .fetch_one(&mut **tx)
    .await?;

    let last_rx = vps.last_processed_cumulative_rx.unwrap_or(0);
    let last_tx = vps.last_processed_cumulative_tx.unwrap_or(0);
    let mut current_cycle_rx = vps.traffic_current_cycle_rx_bytes.unwrap_or(0);
    let mut current_cycle_tx = vps.traffic_current_cycle_tx_bytes.unwrap_or(0);

    // 2. Calculate delta, handling counter resets
    let delta_rx = if new_cumulative_rx >= last_rx {
        new_cumulative_rx - last_rx
    } else {
        new_cumulative_rx 
    };

    let delta_tx = if new_cumulative_tx >= last_tx {
        new_cumulative_tx - last_tx
    } else {
        new_cumulative_tx
    };

    // 3. Update cycle usage
    current_cycle_rx += delta_rx;
    current_cycle_tx += delta_tx;

    // 4. Update Vps table
    sqlx::query!(
        r#"
        UPDATE vps
        SET
            traffic_current_cycle_rx_bytes = $1,
            traffic_current_cycle_tx_bytes = $2,
            last_processed_cumulative_rx = $3,
            last_processed_cumulative_tx = $4,
            updated_at = $5
        WHERE id = $6
        "#,
        current_cycle_rx,
        current_cycle_tx,
        new_cumulative_rx,
        new_cumulative_tx,
        Utc::now(),
        vps_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// Processes traffic reset for a single VPS if due.
/// Resets current cycle usage, updates last reset time, and calculates the next reset time.
/// Returns Ok(true) if a reset was performed, Ok(false) otherwise.
pub async fn process_vps_traffic_reset(pool: &PgPool, vps_id: i32) -> Result<bool> {
    let mut tx = pool.begin().await?;
    let now = Utc::now();

    let vps = sqlx::query_as!(
        Vps,
        r#"
        SELECT 
            id, user_id, name, ip_address, os_type, agent_secret, status, metadata, created_at, updated_at, "group",
            agent_config_override, config_status, last_config_update_at, last_config_error,
            traffic_limit_bytes, traffic_billing_rule, traffic_current_cycle_rx_bytes, traffic_current_cycle_tx_bytes,
            last_processed_cumulative_rx, last_processed_cumulative_tx, traffic_last_reset_at,
            traffic_reset_config_type, traffic_reset_config_value, next_traffic_reset_at
        FROM vps WHERE id = $1 FOR UPDATE
        "#,
        vps_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    if vps.is_none() {
        tx.commit().await?;
        return Ok(false); 
    }
    let vps_data = vps.unwrap();

    if vps_data.next_traffic_reset_at.is_none() || vps_data.next_traffic_reset_at.unwrap() > now {
        tx.commit().await?;
        return Ok(false);
    }

    let last_reset_time = vps_data.next_traffic_reset_at.unwrap(); 

    let new_next_reset_at: Option<DateTime<Utc>>;
    match (vps_data.traffic_reset_config_type.as_deref(), vps_data.traffic_reset_config_value.as_deref()) {
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
                }.signed_duration_since(first_day_of_next_month).num_days() as u32;
                
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

    sqlx::query!(
        r#"
        UPDATE vps
        SET
            traffic_current_cycle_rx_bytes = 0,
            traffic_current_cycle_tx_bytes = 0,
            traffic_last_reset_at = $1,
            next_traffic_reset_at = $2,
            updated_at = $3
        WHERE id = $4
        "#,
        last_reset_time,
        new_next_reset_at,
        Utc::now(),
        vps_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(true)
}

/// Retrieves IDs of VPS that are due for a traffic reset check.
pub async fn get_vps_due_for_traffic_reset(pool: &PgPool) -> Result<Vec<i32>> {
    let now = Utc::now();
    let vps_ids = sqlx::query!(
        r#"
        SELECT id
        FROM vps
        WHERE traffic_reset_config_type IS NOT NULL 
          AND traffic_reset_config_value IS NOT NULL
          AND next_traffic_reset_at IS NOT NULL
          AND next_traffic_reset_at <= $1
        "#,
        now
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| row.id)
    .collect();
    Ok(vps_ids)
}