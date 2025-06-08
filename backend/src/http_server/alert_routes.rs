use std::sync::Arc;
use axum::{
    extract::{State, Path, Extension},
    routing::{get, post, put, delete},
    Json, Router,
};
use crate::http_server::{
    AppState, AppError,
    auth_logic::AuthenticatedUser,
    models::alert_models::{CreateAlertRuleRequest, UpdateAlertRuleRequest, UpdateAlertRuleStatusRequest}, // Added UpdateAlertRuleStatusRequest
};
use crate::db::models::AlertRule;

pub fn create_alert_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(create_alert_rule_handler).get(get_all_alert_rules_handler))
        .route("/{id}", get(get_alert_rule_handler).put(update_alert_rule_handler).delete(delete_alert_rule_handler))
        .route("/{id}/status", put(update_alert_rule_status_handler)) // Added route for status update
}

async fn create_alert_rule_handler(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<CreateAlertRuleRequest>,
) -> Result<Json<AlertRule>, AppError> {
    let user_id = authenticated_user.id;
    let alert_rule = app_state.alert_service.create_alert_rule(user_id, payload).await?;
    Ok(Json(alert_rule))
}

async fn get_all_alert_rules_handler(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<Vec<AlertRule>>, AppError> {
    let user_id = authenticated_user.id;
    let alert_rules = app_state.alert_service.get_all_alert_rules_for_user(user_id).await?;
    Ok(Json(alert_rules))
}

async fn get_alert_rule_handler(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
) -> Result<Json<AlertRule>, AppError> {
    let user_id = authenticated_user.id;
    let alert_rule = app_state.alert_service.get_alert_rule_by_id_for_user(id, user_id).await?;
    Ok(Json(alert_rule))
}

async fn update_alert_rule_handler(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateAlertRuleRequest>,
) -> Result<Json<AlertRule>, AppError> {
    let user_id = authenticated_user.id;
    let updated_rule = app_state.alert_service.update_alert_rule(id, user_id, payload).await?;
    Ok(Json(updated_rule))
}

async fn delete_alert_rule_handler(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
) -> Result<(), AppError> {
    let user_id = authenticated_user.id;
    app_state.alert_service.delete_alert_rule(id, user_id).await?;
    Ok(())
}

async fn update_alert_rule_status_handler(
    State(app_state): State<Arc<AppState>>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(id): Path<i32>,
    Json(payload): Json<UpdateAlertRuleStatusRequest>,
) -> Result<Json<AlertRule>, AppError> {
    let user_id = authenticated_user.id;
    let updated_rule = app_state.alert_service.update_alert_rule_status(id, user_id, payload.is_active).await?;
    Ok(Json(updated_rule))
}