use crate::db::duckdb_service::user_service;
use crate::db::duckdb_service::DuckDbPool;
use crate::services::encryption_service::{decrypt, encrypt};
use crate::web::error::AppError;
use chrono::{DateTime, Utc};
use duckdb::{params, types::ToSql, Result as DuckDbResult, Row};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::task::JoinError;
use reqwest::Client;
use crate::server::config::ServerConfig;
use crate::services::auth_service;
use super::Error as UserServiceError;


// --- Data Models ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Oauth2Provider {
    pub id: i32,
    pub provider_name: String,
    pub client_id: String,
    pub client_secret: String, // Encrypted in DB, decrypted in this struct
    pub auth_url: String,
    pub token_url: String,
    pub user_info_url: String,
    pub scopes: Option<String>,
    pub user_info_mapping: Option<JsonValue>,
    pub icon_url: Option<String>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIdentityProvider {
    pub id: i32,
    pub user_id: i32,
    pub provider_name: String,
    pub provider_user_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// --- Payloads & DTOs ---

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProviderUpsertPayload {
    pub provider_name: String,
    pub client_id: String,
    pub client_secret: String, // Plain text from user
    pub auth_url: String,
    pub token_url: String,
    pub user_info_url: String,
    pub scopes: Option<String>,
    pub icon_url: Option<String>,
    pub user_info_mapping: JsonValue,
    pub enabled: bool,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AdminProviderInfo {
    pub provider_name: String,
    pub client_id: String,
    pub client_secret: String, // Decrypted
    pub auth_url: String,
    pub token_url: String,
    pub user_info_url: String,
    pub scopes: Option<String>,
    pub icon_url: Option<String>,
    pub user_info_mapping: Option<JsonValue>,
    pub enabled: bool,
}

#[derive(Deserialize, Debug)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub scope: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OAuthState {
    pub nonce: String,
    pub action: String,
    pub user_id: Option<i32>,
}

pub enum OAuthCallbackResult {
    Login { token: String },
    LinkSuccess,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PublicProviderInfo {
    pub name: String,
    pub client_id: String,
    pub auth_url: String,
    pub scopes: Option<String>,
    pub icon_url: Option<String>,
}


// --- Error Handling ---

#[derive(Debug, thiserror::Error)]
pub enum OAuthServiceError {
    #[error("Database error: {0}")]
    DbErr(#[from] duckdb::Error),
    #[error("Pool error: {0}")]
    PoolError(#[from] r2d2::Error),
    #[error("Provider not found: {0}")]
    NotFound(String),
    #[error("Tokio join error: {0}")]
    JoinError(#[from] JoinError),
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Request error: {0}")]
    RequestError(#[from] reqwest::Error),
    #[error("OAuth Error: {0}")]
    OAuthError(String),
    #[error("User not found")]
    UserNotFound,
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("User service error: {0}")]
    UserServiceError(#[from] UserServiceError),
}

impl From<OAuthServiceError> for AppError {
    fn from(err: OAuthServiceError) -> Self {
        match err {
            OAuthServiceError::DbErr(e) => AppError::DatabaseError(e.to_string()),
            OAuthServiceError::PoolError(e) => AppError::DatabaseError(e.to_string()),
            OAuthServiceError::NotFound(name) => AppError::NotFound(format!("OAuth provider '{name}' not found")),
            OAuthServiceError::JoinError(e) => AppError::InternalServerError(e.to_string()),
            OAuthServiceError::EncryptionError(e) => AppError::InternalServerError(e),
            OAuthServiceError::JsonError(e) => AppError::InternalServerError(format!("JSON processing error: {e}")),
            OAuthServiceError::RequestError(e) => AppError::InternalServerError(format!("External request failed: {e}")),
            OAuthServiceError::OAuthError(e) => AppError::InternalServerError(e),
            OAuthServiceError::UserNotFound => AppError::UserNotFound,
            OAuthServiceError::InvalidInput(e) => AppError::InvalidInput(e),
            OAuthServiceError::Conflict(e) => AppError::Conflict(e),
            OAuthServiceError::UserServiceError(e) => AppError::DatabaseError(e.to_string()),
        }
    }
}

// --- Row Mappers ---

fn row_to_oauth2_provider(row: &Row) -> DuckDbResult<Oauth2Provider> {
    let user_info_mapping_str: Option<String> = row.get("user_info_mapping")?;
    let user_info_mapping = user_info_mapping_str
        .map(|s| serde_json::from_str(&s))
        .transpose()
        .map_err(|e| duckdb::Error::FromSqlConversionFailure(8, duckdb::types::Type::Text, Box::new(e)))?;

    Ok(Oauth2Provider {
        id: row.get("id")?,
        provider_name: row.get("provider_name")?,
        client_id: row.get("client_id")?,
        client_secret: row.get("client_secret")?,
        auth_url: row.get("auth_url")?,
        token_url: row.get("token_url")?,
        user_info_url: row.get("user_info_url")?,
        scopes: row.get("scopes")?,
        user_info_mapping,
        icon_url: row.get("icon_url")?,
        enabled: row.get("enabled")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

fn row_to_user_identity_provider(row: &Row) -> DuckDbResult<UserIdentityProvider> {
    Ok(UserIdentityProvider {
        id: row.get("id")?,
        user_id: row.get("user_id")?,
        provider_name: row.get("provider_name")?,
        provider_user_id: row.get("provider_user_id")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
    })
}

// --- Service Functions ---

pub async fn get_all_providers_for_admin(
    db_pool: DuckDbPool,
    encryption_key: &str,
) -> Result<Vec<AdminProviderInfo>, OAuthServiceError> {
    let key = encryption_key.to_string();
    tokio::task::spawn_blocking(move || {
        let conn = db_pool.get()?;
        let mut stmt = conn.prepare("SELECT * FROM oauth2_providers")?;
        let rows = stmt.query_map([], row_to_oauth2_provider)?;
        
        let mut admin_providers = Vec::new();
        for provider_result in rows {
            let mut provider = provider_result?;
            provider.client_secret = decrypt(&provider.client_secret, &key)
                .map_err(OAuthServiceError::EncryptionError)?;
            
            admin_providers.push(AdminProviderInfo {
                provider_name: provider.provider_name,
                client_id: provider.client_id,
                client_secret: provider.client_secret,
                auth_url: provider.auth_url,
                token_url: provider.token_url,
                user_info_url: provider.user_info_url,
                scopes: provider.scopes,
                icon_url: provider.icon_url,
                user_info_mapping: provider.user_info_mapping,
                enabled: provider.enabled,
            });
        }
        Ok(admin_providers)
    }).await?
}

pub async fn create_provider(
    db_pool: DuckDbPool,
    payload: ProviderUpsertPayload,
    encryption_key: &str,
) -> Result<Oauth2Provider, OAuthServiceError> {
    let key = encryption_key.to_string();
    tokio::task::spawn_blocking(move || {
        let conn = db_pool.get()?;
        let encrypted_secret = encrypt(&payload.client_secret, &key)
            .map_err(OAuthServiceError::EncryptionError)?;
        
        let user_info_mapping_str = serde_json::to_string(&payload.user_info_mapping)?;

        let now = Utc::now();
        let mut stmt = conn.prepare(
            "INSERT INTO oauth2_providers (provider_name, client_id, client_secret, auth_url, token_url, user_info_url, scopes, icon_url, user_info_mapping, enabled, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING *"
        )?;
        
        let provider = stmt.query_row(
            params![
                payload.provider_name,
                payload.client_id,
                encrypted_secret,
                payload.auth_url,
                payload.token_url,
                payload.user_info_url,
                payload.scopes,
                payload.icon_url,
                user_info_mapping_str,
                payload.enabled,
                now,
                now,
            ],
            row_to_oauth2_provider,
        )?;
        Ok(provider)
    }).await?
}

pub async fn update_provider(
    db_pool: DuckDbPool,
    provider_name: &str,
    payload: ProviderUpsertPayload,
    encryption_key: &str,
) -> Result<Oauth2Provider, OAuthServiceError> {
    let key = encryption_key.to_string();
    let name = provider_name.to_string();
    tokio::task::spawn_blocking(move || {
        let conn = db_pool.get()?;
        
        let encrypted_secret = if !payload.client_secret.is_empty() {
            Some(encrypt(&payload.client_secret, &key).map_err(OAuthServiceError::EncryptionError)?)
        } else {
            None
        };

        let user_info_mapping_str = serde_json::to_string(&payload.user_info_mapping)?;
        let now = Utc::now();

        let mut set_clauses = vec![
            "client_id = ?", "auth_url = ?", "token_url = ?", "user_info_url = ?",
            "scopes = ?", "icon_url = ?", "user_info_mapping = ?", "enabled = ?", "updated_at = ?"
        ];
        let mut params_vec: Vec<&dyn ToSql> = vec![
            &payload.client_id, &payload.auth_url, &payload.token_url, &payload.user_info_url,
            &payload.scopes, &payload.icon_url, &user_info_mapping_str, &payload.enabled, &now
        ];

        if let Some(secret) = &encrypted_secret {
            set_clauses.push("client_secret = ?");
            params_vec.push(secret);
        }
        
        let sql = format!(
            "UPDATE oauth2_providers SET {} WHERE provider_name = ? RETURNING *",
            set_clauses.join(", ")
        );
        params_vec.push(&name);

        let provider = conn.query_row(&sql, &params_vec[..], row_to_oauth2_provider)
            .map_err(|e| match e {
                duckdb::Error::QueryReturnedNoRows => OAuthServiceError::NotFound(name.clone()),
                _ => e.into(),
            })?;

        Ok(provider)
    }).await?
}

pub async fn delete_provider(
    db_pool: DuckDbPool,
    provider_name: &str,
) -> Result<(), OAuthServiceError> {
    let name = provider_name.to_string();
    tokio::task::spawn_blocking(move || {
        let conn = db_pool.get()?;
        let changes = conn.execute(
            "DELETE FROM oauth2_providers WHERE provider_name = ?",
            params![name.clone()],
        )?;
        if changes == 0 {
            Err(OAuthServiceError::NotFound(name))
        } else {
            Ok(())
        }
    }).await?
}

pub async fn get_all_providers(
    db_pool: DuckDbPool,
) -> Result<Vec<PublicProviderInfo>, OAuthServiceError> {
    tokio::task::spawn_blocking(move || {
        let conn = db_pool.get()?;
        let mut stmt = conn.prepare("SELECT provider_name, client_id, auth_url, scopes, icon_url FROM oauth2_providers WHERE enabled = TRUE")?;
        let rows = stmt.query_map([], |row| {
            Ok(PublicProviderInfo {
                name: row.get(0)?,
                client_id: row.get(1)?,
                auth_url: row.get(2)?,
                scopes: row.get(3)?,
                icon_url: row.get(4)?,
            })
        })?;
        
        let providers = rows.collect::<DuckDbResult<Vec<_>>>()?;
        Ok(providers)
    }).await?
}

pub async fn get_provider_config(
    db_pool: DuckDbPool,
    provider_name: &str,
    encryption_key: &str,
) -> Result<Oauth2Provider, OAuthServiceError> {
    let name = provider_name.to_string();
    let key = encryption_key.to_string();
    tokio::task::spawn_blocking(move || {
        let conn = db_pool.get()?;
        let mut provider = conn.query_row(
            "SELECT * FROM oauth2_providers WHERE provider_name = ?",
            params![name.clone()],
            row_to_oauth2_provider,
        ).map_err(|_| OAuthServiceError::NotFound(name))?;

        provider.client_secret = decrypt(&provider.client_secret, &key)
            .map_err(OAuthServiceError::EncryptionError)?;
        
        Ok(provider)
    }).await?
}

pub async fn handle_oauth_callback(
    db_pool: DuckDbPool,
    config: &ServerConfig,
    provider_name: &str,
    code: &str,
    state: &OAuthState,
) -> Result<OAuthCallbackResult, OAuthServiceError> {
    let provider_config = get_provider_config(db_pool.clone(), provider_name, &config.notification_encryption_key).await?;

    let redirect_uri = format!("{}/api/auth/{}/callback", &config.frontend_url, provider_name);
    let token_response = exchange_code_for_token(&provider_config, code, &redirect_uri).await?;
    let user_info = get_user_info(&provider_config, &token_response.access_token).await?;

    let mapping = provider_config.user_info_mapping.as_ref().and_then(|v| v.as_object())
        .ok_or_else(|| OAuthServiceError::OAuthError("User info mapping is missing or invalid.".to_string()))?;

    let provider_user_id = user_info.get(mapping.get("id_field").and_then(|v| v.as_str()).unwrap_or("id"))
        .and_then(|v| v.as_str().map(ToString::to_string).or_else(|| v.as_i64().map(|n| n.to_string())))
        .ok_or_else(|| OAuthServiceError::OAuthError("Could not extract provider user ID.".to_string()))?;

    let _username = user_info.get(mapping.get("username_field").and_then(|v| v.as_str()).unwrap_or("login"))
        .and_then(|v| v.as_str().map(ToString::to_string))
        .ok_or_else(|| OAuthServiceError::OAuthError("Could not extract username.".to_string()))?;

    if state.action == "link" {
        let user_id = state.user_id.ok_or(OAuthServiceError::InvalidInput("User ID missing for link action".to_string()))?;
        let pool = db_pool.clone();
        let p_name = provider_name.to_string();
        let p_user_id = provider_user_id.clone();

        tokio::task::spawn_blocking(move || {
            let conn = pool.get().map_err(OAuthServiceError::PoolError)?;
            let existing_link: Result<UserIdentityProvider, _> = conn.query_row(
                "SELECT * FROM user_identity_providers WHERE provider_name = ? AND provider_user_id = ?",
                params![p_name.clone(), p_user_id.clone()],
                row_to_user_identity_provider,
            );

            if let Ok(link) = existing_link {
                if link.user_id != user_id {
                    return Err(OAuthServiceError::Conflict("This external account is already linked to another user.".to_string()));
                }
            } else {
                let now = Utc::now();
                conn.execute(
                    "INSERT INTO user_identity_providers (user_id, provider_name, provider_user_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
                    params![user_id, p_name, p_user_id, now, now],
                )?;
            }
            Ok(())
        }).await??;
        Ok(OAuthCallbackResult::LinkSuccess)
    } else { // "login" action
        let pool = db_pool.clone();
        let p_name = provider_name.to_string();
        let p_user_id = provider_user_id.clone();

        let user_id: i32 = tokio::task::spawn_blocking(move || -> Result<i32, OAuthServiceError> {
            let conn = pool.get()?;
            let user_id = conn.query_row(
                "SELECT user_id FROM user_identity_providers WHERE provider_name = ? AND provider_user_id = ?",
                params![p_name, p_user_id],
                |row| row.get(0),
            )?;
            Ok(user_id)
        }).await??;

        let user_model = user_service::get_user_by_id(db_pool, user_id).await?
            .ok_or(OAuthServiceError::UserNotFound)?;

        let login_response = auth_service::create_jwt_for_user(&user_model, &config.jwt_secret)
            .map_err(|e| OAuthServiceError::OAuthError(e.to_string()))?;
            
        Ok(OAuthCallbackResult::Login { token: login_response.token })
    }
}


// --- External API Calls (unchanged) ---

pub async fn exchange_code_for_token(
    provider: &Oauth2Provider,
    code: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, OAuthServiceError> {
    let client = Client::new();
    let params = [
        ("client_id", &provider.client_id),
        ("client_secret", &provider.client_secret),
        ("code", &code.to_string()),
        ("redirect_uri", &redirect_uri.to_string()),
        ("grant_type", &"authorization_code".to_string()),
    ];

    let response = client
        .post(&provider.token_url)
        .form(&params)
        .header("Accept", "application/json")
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(OAuthServiceError::OAuthError(format!("Failed to get token: {error_text}")));
    }

    response.json::<TokenResponse>().await.map_err(Into::into)
}

pub async fn get_user_info(
    provider: &Oauth2Provider,
    access_token: &str,
) -> Result<JsonValue, OAuthServiceError> {
    let client = Client::new();
    client
        .get(&provider.user_info_url)
        .bearer_auth(access_token)
        .header("User-Agent", "node-nexus-agent")
        .send()
        .await?
        .json::<JsonValue>()
        .await
        .map_err(Into::into)
}