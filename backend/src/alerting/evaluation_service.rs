use std::sync::Arc;
use sqlx::PgPool;
use tokio::time::{interval, Duration as TokioDuration}; // Renamed to avoid conflict
use chrono::{Utc, Duration as ChronoDuration}; // Added ChronoDuration
use crate::{
    db::{models::{AlertRuleFromDb, PerformanceMetric, Vps}, services::{AlertService, vps_service}}, // Added PerformanceMetric, Vps, vps_service
    notifications::service::NotificationService,
};

#[derive(Debug, thiserror::Error)]
pub enum EvaluationError {
    #[error("Database query error: {0}")]
    DatabaseError(#[from] sqlx::Error),
    #[error("Notification error: {0}")]
    NotificationError(#[from] crate::notifications::service::NotificationError),
    #[error("Failed to get VPS name for ID {0}")]
    VpsNameNotFound(i32),
    // Add other specific error types as needed
}

pub struct EvaluationService {
    pool: Arc<PgPool>,
    notification_service: Arc<NotificationService>,
    alert_service: Arc<AlertService>,
}

impl EvaluationService {
    pub fn new(
        pool: Arc<PgPool>,
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
        println!("Alert evaluation service started. Evaluation interval: {} seconds.", period_seconds);
        let mut interval = interval(TokioDuration::from_secs(period_seconds)); // Use TokioDuration
        loop {
            interval.tick().await;
            println!("Running alert evaluation cycle...");
            if let Err(e) = self.run_evaluation_cycle().await {
                eprintln!("Error during alert evaluation cycle: {}", e);
            }
        }
    }

    async fn run_evaluation_cycle(&self) -> Result<(), EvaluationError> {
        println!("Fetching active alert rules...");
        let active_rules = self.alert_service.get_all_active_rules_for_evaluation().await?; // Assuming this method exists or will be created

        println!("Found {} active rules to evaluate.", active_rules.len());

        for rule in active_rules {
            // TODO: Implement cooldown logic here later if needed (e.g., check rule.last_triggered_at)
            
            match self.evaluate_rule(&rule).await {
                Ok(Some(notification_message)) => {
                    println!("Alert rule '{}' (ID: {}) triggered. Sending notifications.", rule.name, rule.id);
                    // We need a method in NotificationService to send notifications for a given rule_id / user_id
                    // This method would look up associated channels and send the message.
                    // For now, let's assume such a method exists or we'll add it.
                    // The user_id is on the rule object.
                    match self.notification_service.send_notifications_for_alert_rule(rule.id, rule.user_id, &notification_message).await {
                        Ok(_) => {
                            println!("Successfully sent notifications for alert rule ID: {}", rule.id);
                            // Update last_triggered_at timestamp
                            if let Err(e_update) = self.alert_service.update_alert_rule_last_triggered(rule.id, rule.user_id).await {
                                eprintln!("Failed to update last_triggered_at for rule ID {}: {}", rule.id, e_update);
                            }
                        }
                        Err(e) => eprintln!("Failed to send notifications for alert rule ID {}: {}", rule.id, e),
                    }
                }
                Ok(None) => {
                    // Rule not triggered, do nothing or log verbosely if needed
                    // println!("Rule '{}' (ID: {}) not triggered.", rule.name, rule.id);
                }
                Err(e) => {
                    eprintln!("Error evaluating rule '{}' (ID: {}): {}", rule.name, rule.id, e);
                }
            }
        }
        Ok(())
    }

    async fn evaluate_rule(&self, rule: &AlertRuleFromDb) -> Result<Option<String>, EvaluationError> {
        if let Some(specific_vps_id) = rule.vps_id {
            // Rule is for a specific VPS
            let vps_name = sqlx::query_scalar!(r#"SELECT name FROM vps WHERE id = $1"#, specific_vps_id)
                .fetch_optional(&*self.pool)
                .await?
                .unwrap_or_else(|| format!("VPS_ID_{}", specific_vps_id));
            
            self.evaluate_rule_for_single_vps(rule, specific_vps_id, &vps_name).await
        } else {
            // Rule is global, apply to all user's VPS
            println!("Evaluating global rule '{}' (ID: {}) for user ID: {}", rule.name, rule.id, rule.user_id);
            let user_vps_list = crate::db::services::vps_service::get_all_vps_for_user(&self.pool, rule.user_id).await?;
            
            if user_vps_list.is_empty() {
                println!("No VPS found for user ID {} to evaluate global rule '{}'", rule.user_id, rule.name);
                return Ok(None);
            }

            for vps_instance in user_vps_list {
                match self.evaluate_rule_for_single_vps(rule, vps_instance.id, &vps_instance.name).await {
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
                        eprintln!("Error evaluating global rule '{}' for VPS ID {}: {}", rule.name, vps_instance.id, e);
                    }
                }
            }
            // If no VPS triggered the global rule
            Ok(None)
        }
    }

    async fn evaluate_rule_for_single_vps(
        &self,
        rule: &AlertRuleFromDb,
        vps_id: i32,
        vps_name: &str,
    ) -> Result<Option<String>, EvaluationError> {
        let now = Utc::now();
        
        // Cooldown check for the rule itself (relevant if called directly or for specific VPS rules)
        // For global rules, this check is effectively done before iterating user's VPS if we consider the rule's last_triggered_at
        if rule.vps_id.is_some() || rule.vps_id.is_none() { // Apply cooldown check universally for now
            if let Some(last_triggered) = rule.last_triggered_at {
                let cooldown_period = ChronoDuration::seconds(rule.cooldown_seconds as i64); // Use rule.cooldown_seconds
                if now < last_triggered + cooldown_period {
                    println!(
                        "Rule '{}' (ID: {}) for VPS '{}' (ID: {}) is in cooldown for {}s. Last triggered: {}. Now: {}",
                        rule.name, rule.id, vps_name, vps_id, rule.cooldown_seconds, last_triggered, now
                    );
                    return Ok(None);
                }
            }
        }

        let start_time = now - ChronoDuration::seconds(rule.duration_seconds as i64);

        let metrics: Vec<PerformanceMetric> = sqlx::query_as!(
            PerformanceMetric,
            r#"
            SELECT id, time, vps_id, cpu_usage_percent, memory_usage_bytes, memory_total_bytes,
                   swap_usage_bytes, swap_total_bytes, disk_io_read_bps, disk_io_write_bps,
                   network_rx_bps, network_tx_bps, network_rx_instant_bps, network_tx_instant_bps,
                   uptime_seconds, total_processes_count, running_processes_count, tcp_established_connection_count
            FROM performance_metrics
            WHERE vps_id = $1 AND time >= $2 AND time <= $3
            ORDER BY time ASC
            "#,
            vps_id,
            start_time,
            now
        )
        .fetch_all(&*self.pool)
        .await?;

        if metrics.is_empty() {
            return Ok(None);
        }

        if metrics.len() < 2 && rule.duration_seconds > 0 {
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
                    current_value = (metric_point.memory_usage_bytes as f64 / metric_point.memory_total_bytes as f64) * 100.0;
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
                 last_metric_value_str = format!("{:.2}", current_value);
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
                    },
                };

                if !condition_met {
                    all_match = false;
                    break;
                }
            }
        } // End of metric iteration loop

        // Handle traffic_usage_percent separately as it uses current cycle data, not historical points
        if rule.metric_type.eq("traffic_usage_percent") {
             let vps_data = vps_service::get_vps_by_id(&*self.pool, vps_id).await?;
             if let Some(vps) = vps_data {
                 if let Some(limit_bytes) = vps.traffic_limit_bytes {
                     if limit_bytes > 0 {
                         let current_rx = vps.traffic_current_cycle_rx_bytes.unwrap_or(0);
                         let current_tx = vps.traffic_current_cycle_tx_bytes.unwrap_or(0);
                         let total_used = match vps.traffic_billing_rule.as_deref() {
                             Some("sum_in_out") => current_rx + current_tx,
                             Some("out_only") => current_tx,
                             Some("max_in_out") => std::cmp::max(current_rx, current_tx),
                             _ => {
                                 eprintln!("Unsupported or missing traffic_billing_rule for VPS ID {}", vps_id);
                                 return Ok(None); // Cannot evaluate without a valid rule
                             }
                         };

                         let usage_percent = (total_used as f64 / limit_bytes as f64) * 100.0;
                         last_metric_value_str = format!("{:.2}", usage_percent); // Update for message

                         let condition_met = match rule.comparison_operator.as_str() {
                             ">" => usage_percent > rule.threshold,
                             "<" => usage_percent < rule.threshold,
                             ">=" => usage_percent >= rule.threshold,
                             "<=" => usage_percent <= rule.threshold,
                             "=" | "==" => (usage_percent - rule.threshold).abs() < f64::EPSILON,
                             "!=" => (usage_percent - rule.threshold).abs() > f64::EPSILON,
                             _ => {
                                 eprintln!("Unsupported comparison_operator for traffic_usage_percent rule ID {}", rule.id);
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
                         println!("Traffic limit is 0 or not set for VPS ID {}. Cannot evaluate traffic_usage_percent rule ID {}", vps_id, rule.id);
                         return Ok(None);
                     }
                 } else {
                     // Limit is not set, cannot calculate percentage
                     println!("Traffic limit is not configured for VPS ID {}. Cannot evaluate traffic_usage_percent rule ID {}", vps_id, rule.id);
                     return Ok(None);
                 }
             } else {
                 eprintln!("VPS with ID {} not found during traffic alert evaluation.", vps_id);
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