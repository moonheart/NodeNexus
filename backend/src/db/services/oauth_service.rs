// backend/src/db/services/oauth_service.rs

use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, ActiveModelTrait, Set};
use crate::db::entities::{oauth2_provider, prelude::Oauth2Provider, user, user_identity_provider};
use crate::web::error::AppError;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use crate::services::auth_service;
use crate::server::config::ServerConfig;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProviderUpsertPayload {
    pub provider_name: String,
    pub client_id: String,
    pub client_secret: String, // Plain text, will be encrypted by the service
    pub auth_url: String,
    pub token_url: String,
    pub user_info_url: String,
    pub scopes: Option<String>,
    pub icon_url: Option<String>,
    pub user_info_mapping: serde_json::Value,
    pub enabled: bool,
}

#[derive(Deserialize, Debug)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub scope: Option<String>,
}

pub async fn get_provider_config(
    db: &DatabaseConnection,
    provider_name: &str,
    encryption_key: &str,
) -> Result<oauth2_provider::Model, AppError> {
    let mut provider = Oauth2Provider::find()
        .filter(oauth2_provider::Column::ProviderName.eq(provider_name))
        .one(db)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("OAuth provider '{}' not found", provider_name)))?;

    provider
        .decrypt_client_secret(encryption_key)
        .map_err(AppError::InternalServerError)?;

    Ok(provider)
}

pub async fn exchange_code_for_token(
    provider: &oauth2_provider::Model,
    code: &str,
    redirect_uri: &str,
) -> Result<TokenResponse, AppError> {
    let client = Client::new();
    let params = [
        ("client_id", &provider.client_id),
        ("client_secret", &provider.client_secret), // Already decrypted from get_provider_config
        ("code", &code.to_string()),
        ("redirect_uri", &redirect_uri.to_string()),
        ("grant_type", &"authorization_code".to_string()),
    ];

    let response = client.post(&provider.token_url)
        .form(&params)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to send token request: {}", e)))?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        return Err(AppError::InternalServerError(format!("Failed to get token: {}", error_text)));
    }

    response.json::<TokenResponse>()
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to parse token response: {}", e)))
}

pub async fn get_user_info(
    provider: &oauth2_provider::Model,
    access_token: &str,
) -> Result<serde_json::Value, AppError> {
    let client = Client::new();
    client.get(&provider.user_info_url)
        .bearer_auth(access_token)
        .header("User-Agent", "mjjer-agent") // Some providers require a User-Agent
        .send()
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to send user info request: {}", e)))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| AppError::InternalServerError(format!("Failed to parse user info response: {}", e)))
}

// This struct will be used in the route handler
#[derive(Serialize, Deserialize, Debug)]
pub struct OAuthState {
    pub nonce: String,
    pub action: String,
    pub user_id: Option<i32>, // Only present for 'link' action
}

// The result of a successful callback, to be handled by the route
pub enum OAuthCallbackResult {
    Login { token: String },
    LinkSuccess,
}

pub async fn handle_oauth_callback(
    db: &DatabaseConnection,
    config: &ServerConfig,
    provider_name: &str,
    code: &str,
    state: &OAuthState,
) -> Result<OAuthCallbackResult, AppError> {
    let provider_config = get_provider_config(db, provider_name, &config.notification_encryption_key).await?;
    
    let redirect_uri = format!("{}/api/auth/{}/callback", &config.frontend_url, provider_name);

    let token_response = exchange_code_for_token(&provider_config, code, &redirect_uri).await?;

    let user_info = get_user_info(&provider_config, &token_response.access_token).await?;

    let mapping = provider_config.user_info_mapping.as_ref()
        .and_then(|v| v.as_object())
        .ok_or_else(|| AppError::InternalServerError("User info mapping is missing or invalid.".to_string()))?;

    let provider_user_id = user_info.get(mapping.get("id_field").and_then(|v| v.as_str()).unwrap_or("id"))
        .and_then(|v| v.as_str().map(ToString::to_string).or_else(|| v.as_i64().map(|n| n.to_string())))
        .ok_or_else(|| AppError::InternalServerError("Could not extract provider user ID.".to_string()))?;

    let username = user_info.get(mapping.get("username_field").and_then(|v| v.as_str()).unwrap_or("login"))
        .and_then(|v| v.as_str().map(ToString::to_string))
        .ok_or_else(|| AppError::InternalServerError("Could not extract username.".to_string()))?;

    if state.action == "link" {
        let user_id = state.user_id.ok_or(AppError::InvalidInput("User ID missing for link action".to_string()))?;

        let existing_link = user_identity_provider::Entity::find()
            .filter(user_identity_provider::Column::ProviderName.eq(provider_name))
            .filter(user_identity_provider::Column::ProviderUserId.eq(&provider_user_id))
            .one(db).await?;

        if let Some(link) = existing_link {
            if link.user_id != user_id {
                return Err(AppError::Conflict("This external account is already linked to another user.".to_string()));
            }
        } else {
            let new_identity = user_identity_provider::ActiveModel {
                user_id: Set(user_id),
                provider_name: Set(provider_name.to_string()),
                provider_user_id: Set(provider_user_id),
                ..Default::default()
            };
            new_identity.insert(db).await?;
        }
        Ok(OAuthCallbackResult::LinkSuccess)
    } else { // "login" action
        let identity = user_identity_provider::Entity::find()
            .filter(user_identity_provider::Column::ProviderName.eq(provider_name))
            .filter(user_identity_provider::Column::ProviderUserId.eq(&provider_user_id))
            .one(db).await?;

        let user_model = if let Some(identity) = identity {
            user::Entity::find_by_id(identity.user_id).one(db).await?
                .ok_or(AppError::UserNotFound)?
        } else {
            // Don't create a new user on login. The account must be linked first.
            return Err(AppError::Unauthorized("该外部帐户未关联。请先使用您的用户名和密码登录以关联您的帐户。".to_string()));
        };

        let login_response = auth_service::create_jwt_for_user(&user_model, &config.jwt_secret)?;
        Ok(OAuthCallbackResult::Login { token: login_response.token })
    }
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

pub async fn get_all_providers(
    db: &DatabaseConnection,
) -> Result<Vec<PublicProviderInfo>, AppError> {
    Oauth2Provider::find()
        .all(db)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))
        .map(|providers| {
            providers
                .into_iter()
                .map(|p| PublicProviderInfo {
                    name: p.provider_name,
                    client_id: p.client_id,
                    auth_url: p.auth_url,
                    scopes: p.scopes,
                    icon_url: p.icon_url,
                })
                .collect()
        })
}
#[derive(Serialize, Debug)]
pub struct AdminProviderInfo {
    pub provider_name: String,
    pub client_id: String,
    pub client_secret: String, // Decrypted
    pub auth_url: String,
    pub token_url: String,
    pub user_info_url: String,
    pub scopes: Option<String>,
    pub icon_url: Option<String>,
    pub user_info_mapping: Option<serde_json::Value>,
    pub enabled: bool,
}

pub async fn get_all_providers_for_admin(
    db: &DatabaseConnection,
    encryption_key: &str,
) -> Result<Vec<AdminProviderInfo>, AppError> {
    let mut providers = Oauth2Provider::find()
        .all(db)
        .await
        .map_err(|e| AppError::DatabaseError(e.to_string()))?;

    for provider in &mut providers {
        provider
            .decrypt_client_secret(encryption_key)
            .map_err(AppError::InternalServerError)?;
    }

    let admin_providers = providers
        .into_iter()
        .map(|p| AdminProviderInfo {
            provider_name: p.provider_name,
            client_id: p.client_id,
            client_secret: p.client_secret, // Now decrypted
            auth_url: p.auth_url,
            token_url: p.token_url,
            user_info_url: p.user_info_url,
            scopes: p.scopes,
            icon_url: p.icon_url,
            user_info_mapping: p.user_info_mapping,
            enabled: p.enabled,
        })
        .collect();

    Ok(admin_providers)
}

pub async fn create_provider(
    db: &DatabaseConnection,
    payload: ProviderUpsertPayload,
    encryption_key: &str,
) -> Result<oauth2_provider::Model, AppError> {
    let mut new_provider = oauth2_provider::ActiveModel {
        provider_name: Set(payload.provider_name),
        client_id: Set(payload.client_id),
        client_secret: Set(payload.client_secret), // Will be encrypted next
        auth_url: Set(payload.auth_url),
        token_url: Set(payload.token_url),
        user_info_url: Set(payload.user_info_url),
        scopes: Set(payload.scopes),
        icon_url: Set(payload.icon_url),
        user_info_mapping: Set(Some(payload.user_info_mapping)),
        enabled: Set(payload.enabled),
        ..Default::default()
    };

    new_provider.encrypt_client_secret(encryption_key)
        .map_err(AppError::InternalServerError)?;

    let provider_model = new_provider.insert(db).await?;
    Ok(provider_model)
}
pub async fn update_provider(
    db: &DatabaseConnection,
    provider_name: &str,
    payload: ProviderUpsertPayload,
    encryption_key: &str,
) -> Result<oauth2_provider::Model, AppError> {
    let provider = Oauth2Provider::find()
        .filter(oauth2_provider::Column::ProviderName.eq(provider_name))
        .one(db)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("OAuth provider '{}' not found", provider_name)))?;

    let mut active_provider: oauth2_provider::ActiveModel = provider.into();

    active_provider.client_id = Set(payload.client_id);
    active_provider.auth_url = Set(payload.auth_url);
    active_provider.token_url = Set(payload.token_url);
    active_provider.user_info_url = Set(payload.user_info_url);
    active_provider.scopes = Set(payload.scopes);
    active_provider.icon_url = Set(payload.icon_url);
    active_provider.user_info_mapping = Set(Some(payload.user_info_mapping));
    active_provider.enabled = Set(payload.enabled);

    // Only update and encrypt the secret if it's provided and not empty
    if !payload.client_secret.is_empty() {
        active_provider.client_secret = Set(payload.client_secret);
        active_provider.encrypt_client_secret(encryption_key)
            .map_err(AppError::InternalServerError)?;
    }

    let updated_provider = active_provider.update(db).await?;
    Ok(updated_provider)
}
pub async fn delete_provider(
    db: &DatabaseConnection,
    provider_name: &str,
) -> Result<(), AppError> {
    let delete_result = Oauth2Provider::delete_many()
        .filter(oauth2_provider::Column::ProviderName.eq(provider_name))
        .exec(db)
        .await?;

    if delete_result.rows_affected == 0 {
        Err(AppError::NotFound(format!("OAuth provider '{}' not found", provider_name)))
    } else {
        Ok(())
    }
}