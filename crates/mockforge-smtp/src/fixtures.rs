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

    #[test]
    fn test_fixture_sender_pattern() {
        let fixture = SmtpFixture {
            identifier: "test".to_string(),
            name: "Test Fixture".to_string(),
            description: "".to_string(),
            match_criteria: MatchCriteria {
                recipient_pattern: None,
                sender_pattern: Some(r"^admin@.*$".to_string()),
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

        assert!(fixture.matches("admin@example.com", "recipient@example.com", "Test"));
        assert!(!fixture.matches("user@example.com", "recipient@example.com", "Test"));
    }

    #[test]
    fn test_fixture_subject_pattern() {
        let fixture = SmtpFixture {
            identifier: "test".to_string(),
            name: "Test Fixture".to_string(),
            description: "".to_string(),
            match_criteria: MatchCriteria {
                recipient_pattern: None,
                sender_pattern: None,
                subject_pattern: Some(r"^Important:.*$".to_string()),
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

        assert!(fixture.matches(
            "sender@example.com",
            "recipient@example.com",
            "Important: Action required"
        ));
        assert!(!fixture.matches("sender@example.com", "recipient@example.com", "Regular subject"));
    }

    #[test]
    fn test_match_criteria_default() {
        let criteria = MatchCriteria::default();
        assert!(criteria.recipient_pattern.is_none());
        assert!(criteria.sender_pattern.is_none());
        assert!(criteria.subject_pattern.is_none());
        assert!(!criteria.match_all);
    }

    #[test]
    fn test_storage_config_default() {
        let config = StorageConfig::default();
        assert!(!config.save_to_mailbox);
        assert!(config.export_to_file.is_none());
    }

    #[test]
    fn test_behavior_config_default() {
        let config = BehaviorConfig::default();
        assert_eq!(config.failure_rate, 0.0);
        assert!(config.latency.is_none());
    }

    #[test]
    fn test_stored_email_serialize() {
        let email = StoredEmail {
            id: "test-123".to_string(),
            from: "sender@example.com".to_string(),
            to: vec!["recipient@example.com".to_string()],
            subject: "Test Subject".to_string(),
            body: "Test body content".to_string(),
            headers: HashMap::from([("Content-Type".to_string(), "text/plain".to_string())]),
            received_at: chrono::Utc::now(),
            raw: None,
        };

        let json = serde_json::to_string(&email).unwrap();
        assert!(json.contains("test-123"));
        assert!(json.contains("sender@example.com"));
        assert!(json.contains("Test Subject"));
    }

    #[test]
    fn test_stored_email_deserialize() {
        let json = r#"{
            "id": "email-456",
            "from": "alice@example.com",
            "to": ["bob@example.com", "carol@example.com"],
            "subject": "Hello",
            "body": "Hi there!",
            "headers": {},
            "received_at": "2024-01-15T12:00:00Z"
        }"#;
        let email: StoredEmail = serde_json::from_str(json).unwrap();
        assert_eq!(email.id, "email-456");
        assert_eq!(email.from, "alice@example.com");
        assert_eq!(email.to.len(), 2);
    }

    #[test]
    fn test_smtp_fixture_serialize() {
        let fixture = SmtpFixture {
            identifier: "test".to_string(),
            name: "Test Fixture".to_string(),
            description: "A test fixture".to_string(),
            match_criteria: MatchCriteria::default(),
            response: SmtpResponse {
                status_code: 250,
                message: "OK".to_string(),
                delay_ms: 100,
            },
            auto_reply: None,
            storage: StorageConfig::default(),
            behavior: BehaviorConfig::default(),
        };

        let json = serde_json::to_string(&fixture).unwrap();
        assert!(json.contains("Test Fixture"));
        assert!(json.contains("250"));
    }

    #[test]
    fn test_smtp_response_with_delay() {
        let response = SmtpResponse {
            status_code: 550,
            message: "Mailbox unavailable".to_string(),
            delay_ms: 500,
        };
        assert_eq!(response.status_code, 550);
        assert_eq!(response.delay_ms, 500);
    }

    #[test]
    fn test_auto_reply_config() {
        let auto_reply = AutoReply {
            enabled: true,
            from: "noreply@example.com".to_string(),
            to: "{{metadata.from}}".to_string(),
            subject: "Auto Reply".to_string(),
            body: "Thank you for your email.".to_string(),
            html_body: Some("<p>Thank you for your email.</p>".to_string()),
            headers: HashMap::from([("X-Auto-Reply".to_string(), "true".to_string())]),
        };

        assert!(auto_reply.enabled);
        assert!(auto_reply.html_body.is_some());
    }

    #[test]
    fn test_latency_config() {
        let latency = LatencyConfig {
            min_ms: 100,
            max_ms: 500,
        };
        assert_eq!(latency.min_ms, 100);
        assert_eq!(latency.max_ms, 500);
    }

    #[test]
    fn test_export_config() {
        let export = ExportConfig {
            enabled: true,
            path: "/tmp/emails/{{metadata.from}}/{{timestamp}}.eml".to_string(),
        };
        assert!(export.enabled);
        assert!(export.path.contains("{{metadata.from}}"));
    }

    #[test]
    fn test_fixture_combined_matching() {
        let fixture = SmtpFixture {
            identifier: "combined".to_string(),
            name: "Combined Match".to_string(),
            description: "".to_string(),
            match_criteria: MatchCriteria {
                recipient_pattern: Some(r".*@example\.com$".to_string()),
                sender_pattern: Some(r"^admin@.*$".to_string()),
                subject_pattern: Some(r"^Urgent:.*$".to_string()),
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

        // All patterns match
        assert!(fixture.matches("admin@test.com", "user@example.com", "Urgent: Review needed"));

        // Recipient doesn't match
        assert!(!fixture.matches("admin@test.com", "user@other.com", "Urgent: Review needed"));

        // Sender doesn't match
        assert!(!fixture.matches("user@test.com", "user@example.com", "Urgent: Review needed"));

        // Subject doesn't match
        assert!(!fixture.matches("admin@test.com", "user@example.com", "Regular subject"));
    }

    #[test]
    fn test_stored_email_clone() {
        let email = StoredEmail {
            id: "test-clone".to_string(),
            from: "sender@example.com".to_string(),
            to: vec!["recipient@example.com".to_string()],
            subject: "Clone Test".to_string(),
            body: "Test body".to_string(),
            headers: HashMap::new(),
            received_at: chrono::Utc::now(),
            raw: Some(vec![1, 2, 3]),
        };

        let cloned = email.clone();
        assert_eq!(email.id, cloned.id);
        assert_eq!(email.from, cloned.from);
        assert_eq!(email.raw, cloned.raw);
    }

    #[test]
    fn test_fixture_debug() {
        let fixture = SmtpFixture {
            identifier: "debug-test".to_string(),
            name: "Debug Test".to_string(),
            description: "".to_string(),
            match_criteria: MatchCriteria::default(),
            response: SmtpResponse {
                status_code: 250,
                message: "OK".to_string(),
                delay_ms: 0,
            },
            auto_reply: None,
            storage: StorageConfig::default(),
            behavior: BehaviorConfig::default(),
        };

        let debug = format!("{:?}", fixture);
        assert!(debug.contains("SmtpFixture"));
        assert!(debug.contains("debug-test"));
    }
}
