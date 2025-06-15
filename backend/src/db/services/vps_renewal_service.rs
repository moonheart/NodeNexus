use chrono::{DateTime, Duration, Months, Timelike, Utc}; // Removed Datelike, NaiveDate
use sea_orm::{
    prelude::Expr, ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel, NotSet, QueryFilter, Set, TransactionTrait // Removed ActiveValue
};
use crate::db::entities::vps_renewal_info; // Changed
use tracing::{info, error, warn};

// --- Vps Renewal Service Functions ---

/// Input structure for creating or updating VPS renewal information.
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
    txn: &sea_orm::DatabaseTransaction, // Changed
    vps_id: i32,
    input: &VpsRenewalDataInput,
) -> Result<(), DbErr> { // Changed
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

    let existing_info_model = vps_renewal_info::Entity::find_by_id(vps_id)
        .one(txn)
        .await?;

    let mut reminder_active = existing_info_model.as_ref().and_then(|ei| ei.reminder_active).unwrap_or(false);
    let mut last_reminder_generated_at = existing_info_model.as_ref().and_then(|ei| ei.last_reminder_generated_at);

    if let Some(existing) = &existing_info_model {
        if existing.next_renewal_date != calculated_next_renewal_date {
            reminder_active = false;
            last_reminder_generated_at = None;
        }
    } else if calculated_next_renewal_date.is_some() {
        // New entry and next_renewal_date is calculated, so reset reminder state
        reminder_active = false;
        last_reminder_generated_at = None;
    }
    
    let active_model = vps_renewal_info::ActiveModel {
        vps_id: Set(vps_id), // PK, always set
        renewal_cycle: Set(input.renewal_cycle.clone()),
        renewal_cycle_custom_days: Set(input.renewal_cycle_custom_days),
        renewal_price: Set(input.renewal_price),
        renewal_currency: Set(input.renewal_currency.clone()),
        next_renewal_date: Set(calculated_next_renewal_date),
        last_renewal_date: Set(input.last_renewal_date),
        service_start_date: Set(input.service_start_date),
        payment_method: Set(input.payment_method.clone()),
        auto_renew_enabled: Set(input.auto_renew_enabled),
        renewal_notes: Set(input.renewal_notes.clone()),
        reminder_active: Set(Some(reminder_active)),
        last_reminder_generated_at: Set(last_reminder_generated_at),
        created_at: if existing_info_model.is_some() { NotSet } else { Set(now) },
        updated_at: Set(now),
    };

    // Upsert behavior: insert or update on conflict
    // SeaORM's save() method handles this:
    // If the primary key is set and exists, it updates. Otherwise, it inserts.
    // Since vps_id is the PK and we are setting it, this will effectively be an upsert.
    active_model.save(txn).await?;

    Ok(())
}

/// Retrieves VPS renewal information by VPS ID.
pub async fn get_vps_renewal_info_by_vps_id(
    db: &DatabaseConnection, // Changed
    vps_id: i32,
) -> Result<Option<vps_renewal_info::Model>, DbErr> { // Changed
    vps_renewal_info::Entity::find_by_id(vps_id).one(db).await
}

// --- Renewal Reminder Service Functions ---

/// Dismisses the active renewal reminder for a specific VPS.
/// Sets `reminder_active` to false and updates `updated_at`.
pub async fn dismiss_vps_renewal_reminder(
    db: &DatabaseConnection, // Changed
    vps_id: i32,
) -> Result<u64, DbErr> { // Changed
    let now = Utc::now();
    let result = vps_renewal_info::Entity::update_many()
        .col_expr(vps_renewal_info::Column::ReminderActive, Expr::value(sea_orm::Value::Bool(Some(false))))
        .col_expr(vps_renewal_info::Column::UpdatedAt, Expr::value(sea_orm::Value::ChronoDateTimeUtc(Some(Box::new(now)))))
        .filter(vps_renewal_info::Column::VpsId.eq(vps_id))
        .filter(vps_renewal_info::Column::ReminderActive.eq(true))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

/// Checks all VPS renewal information and activates reminders if they are due.
/// A reminder is considered due if the `next_renewal_date` is within `reminder_threshold_days`
/// from the current date, and the reminder is not already active.
///
/// Returns the number of reminders activated.
pub async fn check_and_generate_reminders(
    db: &DatabaseConnection, // Changed
    reminder_threshold_days: i64,
) -> Result<u64, DbErr> { // Changed
    let now = Utc::now();
    let threshold_date = now + Duration::days(reminder_threshold_days);
    
    let candidates = vps_renewal_info::Entity::find()
        .filter(vps_renewal_info::Column::NextRenewalDate.is_not_null())
        .filter(vps_renewal_info::Column::NextRenewalDate.lte(threshold_date))
        .filter(
            sea_orm::Condition::any()
                .add(vps_renewal_info::Column::ReminderActive.is_null())
                .add(vps_renewal_info::Column::ReminderActive.eq(false)),
        )
        .all(db)
        .await?;

    if candidates.is_empty() {
        return Ok(0);
    }

    let mut updated_count: u64 = 0;
    let txn = db.begin().await?;

    for vps_renewal_info_model in candidates {
        if let Some(nrd) = vps_renewal_info_model.next_renewal_date {
            if nrd < now {
                continue; // Skip if already past due for this check
            }
        } else {
            continue; // Should not happen due to filter, but good practice
        }

        let mut active_model = vps_renewal_info_model.into_active_model();
        active_model.reminder_active = Set(Some(true));
        active_model.last_reminder_generated_at = Set(Some(now));
        active_model.updated_at = Set(now);
        
        match active_model.update(&txn).await {
            Ok(_) => updated_count += 1,
            Err(e) => {
                txn.rollback().await?; // Rollback on any error
                return Err(e);
            }
        }
    }

    txn.commit().await?;
    Ok(updated_count)
}


/// Processes automatic renewals for all VPS that are due and have auto-renew enabled.
/// Returns the number of VPS successfully auto-renewed.
pub async fn process_all_automatic_renewals(
    db: &DatabaseConnection, // Changed
) -> Result<u64, DbErr> { // Changed
    let now = Utc::now();
    let mut renewed_count: u64 = 0;

    // SeaORM doesn't directly support FromRow for arbitrary structs in the same way as sqlx.
    // We need to select specific columns and map them.
    let candidates_to_renew = vps_renewal_info::Entity::find()
        .filter(vps_renewal_info::Column::AutoRenewEnabled.eq(true))
        .filter(vps_renewal_info::Column::NextRenewalDate.is_not_null())
        .filter(vps_renewal_info::Column::NextRenewalDate.lte(now))
        .all(db) // Fetch full models first
        .await?;

    if candidates_to_renew.is_empty() {
        return Ok(0);
    }

    for candidate_model in candidates_to_renew {
        if candidate_model.renewal_cycle.is_none() {
            warn!(
                vps_id = candidate_model.vps_id,
                "Skipping auto-renewal: renewal_cycle is not set."
            );
            continue;
        }
        
        // Ensure current_next_renewal_date is not None, which it shouldn't be due to filters
        let current_next_renewal_date = match candidate_model.next_renewal_date {
            Some(date) => date,
            None => {
                 warn!(vps_id = candidate_model.vps_id, "Skipping auto-renewal: current_next_renewal_date is None unexpectedly.");
                 continue;
            }
        };

        let new_last_renewal_date = current_next_renewal_date;
        let new_next_renewal_date_opt = calculate_next_renewal_date_internal(
            new_last_renewal_date,
            candidate_model.renewal_cycle.as_ref().unwrap(), // Safe due to check above
            candidate_model.renewal_cycle_custom_days,
        );

        if new_next_renewal_date_opt.is_none() {
            warn!(
                vps_id = candidate_model.vps_id,
                "Skipping auto-renewal: Could not calculate new next_renewal_date."
            );
            continue;
        }
        let new_next_renewal_date = new_next_renewal_date_opt.unwrap();

        let txn = db.begin().await?;
        
        let mut active_model = candidate_model.clone().into_active_model(); // Clone before moving into active model
        active_model.last_renewal_date = Set(Some(new_last_renewal_date));
        active_model.next_renewal_date = Set(Some(new_next_renewal_date));
        active_model.reminder_active = Set(Some(false));
        active_model.last_reminder_generated_at = Set(None);
        active_model.updated_at = Set(now);

        // Add a condition to the update to ensure we are updating the expected row (optimistic lock)
        // This is implicitly handled if we fetch the model first and then update its active model,
        // but for an update_many style, we'd need a filter.
        // Here, since we fetched the model, then convert to active model and save,
        // SeaORM's save will perform an update based on the primary key.
        // We need to ensure the `next_renewal_date` hasn't changed since we fetched it.
        // This is tricky with `save`. A direct `update_many` with a filter on `next_renewal_date`
        // would be more robust for optimistic locking if not fetching the full model first.
        // For now, we proceed with the fetched model's update.

        match active_model.update(&txn).await {
            Ok(updated_model) => {
                // Check if the update actually happened (e.g. if the model was found and updated)
                // `update` returns the updated model. If it was successful, we assume rows_affected > 0.
                // A more robust check might involve comparing fields or if SeaORM provides rows_affected for `save`.
                // For now, success of `update` implies it worked.
                match txn.commit().await {
                    Ok(_) => {
                        renewed_count += 1;
                        info!(vps_id = updated_model.vps_id, "Successfully auto-renewed VPS.");
                    }
                    Err(commit_err) => {
                        error!(vps_id = candidate_model.vps_id, error = %commit_err, "Error committing transaction for auto-renewal.");
                    }
                }
            }
            Err(e) => {
                if let Err(rollback_err) = txn.rollback().await {
                    error!(vps_id = candidate_model.vps_id, error = %rollback_err, "Error rolling back transaction for auto-renewal (query error).");
                }
                error!(vps_id = candidate_model.vps_id, error = %e, "Error during SQL execution for auto-renewal.");
            }
        }
    }
    Ok(renewed_count)
}