use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;
use std::collections::HashMap;

use super::{NotificationSender, SenderError};
use crate::notifications::models::ChannelConfig;

/// A sender for pushing notifications via the Telegram Bot API.
pub struct TelegramSender {
    client: Client,
}

impl Default for TelegramSender {
    fn default() -> Self {
        Self::new()
    }
}

impl TelegramSender {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Escapes text for Telegram MarkdownV2.
    /// Characters to escape: _ * [ ] ( ) ~ ` > # + - = | { } . !
    fn escape_markdown_v2(&self, text: &str) -> String {
        let mut escaped_text = String::with_capacity(text.len());
        for char_to_escape in text.chars() {
            match char_to_escape {
                '_' | '*' | '[' | ']' | '(' | ')' | '~' | '`' | '>' | '#' | '+' | '-' | '='
                | '|' | '{' | '}' | '.' | '!' => {
                    escaped_text.push('\\');
                    escaped_text.push(char_to_escape);
                }
                _ => {
                    escaped_text.push(char_to_escape);
                }
            }
        }
        escaped_text
    }
}

#[derive(Serialize)]
struct TelegramMessage<'a> {
    chat_id: &'a str,
    text: &'a str,
    parse_mode: &'a str,
}

#[async_trait]
impl NotificationSender for TelegramSender {
    async fn send(
        &self,
        config: &ChannelConfig,
        message: &str,
        _context: &HashMap<String, String>, // Telegram doesn't use templating in this basic version
    ) -> Result<(), SenderError> {
        let (bot_token, chat_id) = match config {
            ChannelConfig::Telegram { bot_token, chat_id } => (bot_token, chat_id),
            _ => {
                return Err(SenderError::InvalidConfiguration(
                    "Expected Telegram config, but found a different type.".to_string(),
                ));
            }
        };

        let api_url = format!("https://api.telegram.org/bot{bot_token}/sendMessage");

        let escaped_message = self.escape_markdown_v2(message);
        let payload = TelegramMessage {
            chat_id,
            text: &escaped_message,
            parse_mode: "MarkdownV2",
        };

        let response = self.client.post(&api_url).json(&payload).send().await?;
        let status = response.status();

        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error body".to_string());
            return Err(SenderError::SendFailed(format!(
                "Telegram API returned non-success status: {status}. Body: {error_body}"
            )));
        }

        Ok(())
    }
}
