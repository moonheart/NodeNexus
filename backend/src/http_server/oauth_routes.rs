// backend/src/http_server/oauth_routes.rs

use axum::{
    extract::{Path, Query, State},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use std::sync::Arc;
use crate::http_server::{AppState, AppError};
use crate::db::services::oauth_service;
use uuid::Uuid;
use axum_extra::extract::cookie::{Cookie, SameSite};

pub fn create_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/auth/providers", get(get_providers_handler))
        .route("/api/auth/{provider}/login", get(login_handler))
        .route("/api/auth/{provider}/callback", get(callback_handler))
}

async fn get_providers_handler(
    State(app_state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let providers = oauth_service::get_all_providers(&app_state.db_pool).await?;
    Ok(axum::Json(providers))
}

async fn login_handler(
    State(app_state): State<Arc<AppState>>,
    Path(provider): Path<String>,
) -> Result<Response, AppError> {
    let provider_config = oauth_service::get_provider_config(
        &app_state.db_pool,
        &provider,
        &app_state.config.notification_encryption_key,
    )
    .await?;

    let state = Uuid::new_v4().to_string();
    let mut auth_url = format!(
        "{}?client_id={}&response_type=code&redirect_uri={}&state={}",
        provider_config.auth_url,
        provider_config.client_id,
        // This should be configurable or derived from request headers
        format!("{}/api/auth/{}/callback", &app_state.config.frontend_url, provider),
        state
    );

    if let Some(scopes) = provider_config.scopes {
        auth_url.push_str(&format!("&scope={}", scopes));
    }

    let cookie = Cookie::build(("oauth_state", state))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(true) // Should be true in production
        .build();

    let mut response = Redirect::to(&auth_url).into_response();
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        cookie.to_string().parse().unwrap(),
    );

    Ok(response)
}

use serde::Deserialize;
use axum_extra::extract::cookie::CookieJar;

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: String,
    state: String,
}

async fn callback_handler(
    State(app_state): State<Arc<AppState>>,
    Path(provider): Path<String>,
    Query(query): Query<CallbackQuery>,
    jar: CookieJar,
) -> Result<Response, AppError> {
    // 1. Verify state to prevent CSRF
    let stored_state = jar.get("oauth_state")
        .map(|c| c.value().to_string())
        .ok_or_else(|| AppError::InvalidInput("Missing CSRF state cookie.".to_string()))?;

    if stored_state != query.state {
        return Err(AppError::InvalidInput("CSRF state mismatch.".to_string()));
    }

    let provider_config = oauth_service::get_provider_config(
        &app_state.db_pool,
        &provider,
        &app_state.config.notification_encryption_key,
    )
    .await?;
    
    let redirect_uri = format!("{}/api/auth/{}/callback", &app_state.config.frontend_url, provider);

    let token_response = oauth_service::exchange_code_for_token(
        &provider_config,
        &query.code,
        &redirect_uri,
    ).await?;

    let user_info = oauth_service::get_user_info(&provider_config, &token_response.access_token).await?;

    let mapping = provider_config.user_info_mapping.as_ref()
        .and_then(|v| v.as_object())
        .ok_or_else(|| AppError::InternalServerError("User info mapping is missing or invalid.".to_string()))?;

    let provider_user_id = user_info.get(mapping.get("id_field").and_then(|v| v.as_str()).unwrap_or("id"))
        .and_then(|v| v.as_str().map(ToString::to_string).or_else(|| v.as_i64().map(|n| n.to_string())))
        .ok_or_else(|| AppError::InternalServerError("Could not extract provider user ID.".to_string()))?;

    let username = user_info.get(mapping.get("username_field").and_then(|v| v.as_str()).unwrap_or("login"))
        .and_then(|v| v.as_str().map(ToString::to_string))
        .ok_or_else(|| AppError::InternalServerError("Could not extract username.".to_string()))?;

    use sea_orm::{EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};
    use crate::db::entities::{user, user_identity_provider};
    use crate::http_server::auth_logic;

    let identity = user_identity_provider::Entity::find()
        .filter(user_identity_provider::Column::ProviderName.eq(&provider))
        .filter(user_identity_provider::Column::ProviderUserId.eq(&provider_user_id))
        .one(&app_state.db_pool).await?;

    let user_model = if let Some(identity) = identity {
        // Identity found, fetch the associated user
        user::Entity::find_by_id(identity.user_id).one(&app_state.db_pool).await?
            .ok_or(AppError::UserNotFound)?
    } else {
        // No identity found, check for existing user by username or create a new one
        let existing_user = user::Entity::find().filter(user::Column::Username.eq(&username)).one(&app_state.db_pool).await?;
        if let Some(existing_user) = existing_user {
             // For now, we'll treat this as a conflict.
             // A more advanced implementation might link the new identity to this existing user.
            return Err(AppError::Conflict("Username already exists.".to_string()));
        }

        // Create new user
        let new_user = user::ActiveModel {
            username: Set(username),
            password_login_disabled: Set(true),
            ..Default::default()
        };
        let user_model = new_user.insert(&app_state.db_pool).await?;

        // Create new identity and link it to the new user
        let new_identity = user_identity_provider::ActiveModel {
            user_id: Set(user_model.id),
            provider_name: Set(provider),
            provider_user_id: Set(provider_user_id),
            ..Default::default()
        };
        new_identity.insert(&app_state.db_pool).await?;
        user_model
    };

    // At this point, we have a user model, either found or newly created.
    // Now, create JWT, set cookie, and redirect.
    let login_response = auth_logic::create_jwt_for_user(&user_model, &app_state.config.jwt_secret)?;
    
    let auth_cookie = Cookie::build(("token", login_response.token))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(true) // Set to true in production
        .finish();

    let redirect_url = format!("{}/auth/callback?token={}", &app_state.config.frontend_url, auth_cookie.value());
    let mut response = Redirect::to(&redirect_url).into_response();
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        auth_cookie.to_string().parse().unwrap(),
    );

    // Also remove the state cookie as it's no longer needed
    let remove_state_cookie = Cookie::build(("oauth_state", ""))
        .path("/")
        .max_age(time::Duration::ZERO)
        .finish();
    response.headers_mut().append(
        axum::http::header::SET_COOKIE,
        remove_state_cookie.to_string().parse().unwrap(),
    );

    Ok(response)
}