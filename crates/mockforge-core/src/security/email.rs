//! Email service for security notifications
//!
//! Supports multiple email providers via HTTP APIs:
//! - Postmark (via API)
//! - Brevo (via API)
//! - SendGrid (via API)
//! - Disabled (logs only, for development/testing)

use anyhow::{Context, Result};
use serde::Serialize;
use std::time::Duration;
use tracing::{debug, error, info};

/// Email provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmailProvider {
    /// Postmark email service
    Postmark,
    /// Brevo (formerly Sendinblue) email service
    Brevo,
    /// SendGrid email service
    SendGrid,
    /// Email disabled (logs only)
    Disabled,
}

impl EmailProvider {
    /// Parse provider from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "postmark" => EmailProvider::Postmark,
            "brevo" | "sendinblue" => EmailProvider::Brevo,
            "sendgrid" => EmailProvider::SendGrid,
            _ => EmailProvider::Disabled,
        }
    }
}

/// Email configuration
#[derive(Debug, Clone)]
pub struct EmailConfig {
    /// Email provider to use
    pub provider: EmailProvider,
    /// From email address
    pub from_email: String,
    /// From name
    pub from_name: String,
    /// API key for email provider
    pub api_key: Option<String>,
}

impl EmailConfig {
    /// Create email config from environment variables
    pub fn from_env() -> Self {
        let provider = std::env::var("EMAIL_PROVIDER").unwrap_or_else(|_| "disabled".to_string());

        Self {
            provider: EmailProvider::from_str(&provider),
            from_email: std::env::var("EMAIL_FROM")
                .unwrap_or_else(|_| "noreply@mockforge.dev".to_string()),
            from_name: std::env::var("EMAIL_FROM_NAME")
                .unwrap_or_else(|_| "MockForge Security".to_string()),
            api_key: std::env::var("EMAIL_API_KEY").ok(),
        }
    }
}

/// Email message
#[derive(Debug, Clone)]
pub struct EmailMessage {
    /// Recipient email address
    pub to: String,
    /// Email subject
    pub subject: String,
    /// HTML body content
    pub html_body: String,
    /// Plain text body content
    pub text_body: String,
}

/// Email service for sending notifications
pub struct EmailService {
    config: EmailConfig,
    client: reqwest::Client,
}

impl EmailService {
    /// Create a new email service
    pub fn new(config: EmailConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client for email service");

        Self { config, client }
    }

    /// Create email service from environment variables
    pub fn from_env() -> Self {
        Self::new(EmailConfig::from_env())
    }

    /// Send an email to a single recipient
    pub async fn send(&self, message: EmailMessage) -> Result<()> {
        match &self.config.provider {
            EmailProvider::Postmark => self.send_via_postmark(message).await,
            EmailProvider::Brevo => self.send_via_brevo(message).await,
            EmailProvider::SendGrid => self.send_via_sendgrid(message).await,
            EmailProvider::Disabled => {
                info!("Email disabled, would send: '{}' to {}", message.subject, message.to);
                debug!("Email body (text): {}", message.text_body);
                Ok(())
            }
        }
    }

    /// Send email to multiple recipients
    pub async fn send_to_multiple(
        &self,
        message: EmailMessage,
        recipients: &[String],
    ) -> Result<()> {
        let mut errors = Vec::new();

        for recipient in recipients {
            let mut msg = message.clone();
            msg.to = recipient.clone();

            match self.send(msg).await {
                Ok(()) => {
                    debug!("Email sent successfully to {}", recipient);
                }
                Err(e) => {
                    let error_msg = format!("Failed to send email to {}: {}", recipient, e);
                    error!("{}", error_msg);
                    errors.push(error_msg);
                }
            }
        }

        if !errors.is_empty() {
            anyhow::bail!("Failed to send emails to some recipients: {}", errors.join("; "));
        }

        Ok(())
    }

    /// Send email via Postmark API
    async fn send_via_postmark(&self, message: EmailMessage) -> Result<()> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .context("Postmark requires EMAIL_API_KEY environment variable")?;

        #[derive(Serialize)]
        struct PostmarkRequest {
            From: String,
            To: String,
            Subject: String,
            HtmlBody: String,
            TextBody: String,
        }

        let to_email = message.to.clone();
        let request = PostmarkRequest {
            From: format!("{} <{}>", self.config.from_name, self.config.from_email),
            To: message.to,
            Subject: message.subject,
            HtmlBody: message.html_body,
            TextBody: message.text_body,
        };
        let response = self
            .client
            .post("https://api.postmarkapp.com/email")
            .header("X-Postmark-Server-Token", api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send email via Postmark API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Postmark API error ({}): {}", status, error_text);
        }

        info!("Email sent via Postmark to {}", to_email);
        Ok(())
    }

    /// Send email via Brevo API
    async fn send_via_brevo(&self, message: EmailMessage) -> Result<()> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .context("Brevo requires EMAIL_API_KEY environment variable")?;

        #[derive(Serialize)]
        struct BrevoSender {
            name: String,
            email: String,
        }

        #[derive(Serialize)]
        struct BrevoTo {
            email: String,
        }

        #[derive(Serialize)]
        struct BrevoRequest {
            sender: BrevoSender,
            to: Vec<BrevoTo>,
            subject: String,
            htmlContent: String,
            textContent: String,
        }

        let to_email = message.to.clone();
        let request = BrevoRequest {
            sender: BrevoSender {
                name: self.config.from_name.clone(),
                email: self.config.from_email.clone(),
            },
            to: vec![BrevoTo { email: message.to }],
            subject: message.subject,
            htmlContent: message.html_body,
            textContent: message.text_body,
        };
        let response = self
            .client
            .post("https://api.brevo.com/v3/smtp/email")
            .header("api-key", api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send email via Brevo API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Brevo API error ({}): {}", status, error_text);
        }

        info!("Email sent via Brevo to {}", to_email);
        Ok(())
    }

    /// Send email via SendGrid API
    async fn send_via_sendgrid(&self, message: EmailMessage) -> Result<()> {
        let api_key = self
            .config
            .api_key
            .as_ref()
            .context("SendGrid requires EMAIL_API_KEY environment variable")?;

        #[derive(Serialize)]
        struct SendGridEmail {
            email: String,
            name: Option<String>,
        }

        #[derive(Serialize)]
        struct SendGridContent {
            #[serde(rename = "type")]
            content_type: String,
            value: String,
        }

        #[derive(Serialize)]
        struct SendGridPersonalization {
            to: Vec<SendGridEmail>,
            subject: String,
        }

        #[derive(Serialize)]
        struct SendGridRequest {
            personalizations: Vec<SendGridPersonalization>,
            from: SendGridEmail,
            subject: String,
            content: Vec<SendGridContent>,
        }

        let request = SendGridRequest {
            personalizations: vec![SendGridPersonalization {
                to: vec![SendGridEmail {
                    email: message.to.clone(),
                    name: None,
                }],
                subject: message.subject.clone(),
            }],
            from: SendGridEmail {
                email: self.config.from_email.clone(),
                name: Some(self.config.from_name.clone()),
            },
            subject: message.subject,
            content: vec![
                SendGridContent {
                    content_type: "text/plain".to_string(),
                    value: message.text_body,
                },
                SendGridContent {
                    content_type: "text/html".to_string(),
                    value: message.html_body,
                },
            ],
        };

        let to_email = message.to.clone();
        let response = self
            .client
            .post("https://api.sendgrid.com/v3/mail/send")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send email via SendGrid API")?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("SendGrid API error ({}): {}", status, error_text);
        }

        info!("Email sent via SendGrid to {}", to_email);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_provider_from_str() {
        assert_eq!(EmailProvider::from_str("postmark"), EmailProvider::Postmark);
        assert_eq!(EmailProvider::from_str("brevo"), EmailProvider::Brevo);
        assert_eq!(EmailProvider::from_str("sendinblue"), EmailProvider::Brevo);
        assert_eq!(EmailProvider::from_str("sendgrid"), EmailProvider::SendGrid);
        assert_eq!(EmailProvider::from_str("disabled"), EmailProvider::Disabled);
        assert_eq!(EmailProvider::from_str("unknown"), EmailProvider::Disabled);
    }

    #[tokio::test]
    async fn test_email_service_disabled() {
        let config = EmailConfig {
            provider: EmailProvider::Disabled,
            from_email: "test@example.com".to_string(),
            from_name: "Test".to_string(),
            api_key: None,
        };

        let service = EmailService::new(config);
        let message = EmailMessage {
            to: "recipient@example.com".to_string(),
            subject: "Test Subject".to_string(),
            html_body: "<p>Test HTML</p>".to_string(),
            text_body: "Test Text".to_string(),
        };

        // Should not fail when disabled
        let result = service.send(message).await;
        assert!(result.is_ok());
    }
}
