use async_trait::async_trait;
use std::collections::HashMap;
use thiserror::Error;

use super::models::ChannelConfig;

pub mod telegram;
pub mod webhook;

#[derive(Error, Debug)]
pub enum SenderError {
    #[error("Failed to send notification: {0}")]
    SendFailed(String),
    #[error("Invalid configuration for sender: {0}")]
    InvalidConfiguration(String),
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),
    #[error("Templating error: {0}")]
    TemplatingError(String),
}

/// A trait for sending notifications to a specific channel type.
/// All concrete sender implementations (e.g., Telegram, Webhook) must implement this trait.
#[async_trait]
pub trait NotificationSender {
    /// Sends a notification.
    ///
    /// # Arguments
    ///
    /// * `config` - The specific, decrypted configuration for this channel.
    /// * `message` - The message to send. For channels that support templating,
    ///             this could be a template string.
    /// * `context` - A map of key-value pairs for template rendering (e.g., "vps_name": "server1").
    async fn send(
        &self,
        config: &ChannelConfig,
        message: &str,
        context: &HashMap<String, String>,
    ) -> Result<(), SenderError>;
}
