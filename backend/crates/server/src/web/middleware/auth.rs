use axum::{
    body::Body as AxumBody,
    extract::State,
    http::{Request, header},
    middleware::Next,
    response::Response,
};
use axum_extra::extract::cookie::CookieJar;
use jsonwebtoken::{DecodingKey, Validation, decode};
use std::sync::Arc;
use tracing::warn;

use crate::web::models::{AuthenticatedUser, Claims};
use crate::web::{AppState, error::AppError};

pub async fn auth(
    State(state): State<Arc<AppState>>,
    jar: CookieJar,
    mut req: Request<AxumBody>,
    next: Next,
) -> Result<Response, AppError> {
    let jwt_secret = &state.config.jwt_secret;

    // Try to get token from Authorization header first, then fall back to cookie
    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|header| header.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .or_else(|| jar.get("token").map(|c| c.value().to_string()))
        .ok_or(AppError::InvalidCredentials)?;

    let token_data = decode::<Claims>(
        &token,
        &DecodingKey::from_secret(jwt_secret.as_ref()),
        &Validation::default(),
    )
    .map_err(|e| {
        warn!(error = ?e, "JWT decoding error during auth middleware.");
        AppError::InvalidCredentials // Or "InvalidToken"
    })?;

    let authenticated_user = AuthenticatedUser {
        id: token_data.claims.user_id,
        username: token_data.claims.sub, // Assuming 'sub' is username
    };
    req.extensions_mut().insert(authenticated_user);
    Ok(next.run(req).await)
}
