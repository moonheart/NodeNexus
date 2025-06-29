use axum::{
    body::Body as AxumBody, extract::{Extension, State}, http::{header, Request}, middleware::Next, response::Response
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::sync::Arc;

use crate::{
    db::entities::user,
    web::{models::AuthenticatedUser, AppState},
};

pub async fn i18n_middleware(
    State(app_state): State<Arc<AppState>>,
    Extension(auth_user): Extension<Option<AuthenticatedUser>>,
    req: Request<AxumBody>,
    next: Next,
) -> Response {
    let mut locale = "auto".to_string();

    if let Some(user) = auth_user {
        if let Ok(Some(user_model)) = user::Entity::find_by_id(user.id)
            .one(&app_state.db_pool)
            .await
        {
            locale = user_model.language;
        }
    }

    if locale == "auto" {
        locale = req
            .headers()
            .get(header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "en".to_string());
    }

    rust_i18n::set_locale(&locale);

    next.run(req).await
}