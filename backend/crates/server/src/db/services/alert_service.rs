use chrono::Utc;
use sea_orm::{
    ActiveModelTrait,
    ColumnTrait,
    DatabaseConnection,
    DatabaseTransaction,
    DbErr,
    EntityTrait,
    IntoActiveModel,
    QueryFilter,
    QueryOrder,
    Set,
    TransactionTrait, // Removed ActiveValue, ModelTrait, Order
    sea_query::{Expr, OnConflict},
};
use std::sync::Arc;

// AlertRule DTO is still used from models.rs for API responses
use crate::db::entities::{alert_rule, alert_rule_channel}; // Removed prelude::*
use crate::db::models::AlertRule;
use crate::web::error::AppError;
use crate::web::models::alert_models::{CreateAlertRuleRequest, UpdateAlertRuleRequest};

#[derive(Clone)]
pub struct AlertService {
    db: Arc<DatabaseConnection>,
}

impl AlertService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn create_alert_rule(
        &self,
        user_id: i32,
        payload: CreateAlertRuleRequest,
    ) -> Result<AlertRule, AppError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let cooldown_seconds = payload.cooldown_seconds.unwrap_or(300);
        let now = Utc::now();

        let new_rule_active_model = alert_rule::ActiveModel {
            user_id: Set(user_id),
            name: Set(payload.name),
            vps_id: Set(payload.vps_id),
            metric_type: Set(payload.metric_type),
            threshold: Set(payload.threshold),
            comparison_operator: Set(payload.comparison_operator),
            duration_seconds: Set(payload.duration_seconds),
            cooldown_seconds: Set(cooldown_seconds),
            is_active: Set(true), // Default to active
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default() // For id and last_triggered_at
        };

        let rule_model = new_rule_active_model
            .insert(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut notification_channel_ids_to_link = Vec::new();
        if let Some(channel_ids) = payload.notification_channel_ids {
            if !channel_ids.is_empty() {
                Self::link_channels_to_rule(&txn, rule_model.id, &channel_ids).await?;
                notification_channel_ids_to_link = channel_ids;
            }
        }

        txn.commit()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(AlertRule {
            id: rule_model.id,
            user_id: rule_model.user_id,
            name: rule_model.name,
            vps_id: rule_model.vps_id,
            metric_type: rule_model.metric_type,
            threshold: rule_model.threshold,
            comparison_operator: rule_model.comparison_operator,
            duration_seconds: rule_model.duration_seconds,
            notification_channel_ids: Some(notification_channel_ids_to_link),
            is_active: rule_model.is_active,
            last_triggered_at: rule_model.last_triggered_at,
            cooldown_seconds: rule_model.cooldown_seconds,
            created_at: rule_model.created_at,
            updated_at: rule_model.updated_at,
        })
    }

    pub async fn get_all_alert_rules_for_user(
        &self,
        user_id: i32,
    ) -> Result<Vec<AlertRule>, AppError> {
        let rule_models = alert_rule::Entity::find()
            .filter(alert_rule::Column::UserId.eq(user_id))
            .order_by_asc(alert_rule::Column::Name)
            .all(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let mut full_rules = Vec::new();
        for rule_model in rule_models {
            let channel_ids = Self::get_linked_channel_ids(&self.db, rule_model.id).await?;
            full_rules.push(AlertRule {
                id: rule_model.id,
                user_id: rule_model.user_id,
                name: rule_model.name,
                vps_id: rule_model.vps_id,
                metric_type: rule_model.metric_type,
                threshold: rule_model.threshold,
                comparison_operator: rule_model.comparison_operator,
                duration_seconds: rule_model.duration_seconds,
                notification_channel_ids: Some(channel_ids),
                is_active: rule_model.is_active,
                last_triggered_at: rule_model.last_triggered_at,
                cooldown_seconds: rule_model.cooldown_seconds,
                created_at: rule_model.created_at,
                updated_at: rule_model.updated_at,
            });
        }
        Ok(full_rules)
    }

    pub async fn get_alert_rule_by_id_for_user(
        &self,
        rule_id: i32,
        user_id: i32,
    ) -> Result<AlertRule, AppError> {
        let rule_model = alert_rule::Entity::find_by_id(rule_id)
            .filter(alert_rule::Column::UserId.eq(user_id))
            .one(&*self.db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| AppError::NotFound("Alert rule not found".to_string()))?;

        let channel_ids = Self::get_linked_channel_ids(&self.db, rule_model.id).await?;
        Ok(AlertRule {
            id: rule_model.id,
            user_id: rule_model.user_id,
            name: rule_model.name,
            vps_id: rule_model.vps_id,
            metric_type: rule_model.metric_type,
            threshold: rule_model.threshold,
            comparison_operator: rule_model.comparison_operator,
            duration_seconds: rule_model.duration_seconds,
            notification_channel_ids: Some(channel_ids),
            is_active: rule_model.is_active,
            last_triggered_at: rule_model.last_triggered_at,
            cooldown_seconds: rule_model.cooldown_seconds,
            created_at: rule_model.created_at,
            updated_at: rule_model.updated_at,
        })
    }

    pub async fn update_alert_rule(
        &self,
        rule_id: i32,
        user_id: i32,
        payload: UpdateAlertRuleRequest,
    ) -> Result<AlertRule, AppError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let current_rule_model = alert_rule::Entity::find_by_id(rule_id)
            .filter(alert_rule::Column::UserId.eq(user_id))
            .one(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?
            .ok_or_else(|| {
                AppError::NotFound("Alert rule not found or not owned by user".to_string())
            })?;

        let mut active_rule: alert_rule::ActiveModel = current_rule_model.into_active_model();

        if let Some(name) = payload.name {
            active_rule.name = Set(name);
        }
        // Handle Option<Option<i32>> for vps_id
        if payload.vps_id.is_some() {
            // Check if the outer Option is Some
            active_rule.vps_id = Set(payload.vps_id); // flatten converts Option<Option<T>> to Option<T>
        }
        if let Some(metric_type) = payload.metric_type {
            active_rule.metric_type = Set(metric_type);
        }
        if let Some(threshold) = payload.threshold {
            active_rule.threshold = Set(threshold);
        }
        if let Some(comparison_operator) = payload.comparison_operator {
            active_rule.comparison_operator = Set(comparison_operator);
        }
        if let Some(duration_seconds) = payload.duration_seconds {
            active_rule.duration_seconds = Set(duration_seconds);
        }
        if let Some(cooldown_seconds) = payload.cooldown_seconds {
            active_rule.cooldown_seconds = Set(cooldown_seconds);
        }
        active_rule.updated_at = Set(Utc::now());

        active_rule
            .update(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if let Some(channel_ids) = payload.notification_channel_ids {
            alert_rule_channel::Entity::delete_many()
                .filter(alert_rule_channel::Column::AlertRuleId.eq(rule_id))
                .exec(&txn)
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;
            if !channel_ids.is_empty() {
                Self::link_channels_to_rule(&txn, rule_id, &channel_ids).await?;
            }
        }

        txn.commit()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        // Fetch again to get the DTO with potentially updated channel links
        self.get_alert_rule_by_id_for_user(rule_id, user_id).await
    }

    pub async fn delete_alert_rule(&self, rule_id: i32, user_id: i32) -> Result<(), AppError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // DB cascade should handle alert_rule_channels, but explicit deletion can be kept if preferred.
        // For SeaORM, relying on cascade or explicit delete:
        // alert_rule_channel::Entity::delete_many()
        //     .filter(alert_rule_channel::Column::AlertRuleId.eq(rule_id))
        //     .exec(&txn)
        //     .await
        //     .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let result = alert_rule::Entity::delete_many()
            .filter(alert_rule::Column::Id.eq(rule_id))
            .filter(alert_rule::Column::UserId.eq(user_id))
            .exec(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if result.rows_affected == 0 {
            txn.rollback()
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;
            return Err(AppError::NotFound(
                "Alert rule not found or not owned by user".to_string(),
            ));
        }

        txn.commit()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn link_channels_to_rule(
        txn: &DatabaseTransaction,
        rule_id: i32,
        channel_ids: &[i32],
    ) -> Result<(), AppError> {
        if channel_ids.is_empty() {
            return Ok(());
        }
        let new_links: Vec<alert_rule_channel::ActiveModel> = channel_ids
            .iter()
            .map(|&channel_id| alert_rule_channel::ActiveModel {
                alert_rule_id: Set(rule_id),
                channel_id: Set(channel_id),
            })
            .collect();

        alert_rule_channel::Entity::insert_many(new_links)
            .on_conflict(
                OnConflict::columns([
                    alert_rule_channel::Column::AlertRuleId,
                    alert_rule_channel::Column::ChannelId,
                ])
                .do_nothing()
                .to_owned(),
            )
            .exec(txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;
        Ok(())
    }

    async fn get_linked_channel_ids(
        db: &DatabaseConnection,
        rule_id: i32,
    ) -> Result<Vec<i32>, AppError> {
        let linked_channels = alert_rule_channel::Entity::find()
            .filter(alert_rule_channel::Column::AlertRuleId.eq(rule_id))
            .all(db)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        Ok(linked_channels
            .into_iter()
            .map(|arc| arc.channel_id)
            .collect())
    }

    pub async fn get_all_rules_for_evaluation(&self) -> Result<Vec<alert_rule::Model>, DbErr> {
        alert_rule::Entity::find()
            .order_by_asc(alert_rule::Column::Id)
            .all(&*self.db)
            .await
    }

    pub async fn get_all_active_rules_for_evaluation(
        &self,
    ) -> Result<Vec<alert_rule::Model>, DbErr> {
        alert_rule::Entity::find()
            .filter(alert_rule::Column::IsActive.eq(true))
            .order_by_asc(alert_rule::Column::Id)
            .all(&*self.db)
            .await
    }

    pub async fn update_alert_rule_last_triggered(
        &self,
        rule_id: i32,
        user_id: i32,
    ) -> Result<(), DbErr> {
        let result = alert_rule::Entity::update_many()
            .col_expr(alert_rule::Column::LastTriggeredAt, Expr::value(Utc::now()))
            .col_expr(alert_rule::Column::UpdatedAt, Expr::value(Utc::now()))
            .filter(alert_rule::Column::Id.eq(rule_id))
            .filter(alert_rule::Column::UserId.eq(user_id))
            .exec(&*self.db)
            .await?;

        if result.rows_affected == 0 {
            // Consider if an error should be returned or if it's acceptable for no update to occur
            // For now, matching original behavior of not explicitly erroring on 0 rows affected here.
        }
        Ok(())
    }

    pub async fn update_alert_rule_status(
        &self,
        rule_id: i32,
        user_id: i32,
        is_active: bool,
    ) -> Result<AlertRule, AppError> {
        let txn = self
            .db
            .begin()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        let update_result = alert_rule::Entity::update_many()
            .col_expr(alert_rule::Column::IsActive, Expr::value(is_active))
            .col_expr(alert_rule::Column::UpdatedAt, Expr::value(Utc::now()))
            .filter(alert_rule::Column::Id.eq(rule_id))
            .filter(alert_rule::Column::UserId.eq(user_id))
            .exec(&txn)
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        if update_result.rows_affected == 0 {
            txn.rollback()
                .await
                .map_err(|e| AppError::DatabaseError(e.to_string()))?;
            return Err(AppError::NotFound(
                "Alert rule not found or not owned by user".to_string(),
            ));
        }

        txn.commit()
            .await
            .map_err(|e| AppError::DatabaseError(e.to_string()))?;

        // Fetch the full rule DTO to return
        self.get_alert_rule_by_id_for_user(rule_id, user_id).await
    }
}
