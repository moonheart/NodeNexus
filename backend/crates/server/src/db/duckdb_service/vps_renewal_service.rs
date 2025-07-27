use crate::db::duckdb_service::DuckDbPool;
use crate::db::entities::vps_renewal_info;
use crate::web::error::AppError;
use chrono::{DateTime, Duration, Months, Timelike, Utc};
use duckdb::{params, OptionalExt, Transaction};
use tracing::{error, info, warn};

#[derive(Debug, Clone, Default)]
pub struct VpsRenewalDataInput {
    pub renewal_cycle: Option<String>,
    pub renewal_cycle_custom_days: Option<i32>,
    pub renewal_price: Option<f64>,
    pub renewal_currency: Option<String>,
    pub next_renewal_date: Option<DateTime<Utc>>,
    pub last_renewal_date: Option<DateTime<Utc>>,
    pub service_start_date: Option<DateTime<Utc>>,
    pub payment_method: Option<String>,
    pub auto_renew_enabled: Option<bool>,
    pub renewal_notes: Option<String>,
}

fn calculate_next_renewal_date_internal(
    reference_date: DateTime<Utc>,
    renewal_cycle: &str,
    custom_days: Option<i32>,
) -> Option<DateTime<Utc>> {
    let naive_ref_date = reference_date.date_naive();
    let naive_next_date = match renewal_cycle {
        "monthly" => naive_ref_date.checked_add_months(Months::new(1)),
        "quarterly" => naive_ref_date.checked_add_months(Months::new(3)),
        "semi_annually" => naive_ref_date.checked_add_months(Months::new(6)),
        "annually" => naive_ref_date.checked_add_months(Months::new(12)),
        "biennially" => naive_ref_date.checked_add_months(Months::new(24)),
        "triennially" => naive_ref_date.checked_add_months(Months::new(36)),
        "custom_days" => {
            custom_days.and_then(|days| naive_ref_date.checked_add_signed(Duration::days(days as i64)))
        }
        _ => None, // Unknown cycle
    };
    naive_next_date.map(|nd| {
        DateTime::<Utc>::from_naive_utc_and_offset(
            nd.and_hms_opt(
                reference_date.hour(),
                reference_date.minute(),
                reference_date.second(),
            )
            .unwrap_or_else(|| {
                nd.and_hms_opt(0, 0, 0)
                    .expect("Valid date should have valid time")
            }),
            Utc,
        )
    })
}

fn row_to_vps_renewal_info(row: &duckdb::Row) -> Result<vps_renewal_info::Model, duckdb::Error> {
    Ok(vps_renewal_info::Model {
        vps_id: row.get("vps_id")?,
        renewal_cycle: row.get("renewal_cycle")?,
        renewal_cycle_custom_days: row.get("renewal_cycle_custom_days")?,
        renewal_price: row.get("renewal_price")?,
        renewal_currency: row.get("renewal_currency")?,
        next_renewal_date: row.get("next_renewal_date")?,
        last_renewal_date: row.get("last_renewal_date")?,
        service_start_date: row.get("service_start_date")?,
        payment_method: row.get("payment_method")?,
        auto_renew_enabled: row.get("auto_renew_enabled")?,
        renewal_notes: row.get("renewal_notes")?,
        reminder_active: row.get("reminder_active")?,
        last_reminder_generated_at: row.get("last_reminder_generated_at")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

pub fn create_or_update_vps_renewal_info(
    tx: &Transaction,
    vps_id: i32,
    input: &VpsRenewalDataInput,
) -> Result<(), AppError> {
    let now = Utc::now();
    let mut calculated_next_renewal_date = input.next_renewal_date;

    if calculated_next_renewal_date.is_none() {
        if let Some(cycle) = &input.renewal_cycle {
            let reference_date_opt = input.last_renewal_date.or(input.service_start_date);
            if let Some(reference_date) = reference_date_opt {
                calculated_next_renewal_date = calculate_next_renewal_date_internal(
                    reference_date,
                    cycle,
                    input.renewal_cycle_custom_days,
                );
            }
        }
    }

    let existing_info_opt: Result<vps_renewal_info::Model, _> = tx.query_row(
        "SELECT * FROM vps_renewal_info WHERE vps_id = ?",
        params![vps_id],
        row_to_vps_renewal_info,
    );

    if let Ok(existing) = existing_info_opt {
        // --- UPDATE PATH ---
        let mut reminder_active = existing.reminder_active.unwrap_or(false);
        let mut last_reminder_generated_at = existing.last_reminder_generated_at;

        if existing.next_renewal_date != calculated_next_renewal_date {
            reminder_active = false;
            last_reminder_generated_at = None;
        }

        tx.execute(
            "UPDATE vps_renewal_info SET
                renewal_cycle = ?, renewal_cycle_custom_days = ?, renewal_price = ?, renewal_currency = ?,
                next_renewal_date = ?, last_renewal_date = ?, service_start_date = ?, payment_method = ?,
                auto_renew_enabled = ?, renewal_notes = ?, reminder_active = ?, last_reminder_generated_at = ?,
                updated_at = ?
            WHERE vps_id = ?",
            params![
                input.renewal_cycle, input.renewal_cycle_custom_days, input.renewal_price, input.renewal_currency,
                calculated_next_renewal_date, input.last_renewal_date, input.service_start_date, input.payment_method,
                input.auto_renew_enabled, input.renewal_notes, reminder_active, last_reminder_generated_at,
                now, vps_id
            ],
        )?;
    } else {
        // --- INSERT PATH ---
        tx.execute(
            "INSERT INTO vps_renewal_info (
                vps_id, renewal_cycle, renewal_cycle_custom_days, renewal_price, renewal_currency,
                next_renewal_date, last_renewal_date, service_start_date, payment_method,
                auto_renew_enabled, renewal_notes, reminder_active, last_reminder_generated_at,
                created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                vps_id, input.renewal_cycle, input.renewal_cycle_custom_days, input.renewal_price, input.renewal_currency,
                calculated_next_renewal_date, input.last_renewal_date, input.service_start_date, input.payment_method,
                input.auto_renew_enabled, input.renewal_notes, false, None::<DateTime<Utc>>,
                now, now
            ],
        )?;
    }

    Ok(())
}

pub async fn get_vps_renewal_info_by_vps_id(
    pool: &DuckDbPool,
    vps_id: i32,
) -> Result<Option<vps_renewal_info::Model>, AppError> {
    let conn = pool.get()?;
    let result = conn.query_row(
        "SELECT * FROM vps_renewal_info WHERE vps_id = ?",
        params![vps_id],
        row_to_vps_renewal_info,
    ).optional()?;
    Ok(result)
}

pub async fn dismiss_vps_renewal_reminder(
    pool: &DuckDbPool,
    vps_id: i32,
) -> Result<usize, AppError> {
    let conn = pool.get()?;
    let rows_affected = conn.execute(
        "UPDATE vps_renewal_info SET reminder_active = FALSE, updated_at = ? WHERE vps_id = ? AND reminder_active = TRUE",
        params![Utc::now(), vps_id],
    )?;
    Ok(rows_affected)
}

pub async fn check_and_generate_reminders(
    pool: DuckDbPool,
    reminder_threshold_days: i64,
) -> Result<u64, AppError> {
    let mut conn = pool.get()?;
    let now = Utc::now();
    let threshold_date = now + Duration::days(reminder_threshold_days);

    let mut stmt = conn.prepare(
        "SELECT * FROM vps_renewal_info WHERE next_renewal_date IS NOT NULL AND next_renewal_date <= ? AND (reminder_active IS NULL OR reminder_active = FALSE)"
    )?;
    let candidates: Vec<vps_renewal_info::Model> = stmt.query_map(params![threshold_date], row_to_vps_renewal_info)?.collect::<Result<_, _>>()?;

    if candidates.is_empty() {
        return Ok(0);
    }

    let tx = conn.transaction()?;
    let mut updated_count: u64 = 0;

    for vps_renewal_info_model in candidates {
        if let Some(nrd) = vps_renewal_info_model.next_renewal_date {
            if nrd < now {
                continue;
            }
        } else {
            continue;
        }

        let rows = tx.execute(
            "UPDATE vps_renewal_info SET reminder_active = TRUE, last_reminder_generated_at = ?, updated_at = ? WHERE vps_id = ?",
            params![now, now, vps_renewal_info_model.vps_id],
        )?;
        updated_count += rows as u64;
    }

    tx.commit()?;
    Ok(updated_count)
}

pub async fn process_all_automatic_renewals(pool: DuckDbPool) -> Result<u64, AppError> {
    let mut conn = pool.get()?;
    let now = Utc::now();
    let mut renewed_count: u64 = 0;

    let mut stmt = conn.prepare(
        "SELECT * FROM vps_renewal_info WHERE auto_renew_enabled = TRUE AND next_renewal_date IS NOT NULL AND next_renewal_date <= ?"
    )?;
    let candidates_to_renew: Vec<vps_renewal_info::Model> = stmt.query_map(params![now], row_to_vps_renewal_info)?.collect::<Result<_, _>>()?;

    if candidates_to_renew.is_empty() {
        return Ok(0);
    }

    for candidate_model in candidates_to_renew {
        let renewal_cycle = match candidate_model.renewal_cycle.as_deref() {
            Some(cycle) => cycle,
            None => {
                warn!(vps_id = candidate_model.vps_id, "Skipping auto-renewal: renewal_cycle is not set.");
                continue;
            }
        };

        let current_next_renewal_date = match candidate_model.next_renewal_date {
            Some(date) => date,
            None => {
                warn!(vps_id = candidate_model.vps_id, "Skipping auto-renewal: current_next_renewal_date is None unexpectedly.");
                continue;
            }
        };

        let new_last_renewal_date = current_next_renewal_date;
        let new_next_renewal_date = match calculate_next_renewal_date_internal(
            new_last_renewal_date,
            renewal_cycle,
            candidate_model.renewal_cycle_custom_days,
        ) {
            Some(date) => date,
            None => {
                warn!(vps_id = candidate_model.vps_id, "Skipping auto-renewal: Could not calculate new next_renewal_date.");
                continue;
            }
        };

        let tx = conn.transaction()?;
        let rows_affected = tx.execute(
            "UPDATE vps_renewal_info SET last_renewal_date = ?, next_renewal_date = ?, reminder_active = FALSE, last_reminder_generated_at = NULL, updated_at = ? WHERE vps_id = ? AND next_renewal_date = ?",
            params![new_last_renewal_date, new_next_renewal_date, now, candidate_model.vps_id, current_next_renewal_date],
        )?;

        if rows_affected > 0 {
            if let Err(e) = tx.commit() {
                error!(vps_id = candidate_model.vps_id, error = %e, "Error committing transaction for auto-renewal.");
            } else {
                renewed_count += 1;
                info!(vps_id = candidate_model.vps_id, "Successfully auto-renewed VPS.");
            }
        } else if let Err(e) = tx.rollback() {
            error!(vps_id = candidate_model.vps_id, error = %e, "Error rolling back transaction for auto-renewal.");
        }
    }
    Ok(renewed_count)
}