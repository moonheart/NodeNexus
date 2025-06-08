use sqlx::{PgPool, Postgres, Transaction, Executor};
use std::sync::Arc;
use crate::db::models::{AlertRule, AlertRuleFromDb, AlertRuleChannel}; // Added AlertRuleFromDb
use crate::http_server::models::alert_models::{CreateAlertRuleRequest, UpdateAlertRuleRequest};
use crate::http_server::AppError; // Assuming AppError is accessible

#[derive(Clone)]
pub struct AlertService {
    db_pool: Arc<PgPool>,
}

impl AlertService {
    pub fn new(db_pool: Arc<PgPool>) -> Self {
        Self { db_pool }
    }

    pub async fn create_alert_rule(
        &self,
        user_id: i32,
        payload: CreateAlertRuleRequest,
    ) -> Result<AlertRule, AppError> {
        let mut tx = self.db_pool.begin().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Use a default cooldown if not provided in the payload, or take it from payload
        let cooldown_seconds = payload.cooldown_seconds.unwrap_or(300); // Default to 300s (5min)

        let rule_from_db = sqlx::query_as!(
            AlertRuleFromDb,
            r#"
            INSERT INTO alert_rules (user_id, name, vps_id, metric_type, threshold, comparison_operator, duration_seconds, cooldown_seconds)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, user_id, name, vps_id, metric_type, threshold, comparison_operator, duration_seconds, is_active, last_triggered_at, cooldown_seconds, created_at, updated_at
            "#,
            user_id,
            payload.name,
            payload.vps_id,
            payload.metric_type,
            payload.threshold,
            payload.comparison_operator,
            payload.duration_seconds,
            cooldown_seconds
        )
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut notification_channel_ids_to_link = Vec::new();
        if let Some(channel_ids) = payload.notification_channel_ids {
            if !channel_ids.is_empty() {
                Self::link_channels_to_rule(&mut tx, rule_from_db.id, &channel_ids).await?;
                notification_channel_ids_to_link = channel_ids;
            }
        }
        
        tx.commit().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        Ok(AlertRule {
            id: rule_from_db.id,
            user_id: rule_from_db.user_id,
            name: rule_from_db.name,
            vps_id: rule_from_db.vps_id,
            metric_type: rule_from_db.metric_type,
            threshold: rule_from_db.threshold,
            comparison_operator: rule_from_db.comparison_operator,
            duration_seconds: rule_from_db.duration_seconds,
            notification_channel_ids: Some(notification_channel_ids_to_link),
            is_active: rule_from_db.is_active,
            last_triggered_at: rule_from_db.last_triggered_at,
            cooldown_seconds: rule_from_db.cooldown_seconds,
            created_at: rule_from_db.created_at,
            updated_at: rule_from_db.updated_at,
        })
    }

    pub async fn get_all_alert_rules_for_user(&self, user_id: i32) -> Result<Vec<AlertRule>, AppError> {
        let rules_from_db = sqlx::query_as!(
            AlertRuleFromDb,
            r#"
            SELECT id, user_id, name, vps_id, metric_type, threshold, comparison_operator, duration_seconds, is_active, last_triggered_at, cooldown_seconds, created_at, updated_at
            FROM alert_rules WHERE user_id = $1 ORDER BY name
            "#,
            user_id
        )
        .fetch_all(&*self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut full_rules = Vec::new();
        for rule_db in rules_from_db {
            let channel_ids = Self::get_linked_channel_ids(&self.db_pool, rule_db.id).await?;
            full_rules.push(AlertRule {
                id: rule_db.id,
                user_id: rule_db.user_id,
                name: rule_db.name,
                vps_id: rule_db.vps_id,
                metric_type: rule_db.metric_type,
                threshold: rule_db.threshold,
                comparison_operator: rule_db.comparison_operator,
                duration_seconds: rule_db.duration_seconds,
                notification_channel_ids: Some(channel_ids),
                is_active: rule_db.is_active,
                last_triggered_at: rule_db.last_triggered_at,
                cooldown_seconds: rule_db.cooldown_seconds,
                created_at: rule_db.created_at,
                updated_at: rule_db.updated_at,
            });
        }
        Ok(full_rules)
    }

    pub async fn get_alert_rule_by_id_for_user(&self, rule_id: i32, user_id: i32) -> Result<AlertRule, AppError> {
        let rule_from_db = sqlx::query_as!(
            AlertRuleFromDb,
            r#"
            SELECT id, user_id, name, vps_id, metric_type, threshold, comparison_operator, duration_seconds, is_active, last_triggered_at, cooldown_seconds, created_at, updated_at
            FROM alert_rules WHERE id = $1 AND user_id = $2
            "#,
            rule_id,
            user_id
        )
        .fetch_optional(&*self.db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Alert rule not found".to_string()))?;
        
        let channel_ids = Self::get_linked_channel_ids(&self.db_pool, rule_from_db.id).await?;
        Ok(AlertRule {
            id: rule_from_db.id,
            user_id: rule_from_db.user_id,
            name: rule_from_db.name,
            vps_id: rule_from_db.vps_id,
            metric_type: rule_from_db.metric_type,
            threshold: rule_from_db.threshold,
            comparison_operator: rule_from_db.comparison_operator,
            duration_seconds: rule_from_db.duration_seconds,
            notification_channel_ids: Some(channel_ids),
            is_active: rule_from_db.is_active,
            last_triggered_at: rule_from_db.last_triggered_at,
            cooldown_seconds: rule_from_db.cooldown_seconds,
            created_at: rule_from_db.created_at,
            updated_at: rule_from_db.updated_at,
        })
    }

    pub async fn update_alert_rule(
        &self,
        rule_id: i32,
        user_id: i32,
        payload: UpdateAlertRuleRequest,
    ) -> Result<AlertRule, AppError> {
        let mut tx = self.db_pool.begin().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Fetch current rule to see if it exists and belongs to the user
        let current_rule_from_db = sqlx::query_as!(AlertRuleFromDb,
            "SELECT id, user_id, name, vps_id, metric_type, threshold, comparison_operator, duration_seconds, is_active, last_triggered_at, cooldown_seconds, created_at, updated_at FROM alert_rules WHERE id = $1 AND user_id = $2",
            rule_id, user_id
        )
        .fetch_optional(&mut *tx)
        .await.map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Alert rule not found or not owned by user".to_string()))?;

        let name = payload.name.unwrap_or(current_rule_from_db.name);
        let vps_id = match payload.vps_id {
            Some(Some(id)) => Some(id),
            Some(None) => None,
            None => current_rule_from_db.vps_id,
        };
        let metric_type = payload.metric_type.unwrap_or(current_rule_from_db.metric_type);
        let threshold = payload.threshold.unwrap_or(current_rule_from_db.threshold);
        let comparison_operator = payload.comparison_operator.unwrap_or(current_rule_from_db.comparison_operator);
        let duration_seconds = payload.duration_seconds.unwrap_or(current_rule_from_db.duration_seconds);
        let cooldown_seconds = payload.cooldown_seconds.unwrap_or(current_rule_from_db.cooldown_seconds);

        sqlx::query!(
            r#"
            UPDATE alert_rules
            SET name = $1, vps_id = $2, metric_type = $3, threshold = $4,
                comparison_operator = $5, duration_seconds = $6, cooldown_seconds = $7, updated_at = now()
            WHERE id = $8 AND user_id = $9
            "#,
            name, vps_id, metric_type, threshold, comparison_operator, duration_seconds, cooldown_seconds, rule_id, user_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if let Some(channel_ids) = payload.notification_channel_ids {
            // Clear existing links and add new ones
            sqlx::query!("DELETE FROM alert_rule_channels WHERE alert_rule_id = $1", rule_id)
                .execute(&mut *tx)
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;
            if !channel_ids.is_empty() {
                Self::link_channels_to_rule(&mut tx, rule_id, &channel_ids).await?;
            }
        }

        tx.commit().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        self.get_alert_rule_by_id_for_user(rule_id, user_id).await
    }

    pub async fn delete_alert_rule(&self, rule_id: i32, user_id: i32) -> Result<(), AppError> {
        let mut tx = self.db_pool.begin().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        // First, delete associations in alert_rule_channels
        sqlx::query!("DELETE FROM alert_rule_channels WHERE alert_rule_id = $1", rule_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Then, delete the rule itself
        let result = sqlx::query!("DELETE FROM alert_rules WHERE id = $1 AND user_id = $2", rule_id, user_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if result.rows_affected() == 0 {
            tx.rollback().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
            return Err(AppError::NotFound("Alert rule not found or not owned by user".to_string()));
        }
        
        tx.commit().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    }
    
    async fn link_channels_to_rule(
        tx: &mut Transaction<'_, Postgres>,
        rule_id: i32,
        channel_ids: &[i32],
    ) -> Result<(), AppError> {
        for channel_id in channel_ids {
            sqlx::query!(
                "INSERT INTO alert_rule_channels (alert_rule_id, channel_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
                rule_id,
                channel_id
            )
            .execute(&mut **tx) // Use &mut **tx to dereference and then re-borrow as mutable
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        }
        Ok(())
    }

    async fn get_linked_channel_ids(db_pool: &PgPool, rule_id: i32) -> Result<Vec<i32>, AppError> {
        let linked_channels = sqlx::query_as!(
            AlertRuleChannel,
            "SELECT alert_rule_id, channel_id FROM alert_rule_channels WHERE alert_rule_id = $1",
            rule_id
        )
        .fetch_all(db_pool)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        
        Ok(linked_channels.into_iter().map(|arc| arc.channel_id).collect())
    }

    /// Fetches all alert rules for the evaluation service.
    /// Currently, this fetches all rules as there's no 'is_active' flag.
    /// This also does not yet include 'last_triggered_at' which is needed for cooldown.
    pub async fn get_all_rules_for_evaluation(&self) -> Result<Vec<AlertRuleFromDb>, sqlx::Error> {
        let rules = sqlx::query_as!(
            AlertRuleFromDb,
            r#"
            SELECT
                id,
                user_id,
                name,
                vps_id,
                metric_type,
                threshold,
                comparison_operator,
                duration_seconds,
                is_active,
                last_triggered_at,
                cooldown_seconds,
                created_at,
                updated_at
            FROM alert_rules
            ORDER BY id
            "#
        )
        .fetch_all(&*self.db_pool)
        .await?;
        Ok(rules)
    }

    /// Fetches all active alert rules for the evaluation service.
    pub async fn get_all_active_rules_for_evaluation(&self) -> Result<Vec<AlertRuleFromDb>, sqlx::Error> {
        let rules = sqlx::query_as!(
            AlertRuleFromDb,
            r#"
            SELECT
                id,
                user_id,
                name,
                vps_id,
                metric_type,
                threshold,
                comparison_operator,
                duration_seconds,
                is_active,
                last_triggered_at,
                cooldown_seconds,
                created_at,
                updated_at
            FROM alert_rules
            WHERE is_active = TRUE
            ORDER BY id
            "#
        )
        .fetch_all(&*self.db_pool)
        .await?;
        Ok(rules)
    }

    pub async fn update_alert_rule_last_triggered(&self, rule_id: i32, user_id: i32) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE alert_rules SET last_triggered_at = NOW(), updated_at = NOW() WHERE id = $1 AND user_id = $2",
            rule_id,
            user_id
        )
        .execute(&*self.db_pool)
        .await?;
        Ok(())
    }

    pub async fn update_alert_rule_status(&self, rule_id: i32, user_id: i32, is_active: bool) -> Result<AlertRule, AppError> {
        let mut tx = self.db_pool.begin().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let updated_rule_from_db = sqlx::query_as!(
            AlertRuleFromDb,
            r#"
            UPDATE alert_rules
            SET is_active = $1, updated_at = now()
            WHERE id = $2 AND user_id = $3
            RETURNING id, user_id, name, vps_id, metric_type, threshold, comparison_operator, duration_seconds, is_active, last_triggered_at, cooldown_seconds, created_at, updated_at
            "#,
            is_active,
            rule_id,
            user_id
        )
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AppError::NotFound("Alert rule not found or not owned by user".to_string()))?;

        tx.commit().await.map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Fetch linked channels to construct the full AlertRule
        let channel_ids = Self::get_linked_channel_ids(&self.db_pool, updated_rule_from_db.id).await?;

        Ok(AlertRule {
            id: updated_rule_from_db.id,
            user_id: updated_rule_from_db.user_id,
            name: updated_rule_from_db.name,
            vps_id: updated_rule_from_db.vps_id,
            metric_type: updated_rule_from_db.metric_type,
            threshold: updated_rule_from_db.threshold,
            comparison_operator: updated_rule_from_db.comparison_operator,
            duration_seconds: updated_rule_from_db.duration_seconds,
            notification_channel_ids: Some(channel_ids),
            is_active: updated_rule_from_db.is_active,
            last_triggered_at: updated_rule_from_db.last_triggered_at,
            cooldown_seconds: updated_rule_from_db.cooldown_seconds,
            created_at: updated_rule_from_db.created_at,
            updated_at: updated_rule_from_db.updated_at,
        })
    }
}