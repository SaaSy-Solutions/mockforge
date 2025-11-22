//! Slack notification service for security notifications
//!
//! Supports multiple Slack integration methods:
//! - Incoming Webhooks (simpler, just POST to webhook URL)
//! - Web API (chat.postMessage, requires bot token)
//! - Disabled (logs only, for development/testing)

use anyhow::{Context, Result};
use serde::Serialize;
use std::time::Duration;
use tracing::{debug, error, info};

/// Slack integration method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlackMethod {
    /// Incoming webhook (POST to webhook URL)
    Webhook,
    /// Web API (chat.postMessage with bot token)
    WebApi,
    /// Slack disabled (logs only)
    Disabled,
}

impl SlackMethod {
    /// Parse method from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "webhook" | "incoming_webhook" => SlackMethod::Webhook,
            "webapi" | "web_api" | "api" => SlackMethod::WebApi,
            _ => SlackMethod::Disabled,
        }
    }
}

/// Slack configuration
#[derive(Debug, Clone)]
pub struct SlackConfig {
    /// Integration method to use
    pub method: SlackMethod,
    /// Webhook URL (for incoming webhook method)
    pub webhook_url: Option<String>,
    /// Bot token (for Web API method)
    pub bot_token: Option<String>,
    /// Default channel to send messages to (for Web API)
    pub default_channel: Option<String>,
}

impl SlackConfig {
    /// Create Slack config from environment variables
    pub fn from_env() -> Self {
        let method = std::env::var("SLACK_METHOD")
            .unwrap_or_else(|_| "disabled".to_string());

        Self {
            method: SlackMethod::from_str(&method),
            webhook_url: std::env::var("SLACK_WEBHOOK_URL").ok(),
            bot_token: std::env::var("SLACK_BOT_TOKEN").ok(),
            default_channel: std::env::var("SLACK_DEFAULT_CHANNEL")
                .or_else(|_| std::env::var("SLACK_CHANNEL"))
                .ok(),
        }
    }
}

/// Slack message
#[derive(Debug, Clone)]
pub struct SlackMessage {
    /// Channel to send message to (optional, uses default if not provided)
    pub channel: Option<String>,
    /// Message text
    pub text: String,
    /// Message title/subject (optional)
    pub title: Option<String>,
    /// Additional fields for rich formatting (optional)
    pub fields: Vec<(String, String)>,
}

/// Slack service for sending notifications
pub struct SlackService {
    config: SlackConfig,
    client: reqwest::Client,
}

impl SlackService {
    /// Create a new Slack service
    pub fn new(config: SlackConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client for Slack service");

        Self { config, client }
    }

    /// Create Slack service from environment variables
    pub fn from_env() -> Self {
        Self::new(SlackConfig::from_env())
    }

    /// Send a Slack message
    pub async fn send(&self, message: SlackMessage) -> Result<()> {
        match &self.config.method {
            SlackMethod::Webhook => self.send_via_webhook(message).await,
            SlackMethod::WebApi => self.send_via_webapi(message).await,
            SlackMethod::Disabled => {
                info!("Slack disabled, would send: '{}'", message.text);
                debug!("Slack message details: {:?}", message);
                Ok(())
            }
        }
    }

    /// Send message to multiple channels/recipients
    pub async fn send_to_multiple(&self, message: SlackMessage, recipients: &[String]) -> Result<()> {
        let mut errors = Vec::new();

        for recipient in recipients {
            let mut msg = message.clone();
            msg.channel = Some(recipient.clone());

            match self.send(msg).await {
                Ok(()) => {
                    debug!("Slack message sent successfully to {}", recipient);
                }
                Err(e) => {
                    let error_msg = format!("Failed to send Slack message to {}: {}", recipient, e);
                    error!("{}", error_msg);
                    errors.push(error_msg);
                }
            }
        }

        if !errors.is_empty() {
            anyhow::bail!("Failed to send Slack messages to some recipients: {}", errors.join("; "));
        }

        Ok(())
    }

    /// Send message via Slack Incoming Webhook
    async fn send_via_webhook(&self, message: SlackMessage) -> Result<()> {
        let webhook_url = self.config.webhook_url.as_ref()
            .context("Slack webhook requires SLACK_WEBHOOK_URL environment variable")?;

        #[derive(Serialize)]
        struct SlackWebhookPayload {
            text: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            channel: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            attachments: Option<Vec<SlackAttachment>>,
        }

        #[derive(Serialize)]
        struct SlackAttachment {
            #[serde(skip_serializing_if = "Option::is_none")]
            title: Option<String>,
            text: String,
            color: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            fields: Option<Vec<SlackField>>,
        }

        #[derive(Serialize)]
        struct SlackField {
            title: String,
            value: String,
            short: bool,
        }

        let mut attachments = Vec::new();
        let mut attachment = SlackAttachment {
            title: message.title.clone(),
            text: message.text.clone(),
            color: "#36a64f".to_string(), // Green color for notifications
            fields: None,
        };

        if !message.fields.is_empty() {
            attachment.fields = Some(
                message.fields
                    .iter()
                    .map(|(title, value)| SlackField {
                        title: title.clone(),
                        value: value.clone(),
                        short: true,
                    })
                    .collect(),
            );
        }

        attachments.push(attachment);

        let payload = SlackWebhookPayload {
            text: message.text.clone(),
            channel: message.channel.clone(),
            attachments: Some(attachments),
        };

        let response = self.client
            .post(webhook_url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to send Slack message via webhook")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Slack webhook error ({}): {}", status, error_text);
        }

        info!("Slack message sent via webhook");
        Ok(())
    }

    /// Send message via Slack Web API (chat.postMessage)
    async fn send_via_webapi(&self, message: SlackMessage) -> Result<()> {
        let bot_token = self.config.bot_token.as_ref()
            .context("Slack Web API requires SLACK_BOT_TOKEN environment variable")?;

        let channel = message.channel
            .as_ref()
            .or(self.config.default_channel.as_ref())
            .context("Slack Web API requires channel (set SLACK_DEFAULT_CHANNEL or provide in message)")?;

        #[derive(Serialize)]
        struct SlackApiPayload {
            channel: String,
            text: String,
            #[serde(skip_serializing_if = "Option::is_none")]
            blocks: Option<Vec<serde_json::Value>>,
        }

        // Build rich message blocks if we have title or fields
        let mut blocks = Vec::new();
        if let Some(ref title) = message.title {
            blocks.push(serde_json::json!({
                "type": "header",
                "text": {
                    "type": "plain_text",
                    "text": title
                }
            }));
        }

        blocks.push(serde_json::json!({
            "type": "section",
            "text": {
                "type": "mrkdwn",
                "text": message.text
            }
        }));

        if !message.fields.is_empty() {
            let fields: Vec<serde_json::Value> = message.fields
                .iter()
                .map(|(title, value)| {
                    serde_json::json!({
                        "type": "mrkdwn",
                        "text": format!("*{}:*\n{}", title, value)
                    })
                })
                .collect();

            blocks.push(serde_json::json!({
                "type": "section",
                "fields": fields
            }));
        }

        let payload = SlackApiPayload {
            channel: channel.clone(),
            text: message.text.clone(),
            blocks: if blocks.is_empty() { None } else { Some(blocks) },
        };

        let response = self.client
            .post("https://slack.com/api/chat.postMessage")
            .header("Authorization", format!("Bearer {}", bot_token))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .context("Failed to send Slack message via Web API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Slack Web API error ({}): {}", status, error_text);
        }

        // Check Slack API response for errors
        let api_response: serde_json::Value = response.json().await
            .context("Failed to parse Slack API response")?;

        if let Some(ok) = api_response.get("ok").and_then(|v| v.as_bool()) {
            if !ok {
                let error_msg = api_response.get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");
                anyhow::bail!("Slack API returned error: {}", error_msg);
            }
        }

        info!("Slack message sent via Web API to channel {}", channel);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slack_method_from_str() {
        assert_eq!(SlackMethod::from_str("webhook"), SlackMethod::Webhook);
        assert_eq!(SlackMethod::from_str("incoming_webhook"), SlackMethod::Webhook);
        assert_eq!(SlackMethod::from_str("webapi"), SlackMethod::WebApi);
        assert_eq!(SlackMethod::from_str("web_api"), SlackMethod::WebApi);
        assert_eq!(SlackMethod::from_str("api"), SlackMethod::WebApi);
        assert_eq!(SlackMethod::from_str("disabled"), SlackMethod::Disabled);
        assert_eq!(SlackMethod::from_str("unknown"), SlackMethod::Disabled);
    }

    #[tokio::test]
    async fn test_slack_service_disabled() {
        let config = SlackConfig {
            method: SlackMethod::Disabled,
            webhook_url: None,
            bot_token: None,
            default_channel: None,
        };

        let service = SlackService::new(config);
        let message = SlackMessage {
            channel: None,
            text: "Test message".to_string(),
            title: None,
            fields: Vec::new(),
        };

        // Should not fail when disabled
        let result = service.send(message).await;
        assert!(result.is_ok());
    }
}
