use async_trait::async_trait;
use reqwest::{header, Client, Method};
use std::collections::HashMap;
use tera::{Context, Tera};

use crate::notifications::models::ChannelConfig;
use super::{NotificationSender, SenderError};

/// A sender for pushing notifications via a custom webhook.
pub struct WebhookSender {
    client: Client,
}

impl WebhookSender {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

#[async_trait]
impl NotificationSender for WebhookSender {
    async fn send(
        &self,
        config: &ChannelConfig,
        message: &str, // This will be the body template for POST, or ignored for GET
        context: &HashMap<String, String>,
    ) -> Result<(), SenderError> {
        let (url, method, headers, body_template) = match config {
            ChannelConfig::Webhook { url, method, headers, body_template } => {
                (url, method, headers, body_template)
            }
            _ => {
                return Err(SenderError::InvalidConfiguration(
                    "Expected Webhook config, but found a different type.".to_string(),
                ));
            }
        };

        let http_method = match method.to_uppercase().as_str() {
            "POST" => Method::POST,
            "GET" => Method::GET,
            _ => {
                return Err(SenderError::InvalidConfiguration(format!(
                    "Unsupported HTTP method: {}",
                    method
                )));
            }
        };

        let mut request_builder = self.client.request(http_method, url);

        // Add headers
        if let Some(h) = headers {
            let mut header_map = header::HeaderMap::new();
            for (key, value) in h {
                let header_name = header::HeaderName::from_bytes(key.as_bytes())
                    .map_err(|e| SenderError::InvalidConfiguration(format!("Invalid header name: {}", e)))?;
                let header_value = header::HeaderValue::from_str(value)
                    .map_err(|e| SenderError::InvalidConfiguration(format!("Invalid header value: {}", e)))?;
                header_map.insert(header_name, header_value);
            }
            request_builder = request_builder.headers(header_map);
        }

        // Add body for POST requests
        if method.to_uppercase() == "POST" {
            let template = body_template.as_deref().unwrap_or(message);
            let mut tera_context = Context::new();
            for (key, value) in context {
                tera_context.insert(key, value);
            }
            
            let rendered_body = Tera::one_off(template, &tera_context, true)
                .map_err(|e| SenderError::TemplatingError(e.to_string()))?;

            request_builder = request_builder
                .header(header::CONTENT_TYPE, "application/json")
                .body(rendered_body);
        }

        let response = request_builder.send().await?;
        let status = response.status();

        if !status.is_success() {
            let error_body = response.text().await.unwrap_or_else(|_| "Failed to read error body".to_string());
            return Err(SenderError::SendFailed(format!(
                "Webhook returned non-success status: {}. Body: {}",
                status,
                error_body
            )));
        }

        Ok(())
    }
}