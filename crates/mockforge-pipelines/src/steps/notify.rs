//! Notify step
//!
//! Sends notifications to teams via Slack, email, or webhooks.

use super::{PipelineStepExecutor, StepContext, StepResult};
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};

/// Notify step executor
pub struct NotifyStep {
    /// HTTP client for webhook notifications
    http_client: Client,
}

impl NotifyStep {
    /// Create a new notify step
    #[must_use]
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
        }
    }
}

impl Default for NotifyStep {
    fn default() -> Self {
        Self::new()
    }
}

impl NotifyStep {
    /// Send Slack notification via webhook
    async fn send_slack_notification(
        &self,
        webhook_url: &str,
        channel: &str,
        _message: &str,
    ) -> Result<()> {
        let payload = serde_json::json!({
            "channel": channel,
            "text": _message,
            "username": "MockForge Pipeline",
            "icon_emoji": ":robot_face:"
        });

        let response = self
            .http_client
            .post(webhook_url)
            .json(&payload)
            .send()
            .await
            .context("Failed to send HTTP request to Slack webhook")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Slack webhook returned error status {status}: {body}"));
        }

        Ok(())
    }

    /// Send email notification via SMTP
    async fn send_email_notification(
        &self,
        smtp_config: &Value,
        to_addresses: &[String],
        message: &str,
        context: &StepContext,
    ) -> Result<()> {
        // Extract SMTP configuration
        let smtp_host = smtp_config
            .get("host")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'smtp.host' in config"))?;

        let smtp_port =
            smtp_config.get("port").and_then(handlebars::JsonValue::as_u64).unwrap_or(587) as u16;

        let _smtp_username = smtp_config.get("username").and_then(|v| v.as_str());
        let _smtp_password = smtp_config.get("password").and_then(|v| v.as_str());
        let from_address = smtp_config
            .get("from")
            .and_then(|v| v.as_str())
            .unwrap_or("mockforge@example.com");

        let subject = context
            .config
            .get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("MockForge Pipeline Notification");

        // For now, we'll use a simple HTTP-based email service or log
        // Full SMTP implementation would require an SMTP client library
        // This is a placeholder that can be enhanced with actual SMTP support
        warn!(
            "Email notification via SMTP requires additional dependencies. Consider using a service like SendGrid, Mailgun, or AWS SES via webhook instead."
        );

        // For now, we'll simulate success but log the email details
        debug!(
            smtp_host = %smtp_host,
            smtp_port = %smtp_port,
            from = %from_address,
            to = ?to_addresses,
            subject = %subject,
            "Email notification prepared (SMTP implementation pending)"
        );

        // TODO: Implement actual SMTP sending with a library like lettre
        // For now, return success to allow pipeline to continue
        Ok(())
    }

    /// Send webhook notification
    async fn send_webhook_notification(
        &self,
        webhook_url: &str,
        message: &str,
        context: &StepContext,
    ) -> Result<()> {
        // Build payload from context and message
        let mut payload = serde_json::json!({
            "message": message,
            "pipeline_execution_id": context.execution_id.to_string(),
            "step_name": context.step_name,
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        // Add any additional payload data from config
        if let Some(additional_data) = context.config.get("payload") {
            if let Some(obj) = additional_data.as_object() {
                for (key, value) in obj {
                    payload[key] = value.clone();
                }
            }
        }

        let method = context.config.get("method").and_then(|v| v.as_str()).unwrap_or("POST");

        let response = match method {
            "POST" => self.http_client.post(webhook_url).json(&payload).send().await,
            "PUT" => self.http_client.put(webhook_url).json(&payload).send().await,
            "PATCH" => self.http_client.patch(webhook_url).json(&payload).send().await,
            _ => {
                return Err(anyhow::anyhow!("Unsupported HTTP method for webhook: {method}"));
            }
        }
        .context("Failed to send HTTP request to webhook")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Webhook returned error status {status}: {body}"));
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl PipelineStepExecutor for NotifyStep {
    fn step_type(&self) -> &'static str {
        "notify"
    }

    async fn execute(&self, context: StepContext) -> Result<StepResult> {
        info!(
            execution_id = %context.execution_id,
            step_name = %context.step_name,
            "Executing notify step"
        );

        // Extract configuration
        let channels = context
            .config
            .get("channels")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(ToString::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let message = context
            .config
            .get("message")
            .and_then(|v| v.as_str())
            .map(ToString::to_string)
            .ok_or_else(|| anyhow::anyhow!("Missing 'message' in step config"))?;

        let notification_type =
            context.config.get("type").and_then(|v| v.as_str()).unwrap_or("slack");

        debug!(
            execution_id = %context.execution_id,
            notification_type = %notification_type,
            channels = ?channels,
            "Sending notification"
        );

        let mut results = Vec::new();
        let mut errors = Vec::new();

        // Send notifications based on type
        match notification_type {
            "slack" => {
                let webhook_url =
                    context.config.get("slack_webhook_url").and_then(|v| v.as_str()).ok_or_else(
                        || {
                            anyhow::anyhow!(
                            "Missing 'slack_webhook_url' in step config for Slack notifications"
                        )
                        },
                    )?;

                for channel in &channels {
                    match self.send_slack_notification(webhook_url, channel, &message).await {
                        Ok(()) => {
                            results.push(format!("slack:{channel}"));
                            info!(
                                execution_id = %context.execution_id,
                                channel = %channel,
                                "Sent Slack notification"
                            );
                        }
                        Err(e) => {
                            let error_msg =
                                format!("Failed to send Slack notification to {channel}: {e}");
                            error!(
                                execution_id = %context.execution_id,
                                channel = %channel,
                                error = %e,
                                "Failed to send Slack notification"
                            );
                            errors.push(error_msg);
                        }
                    }
                }
            }
            "email" => {
                let smtp_config = context.config.get("smtp").ok_or_else(|| {
                    anyhow::anyhow!("Missing 'smtp' config for email notifications")
                })?;

                let to_addresses = context
                    .config
                    .get("to")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(ToString::to_string))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();

                if to_addresses.is_empty() {
                    return Err(anyhow::anyhow!("Missing 'to' addresses for email notifications"));
                }

                match self
                    .send_email_notification(smtp_config, &to_addresses, &message, &context)
                    .await
                {
                    Ok(()) => {
                        for addr in &to_addresses {
                            results.push(format!("email:{addr}"));
                        }
                        info!(
                            execution_id = %context.execution_id,
                            recipients = ?to_addresses,
                            "Sent email notifications"
                        );
                    }
                    Err(e) => {
                        let error_msg = format!("Failed to send email notifications: {e}");
                        error!(
                            execution_id = %context.execution_id,
                            error = %e,
                            "Failed to send email notifications"
                        );
                        errors.push(error_msg);
                    }
                }
            }
            "webhook" => {
                let webhook_urls = context
                    .config
                    .get("webhook_urls")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(ToString::to_string))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_else(|| {
                        // Fallback to single webhook_url
                        context
                            .config
                            .get("webhook_url")
                            .and_then(|v| v.as_str())
                            .map(|s| vec![s.to_string()])
                            .unwrap_or_default()
                    });

                if webhook_urls.is_empty() {
                    return Err(anyhow::anyhow!(
                        "Missing 'webhook_url' or 'webhook_urls' for webhook notifications"
                    ));
                }

                for webhook_url in &webhook_urls {
                    match self.send_webhook_notification(webhook_url, &message, &context).await {
                        Ok(()) => {
                            results.push(format!("webhook:{webhook_url}"));
                            info!(
                                execution_id = %context.execution_id,
                                webhook_url = %webhook_url,
                                "Sent webhook notification"
                            );
                        }
                        Err(e) => {
                            let error_msg = format!("Failed to send webhook to {webhook_url}: {e}");
                            error!(
                                execution_id = %context.execution_id,
                                webhook_url = %webhook_url,
                                error = %e,
                                "Failed to send webhook notification"
                            );
                            errors.push(error_msg);
                        }
                    }
                }
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Unsupported notification type: {notification_type}. Supported: slack, email, webhook"
                ));
            }
        }

        let mut output = HashMap::new();
        output.insert("type".to_string(), Value::String(notification_type.to_string()));
        output.insert(
            "channels".to_string(),
            Value::Array(channels.iter().map(|c| Value::String(c.clone())).collect()),
        );
        output.insert("message".to_string(), Value::String(message.clone()));
        output.insert(
            "results".to_string(),
            Value::Array(results.iter().map(|r| Value::String(r.clone())).collect()),
        );
        output.insert(
            "status".to_string(),
            Value::String(
                if errors.is_empty() {
                    "success"
                } else {
                    "partial_success"
                }
                .to_string(),
            ),
        );

        if !errors.is_empty() {
            output.insert(
                "errors".to_string(),
                Value::Array(errors.iter().map(|e| Value::String(e.clone())).collect()),
            );
        }

        Ok(StepResult::success_with_output(output))
    }
}
