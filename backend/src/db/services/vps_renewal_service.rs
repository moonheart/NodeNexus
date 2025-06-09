use chrono::{DateTime, Datelike, Duration, Months, NaiveDate, Timelike, Utc};
use sqlx::{FromRow, PgPool, Result, Transaction, Postgres};
use crate::db::models::VpsRenewalInfo;

// --- Vps Renewal Service Functions ---

/// Input structure for creating or updating VPS renewal information.
#[derive(Debug, Clone, Default)]
pub struct VpsRenewalDataInput {
    pub renewal_cycle: Option<String>, // e.g., "monthly", "annually", "custom_days"
    pub renewal_cycle_custom_days: Option<i32>, // if renewal_cycle is "custom_days"
    pub renewal_price: Option<f64>,
    pub renewal_currency: Option<String>, // e.g., "USD", "CNY"
    pub next_renewal_date: Option<DateTime<Utc>>, // User can override or provide
    pub last_renewal_date: Option<DateTime<Utc>>,
    pub service_start_date: Option<DateTime<Utc>>,
    pub payment_method: Option<String>,
    pub auto_renew_enabled: Option<bool>,
    pub renewal_notes: Option<String>,
}

/// Calculates the next renewal date based on the cycle and a reference date.
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

    naive_next_date.map(|nd| DateTime::<Utc>::from_naive_utc_and_offset(nd.and_hms_opt(reference_date.hour(), reference_date.minute(), reference_date.second()).unwrap_or_else(|| nd.and_hms_opt(0,0,0).expect("Valid date should have valid time")), Utc))
}

/// Creates or updates the renewal information for a VPS.
/// This function is expected to be called within an existing database transaction.
pub async fn create_or_update_vps_renewal_info(
    tx: &mut Transaction<'_, Postgres>,
    vps_id: i32,
    input: &VpsRenewalDataInput,
) -> Result<()> {
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

    let existing_info: Option<VpsRenewalInfo> = sqlx::query_as!(
        VpsRenewalInfo,
        "SELECT * FROM vps_renewal_info WHERE vps_id = $1",
        vps_id
    )
    .fetch_optional(&mut **tx)
    .await?;

    let mut reminder_active = existing_info.as_ref().and_then(|ei| ei.reminder_active).unwrap_or(false);
    let mut last_reminder_generated_at = existing_info.as_ref().and_then(|ei| ei.last_reminder_generated_at);

    if let Some(existing) = &existing_info {
        if existing.next_renewal_date != calculated_next_renewal_date {
            reminder_active = false;
            last_reminder_generated_at = None;
        }
    } else if calculated_next_renewal_date.is_some() {
        reminder_active = false;
        last_reminder_generated_at = None;
    }

    sqlx::query!(
        r#"
        INSERT INTO vps_renewal_info (
            vps_id, renewal_cycle, renewal_cycle_custom_days, renewal_price, renewal_currency,
            next_renewal_date, last_renewal_date, service_start_date, payment_method,
            auto_renew_enabled, renewal_notes,
            reminder_active, last_reminder_generated_at,
            created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        ON CONFLICT (vps_id) DO UPDATE SET
            renewal_cycle = EXCLUDED.renewal_cycle,
            renewal_cycle_custom_days = EXCLUDED.renewal_cycle_custom_days,
            renewal_price = EXCLUDED.renewal_price,
            renewal_currency = EXCLUDED.renewal_currency,
            next_renewal_date = EXCLUDED.next_renewal_date,
            last_renewal_date = EXCLUDED.last_renewal_date,
            service_start_date = EXCLUDED.service_start_date,
            payment_method = EXCLUDED.payment_method,
            auto_renew_enabled = EXCLUDED.auto_renew_enabled,
            renewal_notes = EXCLUDED.renewal_notes,
            reminder_active = EXCLUDED.reminder_active,
            last_reminder_generated_at = EXCLUDED.last_reminder_generated_at,
            updated_at = EXCLUDED.updated_at
        "#,
        vps_id,
        input.renewal_cycle,
        input.renewal_cycle_custom_days,
        input.renewal_price,
        input.renewal_currency,
        calculated_next_renewal_date,
        input.last_renewal_date,
        input.service_start_date,
        input.payment_method,
        input.auto_renew_enabled,
        input.renewal_notes,
        reminder_active,
        last_reminder_generated_at,
        now,
        now
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// Retrieves VPS renewal information by VPS ID.
pub async fn get_vps_renewal_info_by_vps_id(pool: &PgPool, vps_id: i32) -> Result<Option<VpsRenewalInfo>> {
    sqlx::query_as!(
        VpsRenewalInfo,
        "SELECT * FROM vps_renewal_info WHERE vps_id = $1",
        vps_id
    )
    .fetch_optional(pool)
    .await
}

// --- Renewal Reminder Service Functions ---

/// Dismisses the active renewal reminder for a specific VPS.
/// Sets `reminder_active` to false and updates `updated_at`.
pub async fn dismiss_vps_renewal_reminder(pool: &PgPool, vps_id: i32) -> Result<u64> {
    let now = Utc::now();
    let rows_affected = sqlx::query!(
        r#"
        UPDATE vps_renewal_info
        SET reminder_active = false, updated_at = $1
        WHERE vps_id = $2 AND reminder_active = true
        "#,
        now,
        vps_id
    )
    .execute(pool)
    .await?
    .rows_affected();

    Ok(rows_affected)
}

/// Checks all VPS renewal information and activates reminders if they are due.
/// A reminder is considered due if the `next_renewal_date` is within `reminder_threshold_days`
/// from the current date, and the reminder is not already active.
///
/// Returns the number of reminders activated.
pub async fn check_and_generate_reminders(
    pool: &PgPool,
    reminder_threshold_days: i64,
) -> Result<u64> {
    let now = Utc::now();
    let threshold_date = now + Duration::days(reminder_threshold_days);
    let mut updated_count: u64 = 0;

    let candidates = sqlx::query_as!(
        VpsRenewalInfo,
        r#"
        SELECT *
        FROM vps_renewal_info
        WHERE next_renewal_date IS NOT NULL
          AND next_renewal_date <= $1
          AND (reminder_active IS NULL OR reminder_active = false)
        "#,
        threshold_date
    )
    .fetch_all(pool)
    .await?;

    if candidates.is_empty() {
        return Ok(0);
    }

    let mut tx = pool.begin().await?;

    for vps_renewal_info in candidates {
        if let Some(nrd) = vps_renewal_info.next_renewal_date {
            if nrd < now { 
                continue;
            }
        } else { 
            continue;
        }

        let rows_affected = sqlx::query!(
            r#"
            UPDATE vps_renewal_info
            SET reminder_active = true, last_reminder_generated_at = $1, updated_at = $1
            WHERE vps_id = $2
              AND (reminder_active IS NULL OR reminder_active = false)
              AND next_renewal_date IS NOT NULL AND next_renewal_date <= $3
            "#,
            now,
            vps_renewal_info.vps_id,
            threshold_date
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();
        updated_count += rows_affected;
    }

    tx.commit().await?;
    Ok(updated_count)
}

/// Processes automatic renewals for all VPS that are due and have auto-renew enabled.
/// Returns the number of VPS successfully auto-renewed.
pub async fn process_all_automatic_renewals(pool: &PgPool) -> Result<u64> {
    let now = Utc::now();
    let mut renewed_count: u64 = 0;

    #[derive(FromRow, Debug)]
    struct VpsToRenew {
        vps_id: i32,
        renewal_cycle: Option<String>,
        renewal_cycle_custom_days: Option<i32>,
        current_next_renewal_date: DateTime<Utc>,
    }

    let candidates_to_renew = sqlx::query_as!(
        VpsToRenew,
        r#"
        SELECT 
            vps_id, 
            renewal_cycle, 
            renewal_cycle_custom_days,
            next_renewal_date as "current_next_renewal_date!"
        FROM vps_renewal_info
        WHERE auto_renew_enabled = true
          AND next_renewal_date IS NOT NULL
          AND next_renewal_date <= $1
        "#,
        now
    )
    .fetch_all(pool)
    .await?;

    if candidates_to_renew.is_empty() {
        return Ok(0);
    }

    for candidate in candidates_to_renew {
        if candidate.renewal_cycle.is_none() {
            eprintln!(
                "Skipping auto-renewal for VPS ID {}: renewal_cycle is not set.",
                candidate.vps_id
            );
            continue;
        }

        let new_last_renewal_date = candidate.current_next_renewal_date;
        let new_next_renewal_date_opt = calculate_next_renewal_date_internal(
            new_last_renewal_date,
            candidate.renewal_cycle.as_ref().unwrap(),
            candidate.renewal_cycle_custom_days,
        );

        if new_next_renewal_date_opt.is_none() {
            eprintln!(
                "Skipping auto-renewal for VPS ID {}: Could not calculate new next_renewal_date.",
                candidate.vps_id
            );
            continue;
        }
        let new_next_renewal_date = new_next_renewal_date_opt.unwrap();
        
        let mut tx = pool.begin().await?;

        let update_result = sqlx::query!(
            r#"
            UPDATE vps_renewal_info
            SET 
                last_renewal_date = $1,
                next_renewal_date = $2,
                reminder_active = false,
                last_reminder_generated_at = NULL,
                updated_at = $3
            WHERE vps_id = $4
              AND auto_renew_enabled = true 
              AND next_renewal_date = $5 
            "#,
            new_last_renewal_date,
            new_next_renewal_date,
            now, 
            candidate.vps_id,
            candidate.current_next_renewal_date 
        )
        .execute(&mut *tx)
        .await;

        match update_result {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    match tx.commit().await {
                        Ok(_) => {
                            renewed_count += 1;
                            println!("Successfully auto-renewed VPS ID: {}", candidate.vps_id);
                        }
                        Err(commit_err) => {
                            eprintln!("Error committing transaction for auto-renewal of VPS ID {}: {}", candidate.vps_id, commit_err);
                        }
                    }
                } else {
                    if let Err(rollback_err) = tx.rollback().await {
                         eprintln!("Error rolling back transaction for auto-renewal of VPS ID {} (no rows affected): {}", candidate.vps_id, rollback_err);
                    }
                    println!("Auto-renewal for VPS ID {} did not update any rows (possibly changed or optimistic lock failed).", candidate.vps_id);
                }
            }
            Err(e) => {
                if let Err(rollback_err) = tx.rollback().await {
                    eprintln!("Error rolling back transaction for auto-renewal of VPS ID {} (query error): {}", candidate.vps_id, rollback_err);
                }
                eprintln!("Error during SQL execution for auto-renewal of VPS ID {}: {}", candidate.vps_id, e);
            }
        }
    }

    Ok(renewed_count)
}