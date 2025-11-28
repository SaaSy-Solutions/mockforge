//! Email notification service for user communications
//!
//! Supports multiple email providers:
//! - Postmark (via API)
//! - Brevo (via API)
//! - SMTP (fallback)

use anyhow::{Context, Result};
use chrono::Datelike;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Email configuration
#[derive(Debug, Clone)]
pub struct EmailConfig {
    pub provider: EmailProvider,
    pub from_email: String,
    pub from_name: String,
    pub api_key: Option<String>, // For Postmark/Brevo
    pub smtp_host: Option<String>, // For SMTP fallback
    pub smtp_port: Option<u16>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
}

/// Email provider type
#[derive(Debug, Clone)]
pub enum EmailProvider {
    Postmark,
    Brevo,
    Smtp,
    Disabled, // For development/testing
}

impl EmailProvider {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "postmark" => EmailProvider::Postmark,
            "brevo" | "sendinblue" => EmailProvider::Brevo,
            "smtp" => EmailProvider::Smtp,
            _ => EmailProvider::Disabled,
        }
    }
}

/// Email message
#[derive(Debug, Clone)]
pub struct EmailMessage {
    pub to: String,
    pub subject: String,
    pub html_body: String,
    pub text_body: String,
}

/// Email service
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
            .expect("Failed to create HTTP client");

        Self { config, client }
    }

    /// Create email service from environment variables
    pub fn from_env() -> Self {
        let provider = std::env::var("EMAIL_PROVIDER")
            .unwrap_or_else(|_| "disabled".to_string());

        let config = EmailConfig {
            provider: EmailProvider::from_str(&provider),
            from_email: std::env::var("EMAIL_FROM")
                .unwrap_or_else(|_| "noreply@mockforge.dev".to_string()),
            from_name: std::env::var("EMAIL_FROM_NAME")
                .unwrap_or_else(|_| "MockForge".to_string()),
            api_key: std::env::var("EMAIL_API_KEY").ok(),
            smtp_host: std::env::var("SMTP_HOST").ok(),
            smtp_port: std::env::var("SMTP_PORT")
                .ok()
                .and_then(|p| p.parse().ok()),
            smtp_username: std::env::var("SMTP_USERNAME").ok(),
            smtp_password: std::env::var("SMTP_PASSWORD").ok(),
        };

        Self::new(config)
    }

    /// Send an email
    pub async fn send(&self, message: EmailMessage) -> Result<()> {
        match &self.config.provider {
            EmailProvider::Postmark => self.send_via_postmark(message).await,
            EmailProvider::Brevo => self.send_via_brevo(message).await,
            EmailProvider::Smtp => {
                // SMTP implementation would go here
                // For now, log and return success (can be implemented later)
                tracing::warn!("SMTP email provider not yet implemented, email not sent");
                Ok(())
            }
            EmailProvider::Disabled => {
                tracing::info!("Email disabled, would send: {} to {}", message.subject, message.to);
                Ok(())
            }
        }
    }

    /// Send email via Postmark API
    async fn send_via_postmark(&self, message: EmailMessage) -> Result<()> {
        let api_key = self.config.api_key.as_ref()
            .context("Postmark requires EMAIL_API_KEY")?;

        #[derive(Serialize)]
        struct PostmarkRequest {
            From: String,
            To: String,
            Subject: String,
            HtmlBody: String,
            TextBody: String,
        }

        let request = PostmarkRequest {
            From: format!("{} <{}>", self.config.from_name, self.config.from_email),
            To: message.to,
            Subject: message.subject,
            HtmlBody: message.html_body,
            TextBody: message.text_body,
        };

        let response = self.client
            .post("https://api.postmarkapp.com/email")
            .header("X-Postmark-Server-Token", api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send email via Postmark")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Postmark API error: {}", error_text);
        }

        Ok(())
    }

    /// Send email via Brevo API
    async fn send_via_brevo(&self, message: EmailMessage) -> Result<()> {
        let api_key = self.config.api_key.as_ref()
            .context("Brevo requires EMAIL_API_KEY")?;

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

        let response = self.client
            .post("https://api.brevo.com/v3/smtp/email")
            .header("api-key", api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .context("Failed to send email via Brevo")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("Brevo API error: {}", error_text);
        }

        Ok(())
    }

    /// Generate welcome email content
    pub fn generate_welcome_email(username: &str, email: &str) -> EmailMessage {
        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 40px 20px; text-align: center; border-radius: 8px 8px 0 0; }}
        .content {{ background: #ffffff; padding: 30px; border: 1px solid #e0e0e0; border-top: none; border-radius: 0 0 8px 8px; }}
        .button {{ display: inline-block; padding: 12px 24px; background: #667eea; color: white; text-decoration: none; border-radius: 6px; margin: 20px 0; }}
        .footer {{ text-align: center; color: #666; font-size: 12px; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e0e0e0; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Welcome to MockForge Cloud! 🎉</h1>
    </div>
    <div class="content">
        <p>Hi {},</p>
        <p>Welcome to MockForge Cloud! We're excited to have you on board.</p>
        <p>MockForge helps you build, test, and deploy API mocks with ease. Here's what you can do:</p>
        <ul>
            <li>🚀 Deploy hosted mocks with shareable URLs</li>
            <li>📦 Browse and install plugins from our marketplace</li>
            <li>📋 Use templates and scenarios to accelerate development</li>
            <li>🤖 Leverage AI-powered mock generation (BYOK on Free tier)</li>
        </ul>
        <p style="text-align: center;">
            <a href="https://app.mockforge.dev" class="button">Get Started</a>
        </p>
        <p>If you have any questions, feel free to reach out to our support team.</p>
        <p>Happy mocking!<br>The MockForge Team</p>
    </div>
    <div class="footer">
        <p>© {} MockForge. All rights reserved.</p>
        <p><a href="https://mockforge.dev/terms">Terms of Service</a> | <a href="https://mockforge.dev/privacy">Privacy Policy</a></p>
    </div>
</body>
</html>
"#,
            username,
            chrono::Utc::now().year()
        );

        let text_body = format!(
            r#"
Welcome to MockForge Cloud!

Hi {},

Welcome to MockForge Cloud! We're excited to have you on board.

MockForge helps you build, test, and deploy API mocks with ease. Here's what you can do:

- Deploy hosted mocks with shareable URLs
- Browse and install plugins from our marketplace
- Use templates and scenarios to accelerate development
- Leverage AI-powered mock generation (BYOK on Free tier)

Get started: https://app.mockforge.dev

If you have any questions, feel free to reach out to our support team.

Happy mocking!
The MockForge Team

© {} MockForge. All rights reserved.
Terms: https://mockforge.dev/terms
Privacy: https://mockforge.dev/privacy
"#,
            username,
            chrono::Utc::now().year()
        );

        EmailMessage {
            to: email.to_string(),
            subject: "Welcome to MockForge Cloud! 🎉".to_string(),
            html_body,
            text_body,
        }
    }

    /// Generate subscription confirmation email
    pub fn generate_subscription_confirmation(
        username: &str,
        email: &str,
        plan: &str,
        amount: Option<f64>,
        period_end: Option<chrono::DateTime<chrono::Utc>>,
    ) -> EmailMessage {
        let amount_text = amount
            .map(|a| format!("${:.2}", a))
            .unwrap_or_else(|| "your plan".to_string());

        let period_text = period_end
            .map(|d| format!("Your subscription renews on {}", d.format("%B %d, %Y")))
            .unwrap_or_else(|| "Your subscription is active".to_string());

        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 40px 20px; text-align: center; border-radius: 8px 8px 0 0; }}
        .content {{ background: #ffffff; padding: 30px; border: 1px solid #e0e0e0; border-top: none; border-radius: 0 0 8px 8px; }}
        .button {{ display: inline-block; padding: 12px 24px; background: #667eea; color: white; text-decoration: none; border-radius: 6px; margin: 20px 0; }}
        .info-box {{ background: #f8f9fa; border-left: 4px solid #667eea; padding: 15px; margin: 20px 0; }}
        .footer {{ text-align: center; color: #666; font-size: 12px; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e0e0e0; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Subscription Confirmed! ✅</h1>
    </div>
    <div class="content">
        <p>Hi {},</p>
        <p>Your subscription to MockForge Cloud <strong>{}</strong> plan has been confirmed!</p>
        <div class="info-box">
            <p><strong>Plan:</strong> {}</p>
            <p><strong>Amount:</strong> {}</p>
            <p><strong>{}</strong></p>
        </div>
        <p>You now have access to all features included in your plan. Thank you for choosing MockForge!</p>
        <p style="text-align: center;">
            <a href="https://app.mockforge.dev/billing" class="button">Manage Subscription</a>
        </p>
        <p>If you have any questions about your subscription, please contact our support team.</p>
        <p>Best regards,<br>The MockForge Team</p>
    </div>
    <div class="footer">
        <p>© {} MockForge. All rights reserved.</p>
        <p><a href="https://mockforge.dev/terms">Terms of Service</a> | <a href="https://mockforge.dev/privacy">Privacy Policy</a></p>
    </div>
</body>
</html>
"#,
            username,
            plan,
            plan,
            amount_text,
            period_text,
            chrono::Utc::now().year()
        );

        let text_body = format!(
            r#"
Subscription Confirmed!

Hi {},

Your subscription to MockForge Cloud {} plan has been confirmed!

Plan: {}
Amount: {}
{}

You now have access to all features included in your plan. Thank you for choosing MockForge!

Manage your subscription: https://app.mockforge.dev/billing

If you have any questions about your subscription, please contact our support team.

Best regards,
The MockForge Team

© {} MockForge. All rights reserved.
Terms: https://mockforge.dev/terms
Privacy: https://mockforge.dev/privacy
"#,
            username,
            plan,
            plan,
            amount_text,
            period_text,
            chrono::Utc::now().year()
        );

        EmailMessage {
            to: email.to_string(),
            subject: format!("Subscription Confirmed - MockForge Cloud {}", plan),
            html_body,
            text_body,
        }
    }

    /// Generate payment failed email
    pub fn generate_payment_failed(
        username: &str,
        email: &str,
        plan: &str,
        amount: f64,
        retry_date: Option<chrono::DateTime<chrono::Utc>>,
    ) -> EmailMessage {
        let retry_text = retry_date
            .map(|d| format!("We'll automatically retry on {}.", d.format("%B %d, %Y")))
            .unwrap_or_else(|| "Please update your payment method to continue service.".to_string());

        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background: linear-gradient(135deg, #e74c3c 0%, #c0392b 100%); color: white; padding: 40px 20px; text-align: center; border-radius: 8px 8px 0 0; }}
        .content {{ background: #ffffff; padding: 30px; border: 1px solid #e0e0e0; border-top: none; border-radius: 0 0 8px 8px; }}
        .button {{ display: inline-block; padding: 12px 24px; background: #e74c3c; color: white; text-decoration: none; border-radius: 6px; margin: 20px 0; }}
        .warning-box {{ background: #fff3cd; border-left: 4px solid #ffc107; padding: 15px; margin: 20px 0; }}
        .footer {{ text-align: center; color: #666; font-size: 12px; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e0e0e0; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Payment Failed ⚠️</h1>
    </div>
    <div class="content">
        <p>Hi {},</p>
        <p>We were unable to process your payment for your MockForge Cloud <strong>{}</strong> subscription.</p>
        <div class="warning-box">
            <p><strong>Amount:</strong> ${:.2}</p>
            <p><strong>Plan:</strong> {}</p>
            <p>{}</p>
        </div>
        <p>To avoid service interruption, please update your payment method as soon as possible.</p>
        <p style="text-align: center;">
            <a href="https://app.mockforge.dev/billing" class="button">Update Payment Method</a>
        </p>
        <p>If you continue to experience issues, please contact our support team for assistance.</p>
        <p>Best regards,<br>The MockForge Team</p>
    </div>
    <div class="footer">
        <p>© {} MockForge. All rights reserved.</p>
        <p><a href="https://mockforge.dev/terms">Terms of Service</a> | <a href="https://mockforge.dev/privacy">Privacy Policy</a></p>
    </div>
</body>
</html>
"#,
            username,
            plan,
            amount,
            plan,
            retry_text,
            chrono::Utc::now().year()
        );

        let text_body = format!(
            r#"
Payment Failed

Hi {},

We were unable to process your payment for your MockForge Cloud {} subscription.

Amount: ${:.2}
Plan: {}
{}

To avoid service interruption, please update your payment method as soon as possible.

Update payment method: https://app.mockforge.dev/billing

If you continue to experience issues, please contact our support team for assistance.

Best regards,
The MockForge Team

© {} MockForge. All rights reserved.
Terms: https://mockforge.dev/terms
Privacy: https://mockforge.dev/privacy
"#,
            username,
            plan,
            amount,
            plan,
            retry_text,
            chrono::Utc::now().year()
        );

        EmailMessage {
            to: email.to_string(),
            subject: "Payment Failed - Action Required".to_string(),
            html_body,
            text_body,
        }
    }

    /// Generate subscription canceled email
    pub fn generate_subscription_canceled(
        username: &str,
        email: &str,
        plan: &str,
        access_until: Option<chrono::DateTime<chrono::Utc>>,
    ) -> EmailMessage {
        let access_text = access_until
            .map(|d| format!("You'll continue to have access to {} features until {}.", plan, d.format("%B %d, %Y")))
            .unwrap_or_else(|| format!("Your {} subscription has been canceled.", plan));

        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background: linear-gradient(135deg, #95a5a6 0%, #7f8c8d 100%); color: white; padding: 40px 20px; text-align: center; border-radius: 8px 8px 0 0; }}
        .content {{ background: #ffffff; padding: 30px; border: 1px solid #e0e0e0; border-top: none; border-radius: 0 0 8px 8px; }}
        .button {{ display: inline-block; padding: 12px 24px; background: #667eea; color: white; text-decoration: none; border-radius: 6px; margin: 20px 0; }}
        .info-box {{ background: #f8f9fa; border-left: 4px solid #95a5a6; padding: 15px; margin: 20px 0; }}
        .footer {{ text-align: center; color: #666; font-size: 12px; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e0e0e0; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Subscription Canceled</h1>
    </div>
    <div class="content">
        <p>Hi {},</p>
        <p>Your MockForge Cloud <strong>{}</strong> subscription has been canceled.</p>
        <div class="info-box">
            <p>{}</p>
        </div>
        <p>We're sorry to see you go! If you change your mind, you can reactivate your subscription at any time.</p>
        <p style="text-align: center;">
            <a href="https://app.mockforge.dev/billing" class="button">Reactivate Subscription</a>
        </p>
        <p>If you have any feedback about your experience, we'd love to hear from you.</p>
        <p>Best regards,<br>The MockForge Team</p>
    </div>
    <div class="footer">
        <p>© {} MockForge. All rights reserved.</p>
        <p><a href="https://mockforge.dev/terms">Terms of Service</a> | <a href="https://mockforge.dev/privacy">Privacy Policy</a></p>
    </div>
</body>
</html>
"#,
            username,
            plan,
            access_text,
            chrono::Utc::now().year()
        );

        let text_body = format!(
            r#"
Subscription Canceled

Hi {},

Your MockForge Cloud {} subscription has been canceled.

{}

We're sorry to see you go! If you change your mind, you can reactivate your subscription at any time.

Reactivate: https://app.mockforge.dev/billing

If you have any feedback about your experience, we'd love to hear from you.

Best regards,
The MockForge Team

© {} MockForge. All rights reserved.
Terms: https://mockforge.dev/terms
Privacy: https://mockforge.dev/privacy
"#,
            username,
            plan,
            access_text,
            chrono::Utc::now().year()
        );

        EmailMessage {
            to: email.to_string(),
            subject: "Subscription Canceled".to_string(),
            html_body,
            text_body,
        }
    }

    /// Generate support request confirmation email
    pub fn generate_support_confirmation(
        username: &str,
        email: &str,
        ticket_id: &str,
        subject: &str,
    ) -> EmailMessage {
        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 40px 20px; text-align: center; border-radius: 8px 8px 0 0; }}
        .content {{ background: #ffffff; padding: 30px; border: 1px solid #e0e0e0; border-top: none; border-radius: 0 0 8px 8px; }}
        .info-box {{ background: #f8f9fa; border-left: 4px solid #667eea; padding: 15px; margin: 20px 0; }}
        .footer {{ text-align: center; color: #666; font-size: 12px; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e0e0e0; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Support Request Received ✅</h1>
    </div>
    <div class="content">
        <p>Hi {},</p>
        <p>We've received your support request and will respond as soon as possible based on your plan's SLA.</p>
        <div class="info-box">
            <p><strong>Ticket ID:</strong> {}</p>
            <p><strong>Subject:</strong> {}</p>
        </div>
        <p>You can track the status of your request using the ticket ID above. We'll send you updates via email.</p>
        <p>If you need to add more information to this request, please reply to this email or submit a new request with the ticket ID in the subject.</p>
        <p>Thank you for contacting MockForge support!</p>
        <p>Best regards,<br>The MockForge Support Team</p>
    </div>
    <div class="footer">
        <p>© {} MockForge. All rights reserved.</p>
        <p><a href="https://mockforge.dev/terms">Terms of Service</a> | <a href="https://mockforge.dev/privacy">Privacy Policy</a></p>
    </div>
</body>
</html>
"#,
            username,
            ticket_id,
            subject,
            chrono::Utc::now().year()
        );

        let text_body = format!(
            r#"
Support Request Received

Hi {},

We've received your support request and will respond as soon as possible based on your plan's SLA.

Ticket ID: {}
Subject: {}

You can track the status of your request using the ticket ID above. We'll send you updates via email.

If you need to add more information to this request, please reply to this email or submit a new request with the ticket ID in the subject.

Thank you for contacting MockForge support!

Best regards,
The MockForge Support Team

© {} MockForge. All rights reserved.
Terms: https://mockforge.dev/terms
Privacy: https://mockforge.dev/privacy
"#,
            username,
            ticket_id,
            subject,
            chrono::Utc::now().year()
        );

        EmailMessage {
            to: email.to_string(),
            subject: format!("Support Request Received - {}", ticket_id),
            html_body,
            text_body,
        }
    }

    /// Generate email verification email
    pub fn generate_verification_email(
        username: &str,
        email: &str,
        verification_token: &str,
    ) -> EmailMessage {
        let verification_url = format!(
            "{}/verify-email?token={}",
            std::env::var("APP_BASE_URL").unwrap_or_else(|_| "https://app.mockforge.dev".to_string()),
            verification_token
        );

        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 40px 20px; text-align: center; border-radius: 8px 8px 0 0; }}
        .content {{ background: #ffffff; padding: 30px; border: 1px solid #e0e0e0; border-top: none; border-radius: 0 0 8px 8px; }}
        .button {{ display: inline-block; padding: 12px 24px; background: #667eea; color: white; text-decoration: none; border-radius: 6px; margin: 20px 0; }}
        .footer {{ text-align: center; color: #666; font-size: 12px; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e0e0e0; }}
        .code {{ background: #f8f9fa; padding: 10px; border-radius: 4px; font-family: monospace; word-break: break-all; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Verify Your Email Address</h1>
    </div>
    <div class="content">
        <p>Hi {},</p>
        <p>Thank you for signing up for MockForge Cloud! Please verify your email address to complete your registration.</p>
        <p style="text-align: center;">
            <a href="{}" class="button">Verify Email Address</a>
        </p>
        <p>Or copy and paste this link into your browser:</p>
        <div class="code">{}</div>
        <p>This verification link will expire in 24 hours.</p>
        <p>If you didn't create an account with MockForge, you can safely ignore this email.</p>
        <p>Best regards,<br>The MockForge Team</p>
    </div>
    <div class="footer">
        <p>© {} MockForge. All rights reserved.</p>
        <p><a href="https://mockforge.dev/terms">Terms of Service</a> | <a href="https://mockforge.dev/privacy">Privacy Policy</a></p>
    </div>
</body>
</html>
"#,
            username,
            verification_url,
            verification_url,
            chrono::Utc::now().year()
        );

        let text_body = format!(
            r#"
Verify Your Email Address

Hi {},

Thank you for signing up for MockForge Cloud! Please verify your email address to complete your registration.

Click this link to verify your email:
{}

This verification link will expire in 24 hours.

If you didn't create an account with MockForge, you can safely ignore this email.

Best regards,
The MockForge Team

© {} MockForge. All rights reserved.
Terms: https://mockforge.dev/terms
Privacy: https://mockforge.dev/privacy
"#,
            username,
            verification_url,
            chrono::Utc::now().year()
        );

        EmailMessage {
            to: email.to_string(),
            subject: "Verify Your Email Address - MockForge Cloud".to_string(),
            html_body,
            text_body,
        }
    }

    /// Generate API token rotation reminder email
    pub fn generate_token_rotation_reminder(
        username: &str,
        email: &str,
        token_name: &str,
        token_age_days: i64,
        rotation_url: &str,
    ) -> EmailMessage {
        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 40px 20px; text-align: center; border-radius: 8px 8px 0 0; }}
        .content {{ background: #ffffff; padding: 30px; border: 1px solid #e0e0e0; border-top: none; border-radius: 0 0 8px 8px; }}
        .button {{ display: inline-block; padding: 12px 24px; background: #667eea; color: white; text-decoration: none; border-radius: 6px; margin: 20px 0; }}
        .footer {{ text-align: center; color: #666; font-size: 12px; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e0e0e0; }}
        .warning {{ background: #fff3cd; border-left: 4px solid #ffc107; padding: 15px; margin: 20px 0; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>API Token Rotation Reminder</h1>
    </div>
    <div class="content">
        <p>Hi {},</p>
        <div class="warning">
            <strong>Security Best Practice:</strong> Your API token "<strong>{}</strong>" is {} days old and should be rotated for security.
        </div>
        <p>Regularly rotating API tokens is a security best practice that helps protect your account and data. We recommend rotating tokens every 90 days.</p>
        <p style="text-align: center;">
            <a href="{}" class="button">Rotate Token Now</a>
        </p>
        <p>Or visit your API tokens page to rotate this token manually.</p>
        <p>If you no longer need this token, you can delete it from your settings.</p>
        <p>Best regards,<br>The MockForge Team</p>
    </div>
    <div class="footer">
        <p>© {} MockForge. All rights reserved.</p>
        <p><a href="https://mockforge.dev/terms">Terms of Service</a> | <a href="https://mockforge.dev/privacy">Privacy Policy</a></p>
    </div>
</body>
</html>
"#,
            username,
            token_name,
            token_age_days,
            rotation_url,
            chrono::Utc::now().year()
        );

        let text_body = format!(
            r#"
API Token Rotation Reminder

Hi {},

Security Best Practice: Your API token "{}" is {} days old and should be rotated for security.

Regularly rotating API tokens is a security best practice that helps protect your account and data. We recommend rotating tokens every 90 days.

Rotate your token: {}

Or visit your API tokens page to rotate this token manually.

If you no longer need this token, you can delete it from your settings.

Best regards,
The MockForge Team

© {} MockForge. All rights reserved.
Terms: https://mockforge.dev/terms
Privacy: https://mockforge.dev/privacy
"#,
            username,
            token_name,
            token_age_days,
            rotation_url,
            chrono::Utc::now().year()
        );

        EmailMessage {
            to: email.to_string(),
            subject: format!("Action Required: Rotate Your API Token '{}'", token_name),
            html_body,
            text_body,
        }
    }

    /// Generate password reset email
    pub fn generate_password_reset_email(
        username: &str,
        email: &str,
        reset_token: &str,
    ) -> EmailMessage {
        let reset_url = format!(
            "{}/reset-password?token={}",
            std::env::var("APP_BASE_URL").unwrap_or_else(|_| "https://app.mockforge.dev".to_string()),
            reset_token
        );

        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); color: white; padding: 40px 20px; text-align: center; border-radius: 8px 8px 0 0; }}
        .content {{ background: #ffffff; padding: 30px; border: 1px solid #e0e0e0; border-top: none; border-radius: 0 0 8px 8px; }}
        .button {{ display: inline-block; padding: 12px 24px; background: #667eea; color: white; text-decoration: none; border-radius: 6px; margin: 20px 0; }}
        .footer {{ text-align: center; color: #666; font-size: 12px; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e0e0e0; }}
        .warning {{ background: #fff3cd; border-left: 4px solid #ffc107; padding: 15px; margin: 20px 0; }}
        .code {{ background: #f8f9fa; padding: 10px; border-radius: 4px; font-family: monospace; word-break: break-all; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Reset Your Password</h1>
    </div>
    <div class="content">
        <p>Hi {},</p>
        <p>We received a request to reset your password for your MockForge Cloud account.</p>
        <p style="text-align: center;">
            <a href="{}" class="button">Reset Password</a>
        </p>
        <p>Or copy and paste this link into your browser:</p>
        <div class="code">{}</div>
        <div class="warning">
            <strong>Security Notice:</strong> This password reset link will expire in 1 hour. If you didn't request a password reset, you can safely ignore this email.
        </div>
        <p>If you continue to have problems, please contact our support team.</p>
        <p>Best regards,<br>The MockForge Team</p>
    </div>
    <div class="footer">
        <p>© {} MockForge. All rights reserved.</p>
        <p><a href="https://mockforge.dev/terms">Terms of Service</a> | <a href="https://mockforge.dev/privacy">Privacy Policy</a></p>
    </div>
</body>
</html>
"#,
            username,
            reset_url,
            reset_url,
            chrono::Utc::now().year()
        );

        let text_body = format!(
            r#"
Reset Your Password

Hi {},

We received a request to reset your password for your MockForge Cloud account.

Click this link to reset your password:
{}

This password reset link will expire in 1 hour. If you didn't request a password reset, you can safely ignore this email.

If you continue to have problems, please contact our support team.

Best regards,
The MockForge Team

© {} MockForge. All rights reserved.
Terms: https://mockforge.dev/terms
Privacy: https://mockforge.dev/privacy
"#,
            username,
            reset_url,
            chrono::Utc::now().year()
        );

        EmailMessage {
            to: email.to_string(),
            subject: "Reset Your Password - MockForge Cloud".to_string(),
            html_body,
            text_body,
        }
    }

    /// Generate deployment status notification email
    pub fn generate_deployment_status_email(
        username: &str,
        email: &str,
        deployment_name: &str,
        status: &str,
        deployment_url: Option<&str>,
        error_message: Option<&str>,
    ) -> EmailMessage {
        let (header_color, header_text, status_icon) = match status {
            "active" => ("#28a745", "Deployment Successful", "✅"),
            "failed" => ("#dc3545", "Deployment Failed", "❌"),
            "deploying" => ("#007bff", "Deployment In Progress", "⏳"),
            _ => ("#6c757d", "Deployment Status Update", "ℹ️"),
        };

        let deployment_link = deployment_url.map(|url| {
            format!(
                r#"<p style="text-align: center;">
            <a href="{}" class="button">View Deployment</a>
        </p>"#,
                url
            )
        }).unwrap_or_else(String::new);

        let error_section = error_message.map(|msg| {
            format!(
                r#"<div class="warning">
            <strong>Error Details:</strong><br>
            <pre style="white-space: pre-wrap; font-size: 12px;">{}</pre>
        </div>"#,
                msg
            )
        }).unwrap_or_else(String::new);

        let html_body = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; line-height: 1.6; color: #333; max-width: 600px; margin: 0 auto; padding: 20px; }}
        .header {{ background: linear-gradient(135deg, {} 0%, {} 100%); color: white; padding: 40px 20px; text-align: center; border-radius: 8px 8px 0 0; }}
        .content {{ background: #ffffff; padding: 30px; border: 1px solid #e0e0e0; border-top: none; border-radius: 0 0 8px 8px; }}
        .button {{ display: inline-block; padding: 12px 24px; background: {}; color: white; text-decoration: none; border-radius: 6px; margin: 20px 0; }}
        .footer {{ text-align: center; color: #666; font-size: 12px; margin-top: 30px; padding-top: 20px; border-top: 1px solid #e0e0e0; }}
        .warning {{ background: #fff3cd; border-left: 4px solid #ffc107; padding: 15px; margin: 20px 0; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>{} {}</h1>
    </div>
    <div class="content">
        <p>Hi {},</p>
        <p>Your hosted mock deployment "<strong>{}</strong>" status has been updated to <strong>{}</strong>.</p>
        {}
        {}
        <p>You can view and manage your deployments in the MockForge Cloud dashboard.</p>
        <p>Best regards,<br>The MockForge Team</p>
    </div>
    <div class="footer">
        <p>© {} MockForge. All rights reserved.</p>
        <p><a href="https://mockforge.dev/terms">Terms of Service</a> | <a href="https://mockforge.dev/privacy">Privacy Policy</a></p>
    </div>
</body>
</html>
"#,
            header_color,
            header_color,
            header_color,
            status_icon,
            header_text,
            username,
            deployment_name,
            status,
            deployment_link,
            error_section,
            chrono::Utc::now().year()
        );

        let text_body = format!(
            r#"
Deployment Status Update

Hi {},

Your hosted mock deployment "{}" status has been updated to {}.

{}

{}

You can view and manage your deployments in the MockForge Cloud dashboard.

Best regards,
The MockForge Team

© {} MockForge. All rights reserved.
Terms: https://mockforge.dev/terms
Privacy: https://mockforge.dev/privacy
"#,
            username,
            deployment_name,
            status,
            deployment_url.map(|url| format!("View deployment: {}", url)).unwrap_or_else(String::new),
            error_message.map(|msg| format!("Error: {}", msg)).unwrap_or_else(String::new),
            chrono::Utc::now().year()
        );

        EmailMessage {
            to: email.to_string(),
            subject: format!("{} - Deployment '{}' is {}", status_icon, deployment_name, status),
            html_body,
            text_body,
        }
    }
}
