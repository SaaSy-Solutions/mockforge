//! SMTP fixture definitions and loading

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An SMTP fixture defining how to handle emails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpFixture {
    /// Unique identifier for this fixture
    pub identifier: String,

    /// Human-readable name
    pub name: String,

    /// Description of what this fixture does
    #[serde(default)]
    pub description: String,

    /// Matching criteria for emails
    pub match_criteria: MatchCriteria,

    /// Response configuration
    pub response: SmtpResponse,

    /// Auto-reply configuration
    #[serde(default)]
    pub auto_reply: Option<AutoReply>,

    /// Storage configuration
    #[serde(default)]
    pub storage: StorageConfig,

    /// Behavior simulation
    #[serde(default)]
    pub behavior: BehaviorConfig,
}

/// Criteria for matching incoming emails
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchCriteria {
    /// Match by recipient pattern (regex)
    #[serde(default)]
    pub recipient_pattern: Option<String>,

    /// Match by sender pattern (regex)
    #[serde(default)]
    pub sender_pattern: Option<String>,

    /// Match by subject pattern (regex)
    #[serde(default)]
    pub subject_pattern: Option<String>,

    /// Match all (default fixture)
    #[serde(default)]
    pub match_all: bool,
}

/// SMTP response configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpResponse {
    /// SMTP status code (250 = success, 550 = reject)
    pub status_code: u16,

    /// Status message
    pub message: String,

    /// Delay before responding (milliseconds)
    #[serde(default)]
    pub delay_ms: u64,
}

/// Auto-reply email configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoReply {
    /// Enable auto-reply
    pub enabled: bool,

    /// From address
    pub from: String,

    /// To address (supports template: {{metadata.from}})
    pub to: String,

    /// Email subject
    pub subject: String,

    /// Email body (supports templates)
    pub body: String,

    /// Optional HTML body
    #[serde(default)]
    pub html_body: Option<String>,

    /// Custom headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

/// Email storage configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageConfig {
    /// Save emails to mailbox
    #[serde(default)]
    pub save_to_mailbox: bool,

    /// Export to file
    #[serde(default)]
    pub export_to_file: Option<ExportConfig>,
}

/// File export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// Enable export
    pub enabled: bool,

    /// File path (supports templates)
    pub path: String,
}

/// Behavior simulation configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorConfig {
    /// Failure rate (0.0 - 1.0)
    #[serde(default)]
    pub failure_rate: f64,

    /// Latency range
    #[serde(default)]
    pub latency: Option<LatencyConfig>,
}

/// Latency configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyConfig {
    /// Minimum latency in milliseconds
    pub min_ms: u64,

    /// Maximum latency in milliseconds
    pub max_ms: u64,
}

/// Stored email message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEmail {
    /// Unique ID
    pub id: String,

    /// From address
    pub from: String,

    /// To addresses
    pub to: Vec<String>,

    /// Subject
    pub subject: String,

    /// Body content
    pub body: String,

    /// Headers
    pub headers: HashMap<String, String>,

    /// Received timestamp
    pub received_at: chrono::DateTime<chrono::Utc>,

    /// Raw email data
    #[serde(default)]
    pub raw: Option<Vec<u8>>,
}

impl SmtpFixture {
    /// Check if this fixture matches the given email criteria
    pub fn matches(&self, from: &str, to: &str, subject: &str) -> bool {
        use regex::Regex;

        // If match_all is true, this fixture matches everything
        if self.match_criteria.match_all {
            return true;
        }

        // Check recipient pattern
        if let Some(pattern) = &self.match_criteria.recipient_pattern {
            if let Ok(re) = Regex::new(pattern) {
                if !re.is_match(to) {
                    return false;
                }
            }
        }

        // Check sender pattern
        if let Some(pattern) = &self.match_criteria.sender_pattern {
            if let Ok(re) = Regex::new(pattern) {
                if !re.is_match(from) {
                    return false;
                }
            }
        }

        // Check subject pattern
        if let Some(pattern) = &self.match_criteria.subject_pattern {
            if let Ok(re) = Regex::new(pattern) {
                if !re.is_match(subject) {
                    return false;
                }
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixture_matching() {
        let fixture = SmtpFixture {
            identifier: "test".to_string(),
            name: "Test Fixture".to_string(),
            description: "".to_string(),
            match_criteria: MatchCriteria {
                recipient_pattern: Some(r"^user.*@example\.com$".to_string()),
                sender_pattern: None,
                subject_pattern: None,
                match_all: false,
            },
            response: SmtpResponse {
                status_code: 250,
                message: "OK".to_string(),
                delay_ms: 0,
            },
            auto_reply: None,
            storage: StorageConfig::default(),
            behavior: BehaviorConfig::default(),
        };

        assert!(fixture.matches("sender@test.com", "user123@example.com", "Test"));
        assert!(!fixture.matches("sender@test.com", "admin@example.com", "Test"));
    }

    #[test]
    fn test_match_all_fixture() {
        let fixture = SmtpFixture {
            identifier: "default".to_string(),
            name: "Default Fixture".to_string(),
            description: "".to_string(),
            match_criteria: MatchCriteria {
                recipient_pattern: None,
                sender_pattern: None,
                subject_pattern: None,
                match_all: true,
            },
            response: SmtpResponse {
                status_code: 250,
                message: "OK".to_string(),
                delay_ms: 0,
            },
            auto_reply: None,
            storage: StorageConfig::default(),
            behavior: BehaviorConfig::default(),
        };

        assert!(fixture.matches("any@sender.com", "any@recipient.com", "Any Subject"));
    }
}
