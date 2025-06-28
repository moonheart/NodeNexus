// backend/src/http_server/oauth_routes.rs

use axum::{
    extract::{Path, Query, State, Extension},
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use std::sync::Arc;
use crate::web::{AppState, AppError, models::AuthenticatedUser};
use crate::db::services::oauth_service::{self, OAuthState, OAuthCallbackResult};
use uuid::Uuid;
use axum_extra::extract::cookie::{Cookie, SameSite, CookieJar};
use serde::Deserialize;
use urlencoding;

pub fn create_public_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/providers", get(get_providers_handler))
        .route("/{provider}/login", get(login_handler))
        .route("/{provider}/callback", get(callback_handler))
}

pub fn create_protected_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{provider}/link", get(link_handler))
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
    ).await?;

    let state = OAuthState {
        nonce: Uuid::new_v4().to_string(),
        action: "login".to_string(),
        user_id: None,
    };
    let state_str = serde_json::to_string(&state)?;

    let redirect_uri = format!("{}/api/auth/{}/callback", &app_state.config.frontend_url, provider);
    let encoded_state = urlencoding::encode(&state_str);
    let mut auth_url = format!(
        "{}?client_id={}&response_type=code&redirect_uri={}&state={}",
        provider_config.auth_url,
        provider_config.client_id,
        redirect_uri,
        encoded_state
    );

    if let Some(scopes) = provider_config.scopes {
        auth_url.push_str(&format!("&scope={scopes}"));
    }

    let cookie = Cookie::build(("oauth_state", state_str))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(true)
        .build();

    let mut response = Redirect::to(&auth_url).into_response();
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        cookie.to_string().parse().unwrap(),
    );

    Ok(response)
}

async fn link_handler(
    State(app_state): State<Arc<AppState>>,
    Path(provider): Path<String>,
    Extension(user): Extension<AuthenticatedUser>,
) -> Result<Response, AppError> {
    let provider_config = oauth_service::get_provider_config(
        &app_state.db_pool,
        &provider,
        &app_state.config.notification_encryption_key,
    )
    .await?;

    let state = OAuthState {
        nonce: Uuid::new_v4().to_string(),
        action: "link".to_string(),
        user_id: Some(user.id),
    };
    let state_str = serde_json::to_string(&state)?;

    let redirect_uri = format!(
        "{}/api/auth/{}/callback",
        &app_state.config.frontend_url, provider
    );
    let encoded_state = urlencoding::encode(&state_str);
    let mut auth_url = format!(
        "{}?client_id={}&response_type=code&redirect_uri={}&state={}",
        provider_config.auth_url, provider_config.client_id, redirect_uri, encoded_state
    );

    if let Some(scopes) = provider_config.scopes {
        auth_url.push_str(&format!("&scope={scopes}"));
    }

    let cookie = Cookie::build(("oauth_state", state_str))
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(true)
        .build();

    let mut response = Redirect::to(&auth_url).into_response();
    response.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        cookie.to_string().parse().unwrap(),
    );

    Ok(response)
}

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
    let stored_state_str = jar.get("oauth_state")
        .map(|c| c.value().to_string())
        .ok_or_else(|| AppError::InvalidInput("Missing CSRF state cookie.".to_string()))?;
    
    let query_state: OAuthState = serde_json::from_str(&query.state)?;
    let stored_state: OAuthState = serde_json::from_str(&stored_state_str)?;

    if query_state.nonce != stored_state.nonce {
        return Err(AppError::InvalidInput("CSRF state nonce mismatch.".to_string()));
    }

    let result = oauth_service::handle_oauth_callback(
        &app_state.db_pool,
        &app_state.config,
        &provider,
        &query.code,
        &stored_state, // Use the cookie's state as the source of truth
    ).await;

    let mut response = match result {
        Ok(OAuthCallbackResult::Login { token }) => {
            let auth_cookie = Cookie::build(("token", token))
                .path("/")
                .http_only(true)
                .same_site(SameSite::Lax)
                .secure(true)
                .build();
            
            let redirect_url = format!("{}/auth/callback?token={}", &app_state.config.frontend_url, auth_cookie.value());
            let mut resp = Redirect::to(&redirect_url).into_response();
            resp.headers_mut().insert(
                axum::http::header::SET_COOKIE,
                auth_cookie.to_string().parse().unwrap(),
            );
            resp
        },
        Ok(OAuthCallbackResult::LinkSuccess) => {
            let redirect_url = format!("{}/settings/account?link_success=true", &app_state.config.frontend_url);
            Redirect::to(&redirect_url).into_response()
        },
        Err(e) => {
            // On error, redirect to login page with an error message
            let error_str = e.to_string();
            let error_message = urlencoding::encode(&error_str);
            let redirect_url = format!("{}/login?error={}", &app_state.config.frontend_url, error_message);
            Redirect::to(&redirect_url).into_response()
        }
    };

    // Clean up the state cookie
    let remove_state_cookie = Cookie::build(("oauth_state", ""))
        .path("/")
        .max_age(time::Duration::ZERO)
        .build();
    response.headers_mut().append(
        axum::http::header::SET_COOKIE,
        remove_state_cookie.to_string().parse().unwrap(),
    );

    Ok(response)
}