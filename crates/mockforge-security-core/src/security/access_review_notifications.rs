//! Notification system for access reviews
//!
//! This module provides notification capabilities for access review events,
//! including email, Slack, and other notification channels.

use crate::security::access_review::{AccessReview, UserReviewItem};
use crate::security::email::{EmailMessage, EmailService};
use crate::security::slack::{SlackMessage, SlackService};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info};

/// Notification channel types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationChannel {
    /// Email notifications
    Email,
    /// Slack notifications
    Slack,
    /// Webhook notifications
    Webhook,
    /// In-app notifications
    InApp,
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Whether notifications are enabled
    pub enabled: bool,
    /// Notification channels to use
    pub channels: Vec<NotificationChannel>,
    /// Recipient groups
    pub recipients: Vec<String>,
    /// Channel-specific configuration
    pub channel_config: HashMap<String, serde_json::Value>,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            channels: vec![NotificationChannel::Email],
            recipients: vec!["security_team".to_string(), "compliance_team".to_string()],
            channel_config: HashMap::new(),
        }
    }
}

/// Notification service for access reviews
pub struct AccessReviewNotificationService {
    config: NotificationConfig,
    /// Email service for sending email notifications
    email_service: Option<Arc<EmailService>>,
    /// Slack service for sending Slack notifications
    slack_service: Option<Arc<SlackService>>,
    /// HTTP client for webhook notifications
    webhook_client: Option<reqwest::Client>,
}

impl AccessReviewNotificationService {
    /// Create a new notification service
    pub fn new(config: NotificationConfig) -> Self {
        // Initialize email service if email notifications are enabled
        let email_service = if config.channels.contains(&NotificationChannel::Email) {
            Some(Arc::new(EmailService::from_env()))
        } else {
            None
        };

        // Initialize Slack service if Slack notifications are enabled
        let slack_service = if config.channels.contains(&NotificationChannel::Slack) {
            Some(Arc::new(SlackService::from_env()))
        } else {
            None
        };

        // Initialize webhook client if webhook notifications are enabled
        let webhook_client = if config.channels.contains(&NotificationChannel::Webhook) {
            Some(
                reqwest::Client::builder()
                    .timeout(Duration::from_secs(10))
                    .build()
                    .expect("Failed to create HTTP client for webhook notifications"),
            )
        } else {
            None
        };

        Self {
            config,
            email_service,
            slack_service,
            webhook_client,
        }
    }

    /// Create a new notification service with a custom email service
    pub fn with_email_service(
        config: NotificationConfig,
        email_service: Arc<EmailService>,
    ) -> Self {
        let slack_service = if config.channels.contains(&NotificationChannel::Slack) {
            Some(Arc::new(SlackService::from_env()))
        } else {
            None
        };

        let webhook_client = if config.channels.contains(&NotificationChannel::Webhook) {
            Some(
                reqwest::Client::builder()
                    .timeout(Duration::from_secs(10))
                    .build()
                    .expect("Failed to create HTTP client for webhook notifications"),
            )
        } else {
            None
        };

        Self {
            config,
            email_service: Some(email_service),
            slack_service,
            webhook_client,
        }
    }

    /// Send notification for a new review
    pub async fn notify_review_started(&self, review: &AccessReview) -> Result<(), String> {
        if !self.config.enabled {
            return Ok(());
        }

        let subject = format!("Access Review Started: {}", review.review_id);
        let message = format!(
            "A new {:?} review has been started.\n\n\
            Review ID: {}\n\
            Total Items: {}\n\
            Due Date: {}\n\
            Next Review: {}",
            review.review_type,
            review.review_id,
            review.total_items,
            review.due_date,
            review.next_review_date
        );

        self.send_notification(&subject, &message, &self.config.recipients).await
    }

    /// Send notification for pending approvals
    pub async fn notify_pending_approvals(
        &self,
        review: &AccessReview,
        items: &[UserReviewItem],
    ) -> Result<(), String> {
        if !self.config.enabled {
            return Ok(());
        }

        let pending_count = items.iter().filter(|i| i.status == "pending").count();

        if pending_count == 0 {
            return Ok(());
        }

        let subject = format!("Pending Access Review Approvals: {}", review.review_id);
        let message = format!(
            "There are {} pending approvals for review {}.\n\n\
            Review ID: {}\n\
            Due Date: {}\n\n\
            Please review and approve or revoke access for these users.",
            pending_count, review.review_id, review.review_id, review.due_date
        );

        self.send_notification(&subject, &message, &self.config.recipients).await
    }

    /// Send notification for auto-revocation
    pub async fn notify_auto_revocation(
        &self,
        review_id: &str,
        user_id: uuid::Uuid,
        reason: &str,
    ) -> Result<(), String> {
        if !self.config.enabled {
            return Ok(());
        }

        let subject = format!("Access Auto-Revoked: User {}", user_id);
        let message = format!(
            "User {} has been automatically revoked from review {}.\n\n\
            Reason: {}\n\n\
            This action was taken because the approval deadline was exceeded.",
            user_id, review_id, reason
        );

        self.send_notification(&subject, &message, &self.config.recipients).await
    }

    /// Send notification for review completion
    pub async fn notify_review_completed(&self, review: &AccessReview) -> Result<(), String> {
        if !self.config.enabled {
            return Ok(());
        }

        let subject = format!("Access Review Completed: {}", review.review_id);
        let message = format!(
            "Review {} has been completed.\n\n\
            Total Items: {}\n\
            Items Reviewed: {}\n\
            Actions Taken:\n\
            - Users Revoked: {}\n\
            - Permissions Reduced: {}\n\
            - MFA Enforced: {}",
            review.review_id,
            review.total_items,
            review.items_reviewed,
            review.actions_taken.users_revoked,
            review.actions_taken.permissions_reduced,
            review.actions_taken.mfa_enforced
        );

        self.send_notification(&subject, &message, &self.config.recipients).await
    }

    /// Send a notification through configured channels
    async fn send_notification(
        &self,
        subject: &str,
        message: &str,
        recipients: &[String],
    ) -> Result<(), String> {
        for channel in &self.config.channels {
            match channel {
                NotificationChannel::Email => {
                    self.send_email(subject, message, recipients).await?;
                }
                NotificationChannel::Slack => {
                    self.send_slack(subject, message, recipients).await?;
                }
                NotificationChannel::Webhook => {
                    self.send_webhook(subject, message, recipients).await?;
                }
                NotificationChannel::InApp => {
                    // In-app notifications would be stored in a database
                    debug!("In-app notification: {} - {}", subject, message);
                }
            }
        }

        Ok(())
    }

    /// Send email notification via email service
    async fn send_email(
        &self,
        subject: &str,
        message: &str,
        recipients: &[String],
    ) -> Result<(), String> {
        if recipients.is_empty() {
            debug!("No email recipients specified, skipping email notification");
            return Ok(());
        }

        // If email service is not available, log and return
        let email_service = match &self.email_service {
            Some(service) => service,
            None => {
                debug!("Email service not configured, logging notification instead");
                info!("Email notification (not sent): {} - {}", subject, message);
                return Ok(());
            }
        };

        // Convert plain text message to HTML (simple conversion)
        // Escape HTML special characters
        let html_escaped: String = message
            .chars()
            .map(|c| match c {
                '<' => "&lt;".to_string(),
                '>' => "&gt;".to_string(),
                '&' => "&amp;".to_string(),
                '"' => "&quot;".to_string(),
                '\'' => "&#x27;".to_string(),
                _ => c.to_string(),
            })
            .collect();

        let html_body = format!(
            "<html><body><pre style=\"font-family: sans-serif; white-space: pre-wrap;\">{}</pre></body></html>",
            html_escaped
        );

        // Send email to each recipient
        for recipient in recipients {
            let email_message = EmailMessage {
                to: recipient.clone(),
                subject: subject.to_string(),
                html_body: html_body.clone(),
                text_body: message.to_string(),
            };

            match email_service.send(email_message).await {
                Ok(()) => {
                    info!("Email notification sent successfully to {}", recipient);
                }
                Err(e) => {
                    error!("Failed to send email notification to {}: {}", recipient, e);
                    // Continue sending to other recipients even if one fails
                }
            }
        }

        Ok(())
    }

    /// Send Slack notification via Slack service
    async fn send_slack(
        &self,
        subject: &str,
        message: &str,
        recipients: &[String],
    ) -> Result<(), String> {
        if recipients.is_empty() {
            debug!("No Slack recipients specified, skipping Slack notification");
            return Ok(());
        }

        // If Slack service is not available, log and return
        let slack_service = match &self.slack_service {
            Some(service) => service,
            None => {
                debug!("Slack service not configured, logging notification instead");
                info!("Slack notification (not sent): {} - {}", subject, message);
                return Ok(());
            }
        };

        // Create Slack message with subject as title
        let slack_message = SlackMessage {
            channel: None, // Will use default channel or recipient-specific channel
            text: format!("{}\n\n{}", subject, message),
            title: Some(subject.to_string()),
            fields: Vec::new(),
        };

        // Send to each recipient (recipients are treated as channel names or user IDs)
        for recipient in recipients {
            let mut msg = slack_message.clone();
            msg.channel = Some(recipient.clone());

            match slack_service.send(msg).await {
                Ok(()) => {
                    info!("Slack notification sent successfully to {}", recipient);
                }
                Err(e) => {
                    error!("Failed to send Slack notification to {}: {}", recipient, e);
                    // Continue sending to other recipients even if one fails
                }
            }
        }

        Ok(())
    }

    /// Send webhook notification via HTTP POST
    async fn send_webhook(
        &self,
        subject: &str,
        message: &str,
        recipients: &[String],
    ) -> Result<(), String> {
        if recipients.is_empty() {
            debug!("No webhook URLs specified, skipping webhook notification");
            return Ok(());
        }

        // If webhook client is not available, log and return
        let webhook_client = match &self.webhook_client {
            Some(client) => client,
            None => {
                debug!("Webhook client not configured, logging notification instead");
                info!("Webhook notification (not sent): {} - {}", subject, message);
                return Ok(());
            }
        };

        // Build webhook payload
        let payload = json!({
            "subject": subject,
            "message": message,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "source": "mockforge_access_review",
        });

        // Get webhook URL from channel config or use recipients as URLs
        let webhook_urls = if let Some(webhook_url) = self
            .config
            .channel_config
            .get("webhook")
            .and_then(|v| v.get("url"))
            .and_then(|v| v.as_str())
        {
            vec![webhook_url.to_string()]
        } else {
            // Use recipients as webhook URLs
            recipients.to_vec()
        };

        // Send webhook to each URL
        for webhook_url in webhook_urls {
            match webhook_client
                .post(&webhook_url)
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .await
            {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        info!("Webhook notification sent successfully to {}", webhook_url);
                    } else {
                        let error_text = response.text().await.unwrap_or_default();
                        error!(
                            "Webhook notification failed to {} ({}): {}",
                            webhook_url, status, error_text
                        );
                        // Continue sending to other webhooks even if one fails
                    }
                }
                Err(e) => {
                    error!("Failed to send webhook notification to {}: {}", webhook_url, e);
                    // Continue sending to other webhooks even if one fails
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_notification_service() {
        let config = NotificationConfig::default();
        let service = AccessReviewNotificationService::new(config);

        // Test that notifications can be created (even if not actually sent)
        let result = service
            .send_notification("Test", "Test message", &["test@example.com".to_string()])
            .await;
        assert!(result.is_ok());
    }
}
