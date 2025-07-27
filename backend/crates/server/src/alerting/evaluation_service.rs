use crate::{
    db::{
        duckdb_service::{alert_evaluation_service, alert_service, vps_service, DuckDbPool},
        entities::{alert_rule, performance_metric},
    },
    notifications::service::NotificationService,
};
use chrono::{Duration as ChronoDuration, Utc};
use std::sync::Arc;
use tokio::time::{interval, Duration as TokioDuration};
use tracing::{debug, error, info, warn};

#[derive(Debug, thiserror::Error)]
pub enum EvaluationError {
    #[error("Database query error: {0}")]
    DatabaseError(#[from] alert_evaluation_service::AlertEvaluationDbError),
    #[error("Notification error: {0}")]
    NotificationError(#[from] crate::notifications::service::NotificationError),
    #[error("Failed to get VPS name for ID {0}")]
    VpsNameNotFound(i32),
    #[error("Application error: {0}")]
    AppError(#[from] crate::web::error::AppError),
}

pub struct EvaluationService {
    pool: DuckDbPool,
    notification_service: Arc<NotificationService>,
}

impl EvaluationService {
    pub fn new(pool: DuckDbPool, notification_service: Arc<NotificationService>) -> Self {
        Self {
            pool,
            notification_service,
        }
    }

    pub async fn start_periodic_evaluation(self: Arc<Self>, period_seconds: u64) {
        info!(
            interval_seconds = period_seconds,
            "Alert evaluation service started."
        );
        let mut interval = interval(TokioDuration::from_secs(period_seconds));
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
        let active_rules =
            alert_service::get_all_active_rules_for_evaluation(self.pool.clone()).await?;

        info!(count = active_rules.len(), "Active rules to evaluate.");

        for rule in active_rules {
            match self.evaluate_rule(&rule).await {
                Ok(Some(notification_message)) => {
                    info!(rule_name = %rule.name, rule_id = rule.id, "Alert rule triggered. Sending notifications.");
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
                            if let Err(e_update) =
                                alert_service::update_alert_rule_last_triggered(
                                    self.pool.clone(),
                                    rule.id,
                                    rule.user_id,
                                )
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
                Ok(None) => {}
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
            let vps_name =
                vps_service::get_vps_by_id(self.pool.clone(), specific_vps_id)
                    .await?
                    .map(|v| v.name)
                    .unwrap_or_else(|| format!("VPS_ID_{specific_vps_id}"));

            self.evaluate_rule_for_single_vps(rule, specific_vps_id, &vps_name)
                .await
        } else {
            debug!(rule_name = %rule.name, rule_id = rule.id, user_id = rule.user_id, "Evaluating global rule.");
            let user_vps_list =
                alert_evaluation_service::get_all_vps_for_user(self.pool.clone(), rule.user_id)
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
                        return Ok(Some(message));
                    }
                    Ok(None) => {}
                    Err(e) => {
                        error!(rule_name = %rule.name, vps_id = vps_instance.id, error = %e, "Error evaluating global rule for VPS.");
                    }
                }
            }
            Ok(None)
        }
    }

    async fn evaluate_rule_for_single_vps(
        &self,
        rule: &alert_rule::Model,
        vps_id: i32,
        vps_name: &str,
    ) -> Result<Option<String>, EvaluationError> {
        let now = Utc::now();

        if rule.vps_id.is_some() || rule.vps_id.is_none() {
            if let Some(last_triggered) = rule.last_triggered_at {
                let cooldown_period = ChronoDuration::seconds(rule.cooldown_seconds as i64);
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

        let metrics: Vec<performance_metric::Model> =
            alert_evaluation_service::get_performance_metrics(
                self.pool.clone(),
                vps_id,
                start_time,
                now,
            )
            .await?;

        if metrics.is_empty() {
            return Ok(None);
        }

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
                    all_match = false;
                    break;
                }
                _ => {
                    all_match = false;
                    break;
                }
            }
            if !rule.metric_type.eq("traffic_usage_percent") {
                last_metric_value_str = format!("{current_value:.2}");
            }

            if !rule.metric_type.eq("traffic_usage_percent") {
                let condition_met = match rule.comparison_operator.as_str() {
                    ">" => current_value > rule.threshold,
                    "<" => current_value < rule.threshold,
                    ">=" => current_value >= rule.threshold,
                    "<=" => current_value <= rule.threshold,
                    "=" | "==" => (current_value - rule.threshold).abs() < f64::EPSILON,
                    "!=" => (current_value - rule.threshold).abs() > f64::EPSILON,
                    _ => {
                        all_match = false;
                        break;
                    }
                };

                if !condition_met {
                    all_match = false;
                    break;
                }
            }
        }

        if rule.metric_type.eq("traffic_usage_percent") {
            let vps_model_option =
                vps_service::get_vps_by_id(self.pool.clone(), vps_id).await?;
            if let Some(vps_model) = vps_model_option {
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
                                return Ok(None);
                            }
                        };

                        let usage_percent = (total_used as f64 / limit_bytes as f64) * 100.0;
                        last_metric_value_str = format!("{usage_percent:.2}");

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
                                return Ok(None);
                            }
                        };

                        all_match = condition_met;
                    } else {
                        debug!(
                            vps_id = vps_id,
                            rule_id = rule.id,
                            "Traffic limit is 0 or not set. Cannot evaluate traffic_usage_percent rule."
                        );
                        return Ok(None);
                    }
                } else {
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
                return Err(EvaluationError::VpsNameNotFound(vps_id));
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
                vps_id,
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
