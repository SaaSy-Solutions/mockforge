//! SMTP SpecRegistry implementation

use crate::fixtures::{SmtpFixture, StoredEmail};
use mockforge_core::protocol_abstraction::{
    Protocol, ProtocolRequest, ProtocolResponse, ResponseStatus, SpecOperation, SpecRegistry,
    ValidationError, ValidationResult,
};
use mockforge_core::Result;
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Email search filters
#[derive(Debug, Clone, Default)]
pub struct EmailSearchFilters {
    pub sender: Option<String>,
    pub recipient: Option<String>,
    pub subject: Option<String>,
    pub body: Option<String>,
    pub since: Option<chrono::DateTime<chrono::Utc>>,
    pub until: Option<chrono::DateTime<chrono::Utc>>,
    pub use_regex: bool,
    pub case_sensitive: bool,
}

/// SMTP protocol registry implementing SpecRegistry trait
pub struct SmtpSpecRegistry {
    /// Loaded fixtures
    fixtures: Vec<SmtpFixture>,
    /// In-memory mailbox
    mailbox: RwLock<Vec<StoredEmail>>,
    /// Maximum mailbox size
    max_mailbox_size: usize,
}

impl SmtpSpecRegistry {
    /// Create a new SMTP registry
    pub fn new() -> Self {
        Self {
            fixtures: Vec::new(),
            mailbox: RwLock::new(Vec::new()),
            max_mailbox_size: 1000,
        }
    }

    /// Create a new registry with custom mailbox size
    pub fn with_mailbox_size(max_size: usize) -> Self {
        Self {
            fixtures: Vec::new(),
            mailbox: RwLock::new(Vec::new()),
            max_mailbox_size: max_size,
        }
    }

    /// Load fixtures from a directory
    pub fn load_fixtures<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();

        if !path.exists() {
            warn!("Fixtures directory does not exist: {:?}", path);
            return Ok(());
        }

        let entries = std::fs::read_dir(path)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let extension = path.extension().and_then(|s| s.to_str());

                match extension {
                    Some("yaml") | Some("yml") => {
                        self.load_fixture_file(&path)?;
                    }
                    Some("json") => {
                        self.load_fixture_file_json(&path)?;
                    }
                    _ => {
                        debug!("Skipping non-fixture file: {:?}", path);
                    }
                }
            }
        }

        info!("Loaded {} SMTP fixtures", self.fixtures.len());
        Ok(())
    }

    /// Load a single YAML fixture file
    fn load_fixture_file(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let fixture: SmtpFixture = serde_yaml::from_str(&content).map_err(|e| {
            mockforge_core::Error::generic(format!(
                "Failed to parse fixture file {:?}: {}",
                path, e
            ))
        })?;

        debug!("Loaded fixture: {} from {:?}", fixture.name, path);
        self.fixtures.push(fixture);

        Ok(())
    }

    /// Load a single JSON fixture file
    fn load_fixture_file_json(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)?;
        let fixture: SmtpFixture = serde_json::from_str(&content).map_err(|e| {
            mockforge_core::Error::generic(format!(
                "Failed to parse JSON fixture file {:?}: {}",
                path, e
            ))
        })?;

        debug!("Loaded fixture: {} from {:?}", fixture.name, path);
        self.fixtures.push(fixture);

        Ok(())
    }

    /// Find a matching fixture for the given email
    pub fn find_matching_fixture(
        &self,
        from: &str,
        to: &str,
        subject: &str,
    ) -> Option<&SmtpFixture> {
        // First, try to find a specific match
        for fixture in &self.fixtures {
            if !fixture.match_criteria.match_all && fixture.matches(from, to, subject) {
                return Some(fixture);
            }
        }

        // If no specific match, find a default (match_all) fixture
        self.fixtures.iter().find(|f| f.match_criteria.match_all)
    }

    /// Store an email in the mailbox
    pub fn store_email(&self, email: StoredEmail) -> Result<()> {
        let mut mailbox = self.mailbox.write().map_err(|e| {
            mockforge_core::Error::generic(format!("Failed to acquire mailbox lock: {}", e))
        })?;

        // Check mailbox size limit
        if mailbox.len() >= self.max_mailbox_size {
            warn!("Mailbox is full, removing oldest email");
            mailbox.remove(0);
        }

        mailbox.push(email);
        Ok(())
    }

    /// Get all emails from the mailbox
    pub fn get_emails(&self) -> Result<Vec<StoredEmail>> {
        let mailbox = self.mailbox.read().map_err(|e| {
            mockforge_core::Error::generic(format!("Failed to acquire mailbox lock: {}", e))
        })?;

        Ok(mailbox.clone())
    }

    /// Get a specific email by ID
    pub fn get_email_by_id(&self, id: &str) -> Result<Option<StoredEmail>> {
        let mailbox = self.mailbox.read().map_err(|e| {
            mockforge_core::Error::generic(format!("Failed to acquire mailbox lock: {}", e))
        })?;

        Ok(mailbox.iter().find(|e| e.id == id).cloned())
    }

    /// Clear all emails from the mailbox
    pub fn clear_mailbox(&self) -> Result<()> {
        let mut mailbox = self.mailbox.write().map_err(|e| {
            mockforge_core::Error::generic(format!("Failed to acquire mailbox lock: {}", e))
        })?;

        mailbox.clear();
        info!("Mailbox cleared");
        Ok(())
    }

    /// Get mailbox statistics
    pub fn get_mailbox_stats(&self) -> Result<MailboxStats> {
        let mailbox = self.mailbox.read().map_err(|e| {
            mockforge_core::Error::generic(format!("Failed to acquire mailbox lock: {}", e))
        })?;

        Ok(MailboxStats {
            total_emails: mailbox.len(),
            max_capacity: self.max_mailbox_size,
        })
    }

    /// Search emails with filters
    pub fn search_emails(&self, filters: EmailSearchFilters) -> Result<Vec<StoredEmail>> {
        let mailbox = self.mailbox.read().map_err(|e| {
            mockforge_core::Error::generic(format!("Failed to acquire mailbox lock: {}", e))
        })?;

        let mut results: Vec<StoredEmail> = mailbox
            .iter()
            .filter(|email| {
                // Helper function to check if field matches filter
                let matches_filter = |field: &str, filter: &Option<String>| -> bool {
                    if let Some(ref f) = filter {
                        let field_cmp = if filters.case_sensitive {
                            field.to_string()
                        } else {
                            field.to_lowercase()
                        };
                        let filter_cmp = if filters.case_sensitive {
                            f.clone()
                        } else {
                            f.to_lowercase()
                        };

                        if filters.use_regex {
                            if let Ok(re) = Regex::new(&filter_cmp) {
                                re.is_match(&field_cmp)
                            } else {
                                false // invalid regex, no match
                            }
                        } else {
                            field_cmp.contains(&filter_cmp)
                        }
                    } else {
                        true
                    }
                };

                // Filter by sender
                if !matches_filter(&email.from, &filters.sender) {
                    return false;
                }

                // Filter by recipient
                if let Some(ref recipient_filter) = filters.recipient {
                    let has_recipient = email
                        .to
                        .iter()
                        .any(|to| matches_filter(to, &Some(recipient_filter.clone())));
                    if !has_recipient {
                        return false;
                    }
                }

                // Filter by subject
                if !matches_filter(&email.subject, &filters.subject) {
                    return false;
                }

                // Filter by body
                if !matches_filter(&email.body, &filters.body) {
                    return false;
                }

                // Filter by date range
                if let Some(since) = filters.since {
                    if email.received_at < since {
                        return false;
                    }
                }

                if let Some(until) = filters.until {
                    if email.received_at > until {
                        return false;
                    }
                }

                true
            })
            .cloned()
            .collect();

        // Sort by received_at descending (newest first)
        results.sort_by(|a, b| b.received_at.cmp(&a.received_at));

        Ok(results)
    }
}

/// Mailbox statistics
#[derive(Debug, Clone)]
pub struct MailboxStats {
    pub total_emails: usize,
    pub max_capacity: usize,
}

impl Default for SmtpSpecRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SpecRegistry for SmtpSpecRegistry {
    fn protocol(&self) -> Protocol {
        Protocol::Smtp
    }

    fn operations(&self) -> Vec<SpecOperation> {
        self.fixtures
            .iter()
            .map(|fixture| SpecOperation {
                name: fixture.name.clone(),
                path: fixture.identifier.clone(),
                operation_type: "SEND".to_string(),
                input_schema: None,
                output_schema: None,
                metadata: HashMap::from([
                    ("description".to_string(), fixture.description.clone()),
                    ("status_code".to_string(), fixture.response.status_code.to_string()),
                ]),
            })
            .collect()
    }

    fn find_operation(&self, operation: &str, path: &str) -> Option<SpecOperation> {
        self.fixtures
            .iter()
            .find(|f| f.identifier == path)
            .map(|fixture| SpecOperation {
                name: fixture.name.clone(),
                path: fixture.identifier.clone(),
                operation_type: operation.to_string(),
                input_schema: None,
                output_schema: None,
                metadata: HashMap::from([
                    ("description".to_string(), fixture.description.clone()),
                    ("status_code".to_string(), fixture.response.status_code.to_string()),
                ]),
            })
    }

    fn validate_request(&self, request: &ProtocolRequest) -> Result<ValidationResult> {
        // Validate protocol
        if request.protocol != Protocol::Smtp {
            return Ok(ValidationResult::failure(vec![ValidationError {
                message: "Invalid protocol for SMTP registry".to_string(),
                path: None,
                code: Some("INVALID_PROTOCOL".to_string()),
            }]));
        }

        // Basic SMTP validation
        let from = request.metadata.get("from");
        let to = request.metadata.get("to");

        if from.is_none() {
            return Ok(ValidationResult::failure(vec![ValidationError {
                message: "Missing 'from' address".to_string(),
                path: Some("metadata.from".to_string()),
                code: Some("MISSING_FROM".to_string()),
            }]));
        }

        if to.is_none() {
            return Ok(ValidationResult::failure(vec![ValidationError {
                message: "Missing 'to' address".to_string(),
                path: Some("metadata.to".to_string()),
                code: Some("MISSING_TO".to_string()),
            }]));
        }

        Ok(ValidationResult::success())
    }

    fn generate_mock_response(&self, request: &ProtocolRequest) -> Result<ProtocolResponse> {
        let from = request.metadata.get("from").unwrap_or(&String::new()).clone();
        let to = request.metadata.get("to").unwrap_or(&String::new()).clone();
        let subject = request.metadata.get("subject").unwrap_or(&String::new()).clone();

        // Find matching fixture
        let fixture = self
            .find_matching_fixture(&from, &to, &subject)
            .ok_or_else(|| mockforge_core::Error::generic("No matching fixture found for email"))?;

        // Store email if configured
        if fixture.storage.save_to_mailbox {
            let email = StoredEmail {
                id: uuid::Uuid::new_v4().to_string(),
                from: from.clone(),
                to: to.split(',').map(|s| s.trim().to_string()).collect(),
                subject: subject.clone(),
                body: String::from_utf8_lossy(request.body.as_ref().unwrap_or(&Vec::new()))
                    .to_string(),
                headers: request.metadata.clone(),
                received_at: chrono::Utc::now(),
                raw: request.body.clone(),
            };

            if let Err(e) = self.store_email(email) {
                error!("Failed to store email: {}", e);
            }
        }

        // Generate response
        let response_message =
            format!("{} {}\r\n", fixture.response.status_code, fixture.response.message);

        Ok(ProtocolResponse {
            status: ResponseStatus::SmtpStatus(fixture.response.status_code),
            metadata: HashMap::new(),
            body: response_message.into_bytes(),
            content_type: "text/plain".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = SmtpSpecRegistry::new();
        assert_eq!(registry.protocol(), Protocol::Smtp);
        assert_eq!(registry.fixtures.len(), 0);
    }

    #[test]
    fn test_registry_default() {
        let registry = SmtpSpecRegistry::default();
        assert_eq!(registry.protocol(), Protocol::Smtp);
        assert_eq!(registry.max_mailbox_size, 1000);
    }

    #[test]
    fn test_mailbox_operations() {
        let registry = SmtpSpecRegistry::new();

        let email = StoredEmail {
            id: "test-123".to_string(),
            from: "sender@example.com".to_string(),
            to: vec!["recipient@example.com".to_string()],
            subject: "Test".to_string(),
            body: "Test body".to_string(),
            headers: HashMap::new(),
            received_at: chrono::Utc::now(),
            raw: None,
        };

        registry.store_email(email.clone()).unwrap();

        let emails = registry.get_emails().unwrap();
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].from, "sender@example.com");

        registry.clear_mailbox().unwrap();
        let emails = registry.get_emails().unwrap();
        assert_eq!(emails.len(), 0);
    }

    #[test]
    fn test_mailbox_size_limit() {
        let registry = SmtpSpecRegistry::with_mailbox_size(2);

        for i in 0..5 {
            let email = StoredEmail {
                id: format!("test-{}", i),
                from: "sender@example.com".to_string(),
                to: vec!["recipient@example.com".to_string()],
                subject: format!("Test {}", i),
                body: "Test body".to_string(),
                headers: HashMap::new(),
                received_at: chrono::Utc::now(),
                raw: None,
            };

            registry.store_email(email).unwrap();
        }

        let emails = registry.get_emails().unwrap();
        assert_eq!(emails.len(), 2); // Should only keep the last 2
    }

    #[test]
    fn test_get_email_by_id() {
        let registry = SmtpSpecRegistry::new();

        let email = StoredEmail {
            id: "unique-id-123".to_string(),
            from: "sender@example.com".to_string(),
            to: vec!["recipient@example.com".to_string()],
            subject: "Test".to_string(),
            body: "Test body".to_string(),
            headers: HashMap::new(),
            received_at: chrono::Utc::now(),
            raw: None,
        };

        registry.store_email(email).unwrap();

        let found = registry.get_email_by_id("unique-id-123").unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, "unique-id-123");

        let not_found = registry.get_email_by_id("nonexistent").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_mailbox_stats() {
        let registry = SmtpSpecRegistry::with_mailbox_size(100);

        let stats = registry.get_mailbox_stats().unwrap();
        assert_eq!(stats.total_emails, 0);
        assert_eq!(stats.max_capacity, 100);

        for i in 0..5 {
            let email = StoredEmail {
                id: format!("test-{}", i),
                from: "sender@example.com".to_string(),
                to: vec!["recipient@example.com".to_string()],
                subject: format!("Test {}", i),
                body: "Test body".to_string(),
                headers: HashMap::new(),
                received_at: chrono::Utc::now(),
                raw: None,
            };
            registry.store_email(email).unwrap();
        }

        let stats = registry.get_mailbox_stats().unwrap();
        assert_eq!(stats.total_emails, 5);
    }

    #[test]
    fn test_email_search_filters_default() {
        let filters = EmailSearchFilters::default();
        assert!(filters.sender.is_none());
        assert!(filters.recipient.is_none());
        assert!(filters.subject.is_none());
        assert!(filters.body.is_none());
        assert!(!filters.use_regex);
        assert!(!filters.case_sensitive);
    }

    #[test]
    fn test_search_emails_by_sender() {
        let registry = SmtpSpecRegistry::new();

        // Add test emails
        for i in 0..3 {
            let email = StoredEmail {
                id: format!("test-{}", i),
                from: format!("sender{}@example.com", i),
                to: vec!["recipient@example.com".to_string()],
                subject: "Test".to_string(),
                body: "Test body".to_string(),
                headers: HashMap::new(),
                received_at: chrono::Utc::now(),
                raw: None,
            };
            registry.store_email(email).unwrap();
        }

        let filters = EmailSearchFilters {
            sender: Some("sender1".to_string()),
            ..Default::default()
        };
        let results = registry.search_emails(filters).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].from.contains("sender1"));
    }

    #[test]
    fn test_search_emails_by_subject() {
        let registry = SmtpSpecRegistry::new();

        let email1 = StoredEmail {
            id: "test-1".to_string(),
            from: "sender@example.com".to_string(),
            to: vec!["recipient@example.com".to_string()],
            subject: "Important update".to_string(),
            body: "Test body".to_string(),
            headers: HashMap::new(),
            received_at: chrono::Utc::now(),
            raw: None,
        };
        let email2 = StoredEmail {
            id: "test-2".to_string(),
            from: "sender@example.com".to_string(),
            to: vec!["recipient@example.com".to_string()],
            subject: "Newsletter".to_string(),
            body: "Test body".to_string(),
            headers: HashMap::new(),
            received_at: chrono::Utc::now(),
            raw: None,
        };

        registry.store_email(email1).unwrap();
        registry.store_email(email2).unwrap();

        let filters = EmailSearchFilters {
            subject: Some("Important".to_string()),
            ..Default::default()
        };
        let results = registry.search_emails(filters).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].subject.contains("Important"));
    }

    #[test]
    fn test_search_emails_with_regex() {
        let registry = SmtpSpecRegistry::new();

        let email = StoredEmail {
            id: "test-1".to_string(),
            from: "admin@example.com".to_string(),
            to: vec!["recipient@example.com".to_string()],
            subject: "Test".to_string(),
            body: "Test body".to_string(),
            headers: HashMap::new(),
            received_at: chrono::Utc::now(),
            raw: None,
        };
        registry.store_email(email).unwrap();

        let filters = EmailSearchFilters {
            sender: Some(r"^admin@.*\.com$".to_string()),
            use_regex: true,
            ..Default::default()
        };
        let results = registry.search_emails(filters).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_operations_empty() {
        let registry = SmtpSpecRegistry::new();
        let ops = registry.operations();
        assert!(ops.is_empty());
    }

    #[test]
    fn test_find_operation_not_found() {
        let registry = SmtpSpecRegistry::new();
        let op = registry.find_operation("SEND", "/nonexistent");
        assert!(op.is_none());
    }

    #[test]
    fn test_validate_request_missing_from() {
        let registry = SmtpSpecRegistry::new();
        let request = ProtocolRequest {
            protocol: Protocol::Smtp,
            pattern: mockforge_core::protocol_abstraction::MessagePattern::OneWay,
            operation: "SEND".to_string(),
            path: "/".to_string(),
            topic: None,
            routing_key: None,
            partition: None,
            qos: None,
            metadata: HashMap::from([("to".to_string(), "recipient@example.com".to_string())]),
            body: None,
            client_ip: None,
        };

        let result = registry.validate_request(&request).unwrap();
        assert!(!result.valid);
    }

    #[test]
    fn test_validate_request_missing_to() {
        let registry = SmtpSpecRegistry::new();
        let request = ProtocolRequest {
            protocol: Protocol::Smtp,
            pattern: mockforge_core::protocol_abstraction::MessagePattern::OneWay,
            operation: "SEND".to_string(),
            path: "/".to_string(),
            topic: None,
            routing_key: None,
            partition: None,
            qos: None,
            metadata: HashMap::from([("from".to_string(), "sender@example.com".to_string())]),
            body: None,
            client_ip: None,
        };

        let result = registry.validate_request(&request).unwrap();
        assert!(!result.valid);
    }

    #[test]
    fn test_validate_request_valid() {
        let registry = SmtpSpecRegistry::new();
        let request = ProtocolRequest {
            protocol: Protocol::Smtp,
            pattern: mockforge_core::protocol_abstraction::MessagePattern::OneWay,
            operation: "SEND".to_string(),
            path: "/".to_string(),
            topic: None,
            routing_key: None,
            partition: None,
            qos: None,
            metadata: HashMap::from([
                ("from".to_string(), "sender@example.com".to_string()),
                ("to".to_string(), "recipient@example.com".to_string()),
            ]),
            body: None,
            client_ip: None,
        };

        let result = registry.validate_request(&request).unwrap();
        assert!(result.valid);
    }

    #[test]
    fn test_validate_request_wrong_protocol() {
        let registry = SmtpSpecRegistry::new();
        let request = ProtocolRequest {
            protocol: Protocol::Http,
            pattern: mockforge_core::protocol_abstraction::MessagePattern::OneWay,
            operation: "SEND".to_string(),
            path: "/".to_string(),
            topic: None,
            routing_key: None,
            partition: None,
            qos: None,
            metadata: HashMap::new(),
            body: None,
            client_ip: None,
        };

        let result = registry.validate_request(&request).unwrap();
        assert!(!result.valid);
    }

    #[test]
    fn test_load_fixtures_nonexistent_dir() {
        let mut registry = SmtpSpecRegistry::new();
        let result = registry.load_fixtures("/nonexistent/path");
        // Should succeed but not load any fixtures
        assert!(result.is_ok());
        assert_eq!(registry.fixtures.len(), 0);
    }
}
