use crate::{
    db::{
        entities::{alert_rule, performance_metric, vps}, // Changed to use entities
        services::{AlertService, vps_service},
    },
    notifications::service::NotificationService,
};
use chrono::{Duration as ChronoDuration, Utc}; // Added ChronoDuration
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
}; // Added SeaORM imports, removed IntoSimpleExpr
use std::sync::Arc;
use tokio::time::{Duration as TokioDuration, interval}; // Renamed to avoid conflict
use tracing::{debug, error, info, warn};

#[derive(Debug, thiserror::Error)]
pub enum EvaluationError {
    #[error("Database query error: {0}")]
    DatabaseError(#[from] DbErr), // Changed from sqlx::Error
    #[error("Notification error: {0}")]
    NotificationError(#[from] crate::notifications::service::NotificationError),
    #[error("Failed to get VPS name for ID {0}")]
    VpsNameNotFound(i32),
    // Add other specific error types as needed
}

pub struct EvaluationService {
    pool: Arc<DatabaseConnection>, // Changed PgPool to DatabaseConnection
    notification_service: Arc<NotificationService>,
    alert_service: Arc<AlertService>,
}

impl EvaluationService {
    pub fn new(
        pool: Arc<DatabaseConnection>, // Changed PgPool to DatabaseConnection
        notification_service: Arc<NotificationService>,
        alert_service: Arc<AlertService>,
    ) -> Self {
        Self {
            pool,
            notification_service,
            alert_service,
        }
    }

    // Placeholder for the main evaluation loop
    pub async fn start_periodic_evaluation(self: Arc<Self>, period_seconds: u64) {
        info!(
            interval_seconds = period_seconds,
            "Alert evaluation service started."
        );
        let mut interval = interval(TokioDuration::from_secs(period_seconds)); // Use TokioDuration
        loop {
            interval.tick().await;
            debug!("Running alert evaluation cycle...");
            if let Err(e) = self.run_evaluation_cycle().await {
                error!(error = %e, "Error during alert evaluation cycle.");
            }
        }
    }

    async fn run_evaluation_cycle(&self) -> Result<(), EvaluationError> {
        debug!("Fetching active alert rules...");
        let active_rules = self
            .alert_service
            .get_all_active_rules_for_evaluation()
            .await?; // Assuming this method exists or will be created

        info!(count = active_rules.len(), "Active rules to evaluate.");

        for rule in active_rules {
            // TODO: Implement cooldown logic here later if needed (e.g., check rule.last_triggered_at)

            match self.evaluate_rule(&rule).await {
                // rule is alert_rule::Model
                Ok(Some(notification_message)) => {
                    info!(rule_name = %rule.name, rule_id = rule.id, "Alert rule triggered. Sending notifications.");
                    // We need a method in NotificationService to send notifications for a given rule_id / user_id
                    // This method would look up associated channels and send the message.
                    // For now, let's assume such a method exists or we'll add it.
                    // The user_id is on the rule object.
                    match self
                        .notification_service
                        .send_notifications_for_alert_rule(
                            rule.id,
                            rule.user_id,
                            &notification_message,
                        )
                        .await
                    {
                        Ok(_) => {
                            info!(
                                rule_id = rule.id,
                                "Successfully sent notifications for alert rule."
                            );
                            // Update last_triggered_at timestamp
                            if let Err(e_update) = self
                                .alert_service
                                .update_alert_rule_last_triggered(rule.id, rule.user_id)
                                .await
                            {
                                error!(rule_id = rule.id, error = %e_update, "Failed to update last_triggered_at for rule.");
                            }
                        }
                        Err(e) => {
                            error!(rule_id = rule.id, error = %e, "Failed to send notifications for alert rule.")
                        }
                    }
                }
                Ok(None) => {
                    // Rule not triggered, do nothing or log verbosely if needed
                    // debug!("Rule '{}' (ID: {}) not triggered.", rule.name, rule.id);
                }
                Err(e) => {
                    error!(rule_name = %rule.name, rule_id = rule.id, error = %e, "Error evaluating rule.");
                }
            }
        }
        Ok(())
    }

    async fn evaluate_rule(
        &self,
        rule: &alert_rule::Model,
    ) -> Result<Option<String>, EvaluationError> {
        if let Some(specific_vps_id) = rule.vps_id {
            // Rule is for a specific VPS
            let vps_name = vps::Entity::find_by_id(specific_vps_id)
                .select_only()
                .column(vps::Column::Name)
                .into_tuple::<Option<String>>()
                .one(&*self.pool)
                .await?
                .flatten()
                .unwrap_or_else(|| format!("VPS_ID_{specific_vps_id}"));

            self.evaluate_rule_for_single_vps(rule, specific_vps_id, &vps_name)
                .await
        } else {
            // Rule is global, apply to all user's VPS
            debug!(rule_name = %rule.name, rule_id = rule.id, user_id = rule.user_id, "Evaluating global rule.");
            // Assuming get_all_vps_for_user now returns Vec<vps::Model>
            let user_vps_list =
                crate::db::services::vps_service::get_all_vps_for_user(&self.pool, rule.user_id)
                    .await?;

            if user_vps_list.is_empty() {
                warn!(user_id = rule.user_id, rule_name = %rule.name, "No VPS found for user to evaluate global rule.");
                return Ok(None);
            }

            for vps_instance in user_vps_list {
                match self
                    .evaluate_rule_for_single_vps(rule, vps_instance.id, &vps_instance.name)
                    .await
                {
                    Ok(Some(message)) => {
                        // If any VPS triggers the global rule, return the message for that first trigger.
                        // The cooldown for the global rule itself will be handled by run_evaluation_cycle.
                        return Ok(Some(message));
                    }
                    Ok(None) => {
                        // This specific VPS did not trigger the rule, continue to the next.
                    }
                    Err(e) => {
                        // Log error for this specific VPS evaluation but continue trying others for a global rule.
                        error!(rule_name = %rule.name, vps_id = vps_instance.id, error = %e, "Error evaluating global rule for VPS.");
                    }
                }
            }
            // If no VPS triggered the global rule
            Ok(None)
        }
    }

    async fn evaluate_rule_for_single_vps(
        &self,
        rule: &alert_rule::Model, // Changed AlertRuleFromDb to alert_rule::Model
        vps_id: i32,
        vps_name: &str,
    ) -> Result<Option<String>, EvaluationError> {
        let now = Utc::now();

        // Cooldown check for the rule itself (relevant if called directly or for specific VPS rules)
        // For global rules, this check is effectively done before iterating user's VPS if we consider the rule's last_triggered_at
        if rule.vps_id.is_some() || rule.vps_id.is_none() {
            // Apply cooldown check universally for now
            if let Some(last_triggered) = rule.last_triggered_at {
                let cooldown_period = ChronoDuration::seconds(rule.cooldown_seconds as i64); // Use rule.cooldown_seconds
                if now < last_triggered + cooldown_period {
                    debug!(
                        rule_name = %rule.name,
                        rule_id = rule.id,
                        vps_name = %vps_name,
                        vps_id = vps_id,
                        cooldown_seconds = rule.cooldown_seconds,
                        last_triggered = %last_triggered,
                        "Rule is in cooldown."
                    );
                    return Ok(None);
                }
            }
        }

        let start_time = now - ChronoDuration::seconds(rule.duration_seconds as i64);

        let metrics: Vec<performance_metric::Model> = performance_metric::Entity::find()
            .filter(performance_metric::Column::VpsId.eq(vps_id))
            .filter(performance_metric::Column::Time.gte(start_time))
            .filter(performance_metric::Column::Time.lte(now))
            .order_by_asc(performance_metric::Column::Time)
            .all(&*self.pool)
            .await?;

        if metrics.is_empty() {
            return Ok(None);
        }

        // If duration_seconds is 0, we only need one metric point to trigger.
        // If duration_seconds > 0, we need at least two points to confirm persistence over the duration.
        // However, the current logic iterates all points within the duration.
        // If rule.duration_seconds > 0 and metrics.len() < 2, it might be too few points to confirm persistence.
        // For simplicity, let's keep the original logic: if any point in the duration window fails, the rule doesn't trigger.
        // If all points match, it triggers. If duration_seconds is 0, it checks the latest point (or all points if multiple at 'now').
        // The original check `metrics.len() < 2 && rule.duration_seconds > 0` seems to imply that for a duration,
        // you need at least two data points to confirm it held true over that duration.
        // Let's refine this: if duration_seconds > 0, we expect metrics to cover that duration.
        // If only one metric point exists in a non-zero duration, it's hard to say it persisted.
        // For now, we will keep the logic that all points in the window must match.
        // If duration_seconds is 0, it means check the current state (latest metric).
        // The query already filters by time window.

        let mut all_match = true;
        let mut last_metric_value_str = "N/A".to_string();

        for metric_point in &metrics {
            let current_value: f64;
            match rule.metric_type.as_str() {
                "cpu_usage_percent" => {
                    current_value = metric_point.cpu_usage_percent;
                }
                "memory_usage_percent" => {
                    if metric_point.memory_total_bytes == 0 {
                        all_match = false;
                        break;
                    }
                    current_value = (metric_point.memory_usage_bytes as f64
                        / metric_point.memory_total_bytes as f64)
                        * 100.0;
                }
                "traffic_usage_percent" => {
                    // For traffic, we need the current cycle usage from the VPS record, not the metric point.
                    // We should fetch the VPS data once per rule evaluation for a specific VPS, not per metric point.
                    // This logic needs to be outside the metric iteration loop.
                    // For now, we'll break and handle this metric type separately or adjust the loop structure.
                    // Let's adjust the structure to handle traffic separately as it doesn't rely on a series of metric points.
                    all_match = false; // Indicate that this metric type is handled outside this loop
                    break;
                }
                _ => {
                    all_match = false; // Unsupported metric type, stop evaluation for this rule/vps
                    break;
                }
            }
            if !rule.metric_type.eq("traffic_usage_percent") {
                last_metric_value_str = format!("{current_value:.2}");
            }

            // Comparison logic remains the same for metric points
            if !rule.metric_type.eq("traffic_usage_percent") {
                let condition_met = match rule.comparison_operator.as_str() {
                    ">" => current_value > rule.threshold,
                    "<" => current_value < rule.threshold,
                    ">=" => current_value >= rule.threshold,
                    "<=" => current_value <= rule.threshold,
                    "=" | "==" => (current_value - rule.threshold).abs() < f64::EPSILON,
                    "!=" => (current_value - rule.threshold).abs() > f64::EPSILON,
                    _ => {
                        all_match = false; // Unsupported operator
                        break;
                    }
                };

                if !condition_met {
                    all_match = false;
                    break;
                }
            }
        } // End of metric iteration loop

        // Handle traffic_usage_percent separately as it uses current cycle data, not historical points
        if rule.metric_type.eq("traffic_usage_percent") {
            let vps_model_option = vps_service::get_vps_by_id(&self.pool, vps_id).await?; // Assuming this returns Option<vps::Model>
            if let Some(vps_model) = vps_model_option {
                // Changed vps to vps_model
                if let Some(limit_bytes) = vps_model.traffic_limit_bytes {
                    if limit_bytes > 0 {
                        let current_rx = vps_model.traffic_current_cycle_rx_bytes.unwrap_or(0);
                        let current_tx = vps_model.traffic_current_cycle_tx_bytes.unwrap_or(0);
                        let total_used = match vps_model.traffic_billing_rule.as_deref() {
                            Some("sum_in_out") => current_rx + current_tx,
                            Some("out_only") => current_tx,
                            Some("max_in_out") => std::cmp::max(current_rx, current_tx),
                            _ => {
                                warn!(
                                    vps_id = vps_id,
                                    "Unsupported or missing traffic_billing_rule."
                                );
                                return Ok(None); // Cannot evaluate without a valid rule
                            }
                        };

                        let usage_percent = (total_used as f64 / limit_bytes as f64) * 100.0;
                        last_metric_value_str = format!("{usage_percent:.2}"); // Update for message

                        let condition_met = match rule.comparison_operator.as_str() {
                            ">" => usage_percent > rule.threshold,
                            "<" => usage_percent < rule.threshold,
                            ">=" => usage_percent >= rule.threshold,
                            "<=" => usage_percent <= rule.threshold,
                            "=" | "==" => (usage_percent - rule.threshold).abs() < f64::EPSILON,
                            "!=" => (usage_percent - rule.threshold).abs() > f64::EPSILON,
                            _ => {
                                warn!(
                                    rule_id = rule.id,
                                    "Unsupported comparison_operator for traffic_usage_percent rule."
                                );
                                return Ok(None); // Unsupported operator
                            }
                        };

                        if condition_met {
                            all_match = true; // Condition met for traffic
                        } else {
                            all_match = false; // Condition not met for traffic
                        }
                    } else {
                        // Limit is 0 or not set, cannot calculate percentage meaningfully for threshold rules
                        debug!(
                            vps_id = vps_id,
                            rule_id = rule.id,
                            "Traffic limit is 0 or not set. Cannot evaluate traffic_usage_percent rule."
                        );
                        return Ok(None);
                    }
                } else {
                    // Limit is not set, cannot calculate percentage
                    debug!(
                        vps_id = vps_id,
                        rule_id = rule.id,
                        "Traffic limit is not configured. Cannot evaluate traffic_usage_percent rule."
                    );
                    return Ok(None);
                }
            } else {
                error!(
                    vps_id = vps_id,
                    "VPS not found during traffic alert evaluation."
                );
                return Err(EvaluationError::VpsNameNotFound(vps_id)); // VPS not found
            }
        }

        if all_match {
            let duration_suffix = if rule.metric_type.eq("traffic_usage_percent") {
                String::new()
            } else {
                format!(" for {} seconds", rule.duration_seconds)
            };
            let message = format!(
                "ALERT! Rule '{}' triggered for VPS '{}' (ID: {}): Metric {} {} {} (current: {}){}.",
                rule.name,
                vps_name,
                vps_id, // Added vps_id to message for clarity with global rules
                rule.metric_type,
                rule.comparison_operator,
                rule.threshold,
                last_metric_value_str,
                duration_suffix
            );
            return Ok(Some(message));
        }
        Ok(None)
    }
}
