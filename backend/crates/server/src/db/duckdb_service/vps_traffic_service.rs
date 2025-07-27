use crate::db::duckdb_service::DuckDbPool;
use crate::db::entities::vps;
use crate::web::error::AppError;
use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc};
use duckdb::{params, Row, Transaction};
use tracing::error;

fn row_to_vps_model(row: &Row) -> Result<vps::Model, duckdb::Error> {
    Ok(vps::Model {
        id: row.get("id")?,
        user_id: row.get("user_id")?,
        name: row.get("name")?,
        ip_address: row.get("ip_address")?,
        os_type: row.get("os_type")?,
        agent_secret: row.get("agent_secret")?,
        agent_version: row.get("agent_version")?,
        status: row.get("status")?,
        metadata: super::json_from_row(row, "metadata")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        group: row.get("group")?,
        agent_config_override: super::json_from_row(row, "agent_config_override")?,
        config_status: row.get("config_status")?,
        last_config_update_at: row.get("last_config_update_at")?,
        last_config_error: row.get("last_config_error")?,
        traffic_limit_bytes: row.get("traffic_limit_bytes")?,
        traffic_billing_rule: row.get("traffic_billing_rule")?,
        traffic_current_cycle_rx_bytes: row.get("traffic_current_cycle_rx_bytes")?,
        traffic_current_cycle_tx_bytes: row.get("traffic_current_cycle_tx_bytes")?,
        last_processed_cumulative_rx: row.get("last_processed_cumulative_rx")?,
        last_processed_cumulative_tx: row.get("last_processed_cumulative_tx")?,
        traffic_last_reset_at: row.get("traffic_last_reset_at")?,
        traffic_reset_config_type: row.get("traffic_reset_config_type")?,
        traffic_reset_config_value: row.get("traffic_reset_config_value")?,
        next_traffic_reset_at: row.get("next_traffic_reset_at")?,
    })
}

/// Updates VPS traffic statistics after a new performance metric is recorded.
pub fn update_vps_traffic_stats_after_metric(
    txn: &Transaction,
    vps_id: i32,
    new_cumulative_rx: i64,
    new_cumulative_tx: i64,
) -> Result<(), AppError> {
    let vps_model: vps::Model = txn.query_row(
        "SELECT * FROM vps WHERE id = ?",
        params![vps_id],
        row_to_vps_model,
    )?;

    let last_rx = vps_model.last_processed_cumulative_rx.unwrap_or(0);
    let last_tx = vps_model.last_processed_cumulative_tx.unwrap_or(0);
    let mut current_cycle_rx = vps_model.traffic_current_cycle_rx_bytes.unwrap_or(0);
    let mut current_cycle_tx = vps_model.traffic_current_cycle_tx_bytes.unwrap_or(0);

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

    current_cycle_rx += delta_rx;
    current_cycle_tx += delta_tx;

    txn.execute(
        "UPDATE vps SET traffic_current_cycle_rx_bytes = ?, traffic_current_cycle_tx_bytes = ?, last_processed_cumulative_rx = ?, last_processed_cumulative_tx = ?, updated_at = ? WHERE id = ?",
        params![
            current_cycle_rx,
            current_cycle_tx,
            new_cumulative_rx,
            new_cumulative_tx,
            Utc::now(),
            vps_id,
        ],
    )?;

    Ok(())
}

/// Processes traffic reset for a single VPS if due.
pub async fn process_vps_traffic_reset(
    pool: DuckDbPool,
    vps_id: i32,
) -> Result<bool, AppError> {
    let mut conn = pool.get()?;
    let now = Utc::now();

    let tx = conn.transaction()?;

    let vps_model_opt: Result<vps::Model, _> = tx.query_row(
        "SELECT * FROM vps WHERE id = ?",
        params![vps_id],
        row_to_vps_model,
    );

    if vps_model_opt.is_err() {
        tx.commit()?;
        return Ok(false);
    }
    let vps_data = vps_model_opt.unwrap();

    if vps_data.next_traffic_reset_at.is_none() || vps_data.next_traffic_reset_at.unwrap() > now {
        tx.commit()?;
        return Ok(false);
    }

    let last_reset_time = vps_data.next_traffic_reset_at.unwrap();

    let new_next_reset_at = calculate_next_reset_date(
        vps_id,
        last_reset_time,
        vps_data.traffic_reset_config_type.as_deref(),
        vps_data.traffic_reset_config_value.as_deref(),
    );

    tx.execute(
        "UPDATE vps SET traffic_current_cycle_rx_bytes = ?, traffic_current_cycle_tx_bytes = ?, traffic_last_reset_at = ?, next_traffic_reset_at = ?, updated_at = ? WHERE id = ?",
        params![
            0, // reset rx
            0, // reset tx
            last_reset_time,
            new_next_reset_at,
            Utc::now(),
            vps_id,
        ],
    )?;

    tx.commit()?;
    Ok(true)
}

fn calculate_next_reset_date(
    vps_id: i32,
    last_reset_time: DateTime<Utc>,
    config_type: Option<&str>,
    config_value: Option<&str>,
) -> Option<DateTime<Utc>> {
    match (config_type, config_value) {
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

                let first_day_of_next_month =
                    NaiveDate::from_ymd_opt(next_month_year, next_month_month, 1).unwrap();
                let days_in_next_month = if next_month_month == 12 {
                    NaiveDate::from_ymd_opt(next_month_year + 1, 1, 1).unwrap()
                } else {
                    NaiveDate::from_ymd_opt(next_month_year, next_month_month + 1, 1).unwrap()
                }
                .signed_duration_since(first_day_of_next_month)
                .num_days() as u32;

                let actual_day = std::cmp::min(day, days_in_next_month);

                if let Some(naive_date_next) =
                    NaiveDate::from_ymd_opt(next_month_year, next_month_month, actual_day)
                {
                    let naive_datetime_next = naive_date_next.and_hms_opt(0, 0, 0).unwrap()
                        + Duration::seconds(time_offset_seconds);
                    Some(DateTime::<Utc>::from_naive_utc_and_offset(
                        naive_datetime_next,
                        Utc,
                    ))
                } else {
                    error!(
                        vps_id = vps_id,
                        year = next_month_year,
                        month = next_month_month,
                        day = actual_day,
                        "Error calculating next reset date for monthly_day_of_month: Could not form NaiveDate"
                    );
                    None
                }
            } else {
                error!(
                    vps_id = vps_id,
                    "Invalid traffic_reset_config_value (missing day) for monthly_day_of_month"
                );
                None
            }
        }
        (Some("fixed_days"), Some(value_str)) => {
            if let Ok(days) = value_str.parse::<i64>() {
                if days > 0 {
                    Some(last_reset_time + Duration::days(days))
                } else {
                    error!(
                        vps_id = vps_id,
                        "Invalid traffic_reset_config_value (days <= 0) for fixed_days"
                    );
                    None
                }
            } else {
                error!(
                    vps_id = vps_id,
                    "Invalid traffic_reset_config_value (not a number) for fixed_days"
                );
                None
            }
        }
        _ => {
            error!(
                vps_id = vps_id,
                "Missing or unknown traffic_reset_config_type or _value. Cannot calculate next reset."
            );
            None
        }
    }
}

/// Retrieves IDs of VPS that are due for a traffic reset check.
pub async fn get_vps_due_for_traffic_reset(
    pool: DuckDbPool,
) -> Result<Vec<i32>, AppError> {
    let conn = pool.get()?;
    let now = Utc::now();
    let mut stmt = conn.prepare(
        "SELECT id FROM vps WHERE traffic_reset_config_type IS NOT NULL AND traffic_reset_config_value IS NOT NULL AND next_traffic_reset_at IS NOT NULL AND next_traffic_reset_at <= ?",
    )?;
    let ids = stmt
        .query_map(params![now], |row| row.get(0))?
        .collect::<Result<Vec<i32>, _>>()?;
    Ok(ids)
}