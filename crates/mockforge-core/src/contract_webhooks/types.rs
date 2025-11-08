//! Types for contract webhook system

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Contract event types that can trigger webhooks
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum ContractEvent {
    /// Any mismatch detected between request and contract
    #[serde(rename = "contract.mismatch.detected")]
    MismatchDetected {
        /// Endpoint path
        endpoint: String,
        /// HTTP method
        method: String,
        /// Number of mismatches found
        mismatch_count: usize,
        /// Severity of the most critical mismatch
        severity: String,
        /// Summary of mismatches
        summary: String,
    },

    /// Breaking change detected
    #[serde(rename = "contract.breaking_change")]
    BreakingChange {
        /// Endpoint path
        endpoint: String,
        /// HTTP method
        method: String,
        /// Description of the breaking change
        description: String,
        /// Severity level
        severity: String,
        /// Change type
        change_type: String,
    },

    /// Significant drift pattern detected
    #[serde(rename = "contract.drift.warning")]
    DriftWarning {
        /// Endpoint path
        endpoint: String,
        /// HTTP method
        method: String,
        /// Description of the drift
        description: String,
        /// Severity level
        severity: String,
        /// Number of occurrences
        occurrence_count: usize,
    },

    /// Correction proposal applied
    #[serde(rename = "contract.correction.applied")]
    CorrectionApplied {
        /// Endpoint path
        endpoint: String,
        /// Number of corrections applied
        correction_count: usize,
        /// Patch file path
        patch_file: Option<String>,
    },
}

impl ContractEvent {
    /// Get the event type name
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::MismatchDetected { .. } => "contract.mismatch.detected",
            Self::BreakingChange { .. } => "contract.breaking_change",
            Self::DriftWarning { .. } => "contract.drift.warning",
            Self::CorrectionApplied { .. } => "contract.correction.applied",
        }
    }

    /// Get the severity level
    pub fn severity(&self) -> &str {
        match self {
            Self::MismatchDetected { severity, .. } => severity,
            Self::BreakingChange { severity, .. } => severity,
            Self::DriftWarning { severity, .. } => severity,
            Self::CorrectionApplied { .. } => "info",
        }
    }

    /// Check if event severity meets threshold
    pub fn meets_severity_threshold(&self, threshold: &str) -> bool {
        let severity = self.severity();
        let severity_order = ["info", "low", "medium", "high", "critical"];

        let severity_idx = severity_order.iter().position(|&s| s == severity).unwrap_or(0);
        let threshold_idx = severity_order.iter().position(|&s| s == threshold).unwrap_or(0);

        severity_idx >= threshold_idx
    }
}

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Webhook URL
    pub url: String,

    /// Events to trigger this webhook
    pub events: Vec<String>,

    /// Minimum severity threshold (info, low, medium, high, critical)
    pub severity_threshold: Option<String>,

    /// HTTP method (default: POST)
    #[serde(default = "default_webhook_method")]
    pub method: String,

    /// Headers to include in webhook request
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Secret for signing webhooks (optional)
    pub secret: Option<String>,

    /// Retry configuration
    #[serde(default)]
    pub retry: RetryConfig,

    /// Timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_webhook_method() -> String {
    "POST".to_string()
}

fn default_timeout() -> u64 {
    30
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            events: Vec::new(),
            severity_threshold: None,
            method: "POST".to_string(),
            headers: HashMap::new(),
            secret: None,
            retry: RetryConfig::default(),
            timeout_secs: 30,
        }
    }
}

/// Retry configuration for webhooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    #[serde(default = "default_max_attempts")]
    pub max_attempts: usize,

    /// Initial delay in seconds
    #[serde(default = "default_initial_delay")]
    pub initial_delay_secs: u64,

    /// Use exponential backoff
    #[serde(default = "default_exponential_backoff")]
    pub exponential_backoff: bool,

    /// Maximum delay in seconds
    #[serde(default = "default_max_delay")]
    pub max_delay_secs: u64,
}

fn default_max_attempts() -> usize {
    3
}

fn default_initial_delay() -> u64 {
    5
}

fn default_exponential_backoff() -> bool {
    true
}

fn default_max_delay() -> u64 {
    60
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_secs: 5,
            exponential_backoff: true,
            max_delay_secs: 60,
        }
    }
}

/// Webhook payload sent to external systems
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    /// Event type
    pub event_type: String,

    /// Timestamp when event occurred
    pub timestamp: DateTime<Utc>,

    /// Event data
    pub data: serde_json::Value,

    /// Severity level
    pub severity: String,

    /// Webhook signature (if secret is configured)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Result of webhook dispatch
#[derive(Debug, Clone)]
pub struct WebhookResult {
    /// Whether the webhook was successfully sent
    pub success: bool,

    /// HTTP status code (if available)
    pub status_code: Option<u16>,

    /// Error message (if failed)
    pub error: Option<String>,

    /// Number of retry attempts made
    pub retry_count: usize,

    /// Response body (if available)
    pub response_body: Option<String>,
}

impl WebhookResult {
    /// Create a successful result
    pub fn success(status_code: u16, response_body: Option<String>) -> Self {
        Self {
            success: true,
            status_code: Some(status_code),
            error: None,
            retry_count: 0,
            response_body,
        }
    }

    /// Create a failed result
    pub fn failure(error: String, retry_count: usize) -> Self {
        Self {
            success: false,
            status_code: None,
            error: Some(error),
            retry_count,
            response_body: None,
        }
    }
}
