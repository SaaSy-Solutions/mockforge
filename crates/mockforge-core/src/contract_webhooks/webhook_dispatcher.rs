//! Webhook dispatcher for contract change alerts
//!
//! This module dispatches webhooks to configured endpoints when contract events occur.
//! It supports event filtering, retry logic, and webhook signing.

use super::types::{ContractEvent, RetryConfig, WebhookConfig, WebhookPayload, WebhookResult};
use crate::Result;
use chrono::Utc;
use reqwest::Client;
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error, warn};

/// Webhook dispatcher for contract events
pub struct WebhookDispatcher {
    /// Configured webhook endpoints
    webhooks: Vec<WebhookConfig>,

    /// HTTP client for sending webhooks
    client: Client,
}

impl WebhookDispatcher {
    /// Create a new webhook dispatcher
    pub fn new(webhooks: Vec<WebhookConfig>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { webhooks, client }
    }

    /// Dispatch a contract event to all matching webhooks
    pub async fn dispatch(&self, event: &ContractEvent) -> Vec<WebhookResult> {
        let mut results = Vec::new();

        for webhook in &self.webhooks {
            // Check if webhook should receive this event
            if !self.should_dispatch(webhook, event) {
                continue;
            }

            // Dispatch to this webhook
            let result = self.dispatch_to_webhook(webhook, event).await;
            results.push(result);
        }

        results
    }

    /// Check if webhook should receive this event
    fn should_dispatch(&self, webhook: &WebhookConfig, event: &ContractEvent) -> bool {
        // Check if event type matches
        let event_type = event.event_type();
        if !webhook.events.is_empty() && !webhook.events.contains(&event_type.to_string()) {
            return false;
        }

        // Check severity threshold
        if let Some(ref threshold) = webhook.severity_threshold {
            if !event.meets_severity_threshold(threshold) {
                return false;
            }
        }

        true
    }

    /// Dispatch event to a specific webhook
    async fn dispatch_to_webhook(
        &self,
        webhook: &WebhookConfig,
        event: &ContractEvent,
    ) -> WebhookResult {
        // Build payload
        let payload = self.build_payload(event, webhook);

        // Send with retry logic
        self.send_with_retry(webhook, &payload, 0).await
    }

    /// Build webhook payload from event
    fn build_payload(&self, event: &ContractEvent, webhook: &WebhookConfig) -> WebhookPayload {
        let mut data = match event {
            ContractEvent::MismatchDetected {
                endpoint,
                method,
                mismatch_count,
                severity,
                summary,
            } => json!({
                "endpoint": endpoint,
                "method": method,
                "mismatch_count": mismatch_count,
                "severity": severity,
                "summary": summary,
            }),
            ContractEvent::BreakingChange {
                endpoint,
                method,
                description,
                severity,
                change_type,
            } => json!({
                "endpoint": endpoint,
                "method": method,
                "description": description,
                "severity": severity,
                "change_type": change_type,
            }),
            ContractEvent::DriftWarning {
                endpoint,
                method,
                description,
                severity,
                occurrence_count,
            } => json!({
                "endpoint": endpoint,
                "method": method,
                "description": description,
                "severity": severity,
                "occurrence_count": occurrence_count,
            }),
            ContractEvent::CorrectionApplied {
                endpoint,
                correction_count,
                patch_file,
            } => json!({
                "endpoint": endpoint,
                "correction_count": correction_count,
                "patch_file": patch_file,
            }),
        };

        // Add signature if secret is configured
        let signature = if let Some(ref secret) = webhook.secret {
            let payload_str = serde_json::to_string(&data).unwrap_or_default();
            Some(self.sign_payload(&payload_str, secret))
        } else {
            None
        };

        WebhookPayload {
            event_type: event.event_type().to_string(),
            timestamp: Utc::now(),
            data,
            severity: event.severity().to_string(),
            signature,
            metadata: HashMap::new(),
        }
    }

    /// Sign webhook payload with secret
    fn sign_payload(&self, payload: &str, secret: &str) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(secret.as_bytes());
        hasher.update(payload.as_bytes());
        let hash = hasher.finalize();
        format!("sha256={}", hex::encode(hash))
    }

    /// Send webhook with retry logic
    async fn send_with_retry(
        &self,
        webhook: &WebhookConfig,
        payload: &WebhookPayload,
        attempt: usize,
    ) -> WebhookResult {
        if attempt >= webhook.retry.max_attempts {
            return WebhookResult::failure(
                format!("Max retry attempts ({}) exceeded", webhook.retry.max_attempts),
                attempt,
            );
        }

        // Build request
        let mut request = match webhook.method.as_str() {
            "POST" => self.client.post(&webhook.url),
            "PUT" => self.client.put(&webhook.url),
            "PATCH" => self.client.patch(&webhook.url),
            _ => self.client.post(&webhook.url),
        };

        // Add headers
        for (key, value) in &webhook.headers {
            request = request.header(key, value);
        }

        // Add signature header if present
        if let Some(ref signature) = payload.signature {
            request = request.header("X-Webhook-Signature", signature);
        }

        // Add timestamp header
        request = request.header("X-Webhook-Timestamp", payload.timestamp.to_rfc3339());

        // Send request
        match request.json(payload).send().await {
            Ok(response) => {
                let status = response.status();
                let response_body = response.text().await.ok();

                if status.is_success() {
                    debug!("Webhook sent successfully to {}: {}", webhook.url, status);
                    WebhookResult::success(status.as_u16(), response_body)
                } else {
                    warn!("Webhook returned error status: {} to {}", status, webhook.url);

                    // Retry on server errors (5xx)
                    if status.is_server_error() && attempt < webhook.retry.max_attempts - 1 {
                        let delay = self.calculate_retry_delay(&webhook.retry, attempt);
                        tokio::time::sleep(Duration::from_secs(delay)).await;
                        return Box::pin(self.send_with_retry(webhook, payload, attempt + 1)).await;
                    }

                    WebhookResult::failure(
                        format!("HTTP {}: {}", status, response_body.as_deref().unwrap_or("")),
                        attempt,
                    )
                }
            }
            Err(e) => {
                error!("Webhook request failed to {}: {}", webhook.url, e);

                // Retry on network errors
                if attempt < webhook.retry.max_attempts - 1 {
                    let delay = self.calculate_retry_delay(&webhook.retry, attempt);
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                    return Box::pin(self.send_with_retry(webhook, payload, attempt + 1)).await;
                }

                WebhookResult::failure(e.to_string(), attempt)
            }
        }
    }

    /// Calculate retry delay
    fn calculate_retry_delay(&self, retry_config: &RetryConfig, attempt: usize) -> u64 {
        if retry_config.exponential_backoff {
            let delay = retry_config.initial_delay_secs * (1 << attempt) as u64;
            delay.min(retry_config.max_delay_secs)
        } else {
            retry_config.initial_delay_secs
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_dispatch_event_type_match() {
        let webhook = WebhookConfig {
            url: "https://example.com/webhook".to_string(),
            events: vec!["contract.breaking_change".to_string()],
            ..Default::default()
        };

        let event = ContractEvent::BreakingChange {
            endpoint: "/api/users".to_string(),
            method: "POST".to_string(),
            description: "Test".to_string(),
            severity: "critical".to_string(),
            change_type: "field_removed".to_string(),
        };

        let dispatcher = WebhookDispatcher::new(vec![webhook.clone()]);
        assert!(dispatcher.should_dispatch(&webhook, &event));
    }

    #[test]
    fn test_should_dispatch_event_type_mismatch() {
        let webhook = WebhookConfig {
            url: "https://example.com/webhook".to_string(),
            events: vec!["contract.breaking_change".to_string()],
            ..Default::default()
        };

        let event = ContractEvent::MismatchDetected {
            endpoint: "/api/users".to_string(),
            method: "POST".to_string(),
            mismatch_count: 1,
            severity: "high".to_string(),
            summary: "Test".to_string(),
        };

        let dispatcher = WebhookDispatcher::new(vec![]);
        assert!(!dispatcher.should_dispatch(&webhook, &event));
    }

    #[test]
    fn test_should_dispatch_severity_threshold() {
        let webhook = WebhookConfig {
            url: "https://example.com/webhook".to_string(),
            events: vec!["contract.mismatch.detected".to_string()],
            severity_threshold: Some("high".to_string()),
            ..Default::default()
        };

        let event_high = ContractEvent::MismatchDetected {
            endpoint: "/api/users".to_string(),
            method: "POST".to_string(),
            mismatch_count: 1,
            severity: "high".to_string(),
            summary: "Test".to_string(),
        };

        let event_low = ContractEvent::MismatchDetected {
            endpoint: "/api/users".to_string(),
            method: "POST".to_string(),
            mismatch_count: 1,
            severity: "low".to_string(),
            summary: "Test".to_string(),
        };

        let dispatcher = WebhookDispatcher::new(vec![]);
        assert!(dispatcher.should_dispatch(&webhook, &event_high));
        assert!(!dispatcher.should_dispatch(&webhook, &event_low));
    }

    #[test]
    fn test_calculate_retry_delay_exponential() {
        let retry_config = RetryConfig {
            initial_delay_secs: 5,
            exponential_backoff: true,
            max_delay_secs: 60,
            ..Default::default()
        };

        let dispatcher = WebhookDispatcher::new(vec![]);

        assert_eq!(dispatcher.calculate_retry_delay(&retry_config, 0), 5);
        assert_eq!(dispatcher.calculate_retry_delay(&retry_config, 1), 10);
        assert_eq!(dispatcher.calculate_retry_delay(&retry_config, 2), 20);
        assert_eq!(dispatcher.calculate_retry_delay(&retry_config, 3), 40);
        assert_eq!(dispatcher.calculate_retry_delay(&retry_config, 4), 60); // Capped at max
    }

    #[test]
    fn test_calculate_retry_delay_linear() {
        let retry_config = RetryConfig {
            initial_delay_secs: 5,
            exponential_backoff: false,
            ..Default::default()
        };

        let dispatcher = WebhookDispatcher::new(vec![]);

        assert_eq!(dispatcher.calculate_retry_delay(&retry_config, 0), 5);
        assert_eq!(dispatcher.calculate_retry_delay(&retry_config, 1), 5);
        assert_eq!(dispatcher.calculate_retry_delay(&retry_config, 2), 5);
    }
}
