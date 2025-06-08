use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the different types of notification channel configurations.
/// This enum will be serialized to JSON and then encrypted before being stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ChannelConfig {
    Telegram {
        bot_token: String,
        chat_id: String,
    },
    Webhook {
        url: String,
        method: String, // "GET" or "POST"
        headers: Option<HashMap<String, String>>,
        body_template: Option<String>, // JSON template for POST requests
    },
}

/// Defines the structure for a field in a channel template for the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelTemplateField {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String, // e.g., "text", "textarea", "password"
    pub required: bool,
    pub label: String,
    pub help_text: Option<String>,
}

/// Defines the template for a channel type, used to dynamically generate UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelTemplate {
    pub channel_type: String,
    pub name: String,
    pub fields: Vec<ChannelTemplateField>,
}

/// API request body for creating a new notification channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateChannelRequest {
    pub name: String,
    pub channel_type: String, // "telegram" or "webhook"
    pub config: serde_json::Value, // The raw config JSON from the frontend
}

/// API request body for updating an existing notification channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateChannelRequest {
    pub name: Option<String>,
    pub config: Option<serde_json::Value>,
}

/// API response for a single notification channel.
/// Note: This does NOT include the sensitive config details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelResponse {
    pub id: i32,
    pub name: String,
    pub channel_type: String,
}

/// API request for sending a test notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestChannelRequest {
    pub message: Option<String>,
}