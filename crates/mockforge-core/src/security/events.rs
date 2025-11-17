//! Security event definitions for MockForge
//!
//! This module defines all security events that can be emitted by MockForge for SIEM integration
//! and compliance monitoring. Events are categorized by type (authentication, authorization, access
//! management, configuration, data, security, compliance) and include compliance mapping for
//! SOC 2 and ISO 27001.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Security event severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SecurityEventSeverity {
    /// Low severity - informational events
    Low,
    /// Medium severity - events requiring attention
    Medium,
    /// High severity - security concerns
    High,
    /// Critical severity - immediate security threats
    Critical,
}

/// Security event type categories
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEventType {
    // Authentication events
    /// Successful authentication
    AuthSuccess,
    /// Failed authentication attempt
    AuthFailure,
    /// Token expiration
    AuthTokenExpired,
    /// Token revocation
    AuthTokenRevoked,
    /// Multi-factor authentication enabled
    AuthMfaEnabled,
    /// Multi-factor authentication disabled
    AuthMfaDisabled,
    /// Password change
    AuthPasswordChanged,
    /// Password reset
    AuthPasswordReset,

    // Authorization events
    /// Access granted
    AuthzAccessGranted,
    /// Access denied
    AuthzAccessDenied,
    /// Privilege escalation
    AuthzPrivilegeEscalation,
    /// Role change
    AuthzRoleChanged,
    /// Permission change
    AuthzPermissionChanged,

    // Access management events
    /// User account created
    AccessUserCreated,
    /// User account deleted
    AccessUserDeleted,
    /// User account suspended
    AccessUserSuspended,
    /// User account activated
    AccessUserActivated,
    /// API token created
    AccessApiTokenCreated,
    /// API token deleted
    AccessApiTokenDeleted,
    /// API token rotated
    AccessApiTokenRotated,

    // Configuration events
    /// Configuration changed
    ConfigChanged,
    /// Security policy updated
    ConfigSecurityPolicyUpdated,
    /// Encryption key rotated
    ConfigEncryptionKeyRotated,
    /// TLS certificate updated
    ConfigTlsCertificateUpdated,

    // Data events
    /// Data exported
    DataExported,
    /// Data deleted
    DataDeleted,
    /// Data encrypted
    DataEncrypted,
    /// Data decrypted
    DataDecrypted,
    /// Data classified
    DataClassified,

    // Security events
    /// Vulnerability detected
    SecurityVulnerabilityDetected,
    /// Threat detected
    SecurityThreatDetected,
    /// Anomaly detected
    SecurityAnomalyDetected,
    /// Rate limit exceeded
    SecurityRateLimitExceeded,
    /// Suspicious activity detected
    SecuritySuspiciousActivity,

    // Compliance events
    /// Audit log accessed
    ComplianceAuditLogAccessed,
    /// Compliance check performed
    ComplianceComplianceCheck,
    /// Policy violation detected
    CompliancePolicyViolation,
}

impl SecurityEventType {
    /// Get the event type string identifier (e.g., "auth.success")
    pub fn as_str(&self) -> &'static str {
        match self {
            SecurityEventType::AuthSuccess => "auth.success",
            SecurityEventType::AuthFailure => "auth.failure",
            SecurityEventType::AuthTokenExpired => "auth.token_expired",
            SecurityEventType::AuthTokenRevoked => "auth.token_revoked",
            SecurityEventType::AuthMfaEnabled => "auth.mfa_enabled",
            SecurityEventType::AuthMfaDisabled => "auth.mfa_disabled",
            SecurityEventType::AuthPasswordChanged => "auth.password_changed",
            SecurityEventType::AuthPasswordReset => "auth.password_reset",
            SecurityEventType::AuthzAccessGranted => "authz.access_granted",
            SecurityEventType::AuthzAccessDenied => "authz.access_denied",
            SecurityEventType::AuthzPrivilegeEscalation => "authz.privilege_escalation",
            SecurityEventType::AuthzRoleChanged => "authz.role_changed",
            SecurityEventType::AuthzPermissionChanged => "authz.permission_changed",
            SecurityEventType::AccessUserCreated => "access.user_created",
            SecurityEventType::AccessUserDeleted => "access.user_deleted",
            SecurityEventType::AccessUserSuspended => "access.user_suspended",
            SecurityEventType::AccessUserActivated => "access.user_activated",
            SecurityEventType::AccessApiTokenCreated => "access.api_token_created",
            SecurityEventType::AccessApiTokenDeleted => "access.api_token_deleted",
            SecurityEventType::AccessApiTokenRotated => "access.api_token_rotated",
            SecurityEventType::ConfigChanged => "config.changed",
            SecurityEventType::ConfigSecurityPolicyUpdated => "config.security_policy_updated",
            SecurityEventType::ConfigEncryptionKeyRotated => "config.encryption_key_rotated",
            SecurityEventType::ConfigTlsCertificateUpdated => "config.tls_certificate_updated",
            SecurityEventType::DataExported => "data.exported",
            SecurityEventType::DataDeleted => "data.deleted",
            SecurityEventType::DataEncrypted => "data.encrypted",
            SecurityEventType::DataDecrypted => "data.decrypted",
            SecurityEventType::DataClassified => "data.classified",
            SecurityEventType::SecurityVulnerabilityDetected => "security.vulnerability_detected",
            SecurityEventType::SecurityThreatDetected => "security.threat_detected",
            SecurityEventType::SecurityAnomalyDetected => "security.anomaly_detected",
            SecurityEventType::SecurityRateLimitExceeded => "security.rate_limit_exceeded",
            SecurityEventType::SecuritySuspiciousActivity => "security.suspicious_activity",
            SecurityEventType::ComplianceAuditLogAccessed => "compliance.audit_log_accessed",
            SecurityEventType::ComplianceComplianceCheck => "compliance.compliance_check",
            SecurityEventType::CompliancePolicyViolation => "compliance.policy_violation",
        }
    }

    /// Get the default severity for this event type
    pub fn default_severity(&self) -> SecurityEventSeverity {
        match self {
            SecurityEventType::AuthSuccess
            | SecurityEventType::AuthzAccessGranted
            | SecurityEventType::AccessUserCreated
            | SecurityEventType::AccessUserActivated
            | SecurityEventType::AccessApiTokenCreated
            | SecurityEventType::AuthMfaEnabled
            | SecurityEventType::DataEncrypted
            | SecurityEventType::DataClassified
            | SecurityEventType::ComplianceComplianceCheck => SecurityEventSeverity::Low,

            SecurityEventType::AuthFailure
            | SecurityEventType::AuthTokenExpired
            | SecurityEventType::AuthTokenRevoked
            | SecurityEventType::AuthMfaDisabled
            | SecurityEventType::AuthPasswordChanged
            | SecurityEventType::AuthPasswordReset
            | SecurityEventType::AuthzAccessDenied
            | SecurityEventType::AuthzRoleChanged
            | SecurityEventType::AuthzPermissionChanged
            | SecurityEventType::AccessUserSuspended
            | SecurityEventType::AccessApiTokenDeleted
            | SecurityEventType::ConfigChanged
            | SecurityEventType::DataExported
            | SecurityEventType::DataDeleted
            | SecurityEventType::SecurityRateLimitExceeded
            | SecurityEventType::ComplianceAuditLogAccessed => SecurityEventSeverity::Medium,

            SecurityEventType::AuthzPrivilegeEscalation
            | SecurityEventType::AccessUserDeleted
            | SecurityEventType::AccessApiTokenRotated
            | SecurityEventType::ConfigSecurityPolicyUpdated
            | SecurityEventType::ConfigEncryptionKeyRotated
            | SecurityEventType::ConfigTlsCertificateUpdated
            | SecurityEventType::DataDecrypted
            | SecurityEventType::SecurityThreatDetected
            | SecurityEventType::SecurityAnomalyDetected
            | SecurityEventType::SecuritySuspiciousActivity
            | SecurityEventType::CompliancePolicyViolation => SecurityEventSeverity::High,

            SecurityEventType::SecurityVulnerabilityDetected => SecurityEventSeverity::Critical,
        }
    }

    /// Get SOC 2 Common Criteria mappings for this event type
    pub fn soc2_cc(&self) -> Vec<&'static str> {
        match self {
            SecurityEventType::AuthSuccess
            | SecurityEventType::AuthFailure
            | SecurityEventType::AuthTokenExpired
            | SecurityEventType::AuthTokenRevoked
            | SecurityEventType::AuthMfaEnabled
            | SecurityEventType::AuthMfaDisabled
            | SecurityEventType::AuthPasswordChanged
            | SecurityEventType::AuthPasswordReset
            | SecurityEventType::AccessUserCreated
            | SecurityEventType::AccessUserDeleted
            | SecurityEventType::AccessUserSuspended
            | SecurityEventType::AccessUserActivated
            | SecurityEventType::AccessApiTokenCreated
            | SecurityEventType::AccessApiTokenDeleted
            | SecurityEventType::AccessApiTokenRotated
            | SecurityEventType::AuthzPrivilegeEscalation
            | SecurityEventType::AuthzRoleChanged
            | SecurityEventType::AuthzPermissionChanged => vec!["CC6"],

            SecurityEventType::AuthzAccessGranted | SecurityEventType::AuthzAccessDenied => {
                vec!["CC6"]
            }

            SecurityEventType::ConfigChanged
            | SecurityEventType::ConfigSecurityPolicyUpdated
            | SecurityEventType::ConfigEncryptionKeyRotated
            | SecurityEventType::ConfigTlsCertificateUpdated => vec!["CC7"],

            SecurityEventType::DataExported
            | SecurityEventType::DataDeleted
            | SecurityEventType::DataEncrypted
            | SecurityEventType::DataDecrypted
            | SecurityEventType::DataClassified
            | SecurityEventType::SecurityVulnerabilityDetected
            | SecurityEventType::SecurityThreatDetected
            | SecurityEventType::SecurityAnomalyDetected
            | SecurityEventType::SecurityRateLimitExceeded
            | SecurityEventType::SecuritySuspiciousActivity
            | SecurityEventType::ComplianceAuditLogAccessed
            | SecurityEventType::ComplianceComplianceCheck
            | SecurityEventType::CompliancePolicyViolation => vec!["CC4"],
        }
    }

    /// Get ISO 27001 control mappings for this event type
    pub fn iso27001(&self) -> Vec<&'static str> {
        match self {
            SecurityEventType::AuthSuccess
            | SecurityEventType::AuthFailure
            | SecurityEventType::AuthTokenExpired
            | SecurityEventType::AuthTokenRevoked
            | SecurityEventType::AuthMfaEnabled
            | SecurityEventType::AuthMfaDisabled
            | SecurityEventType::AuthPasswordChanged
            | SecurityEventType::AuthPasswordReset
            | SecurityEventType::AccessUserCreated
            | SecurityEventType::AccessUserDeleted
            | SecurityEventType::AccessUserSuspended
            | SecurityEventType::AccessUserActivated
            | SecurityEventType::AccessApiTokenCreated
            | SecurityEventType::AccessApiTokenDeleted
            | SecurityEventType::AccessApiTokenRotated
            | SecurityEventType::AuthzPrivilegeEscalation
            | SecurityEventType::AuthzRoleChanged
            | SecurityEventType::AuthzPermissionChanged => vec!["A.9.2"],

            SecurityEventType::AuthzAccessGranted | SecurityEventType::AuthzAccessDenied => {
                vec!["A.9.4"]
            }

            SecurityEventType::ConfigChanged
            | SecurityEventType::ConfigSecurityPolicyUpdated
            | SecurityEventType::ConfigEncryptionKeyRotated
            | SecurityEventType::ConfigTlsCertificateUpdated => vec!["A.12.1"],

            SecurityEventType::DataExported
            | SecurityEventType::DataDeleted
            | SecurityEventType::DataEncrypted
            | SecurityEventType::DataDecrypted
            | SecurityEventType::DataClassified
            | SecurityEventType::SecurityVulnerabilityDetected
            | SecurityEventType::SecurityThreatDetected
            | SecurityEventType::SecurityAnomalyDetected
            | SecurityEventType::SecurityRateLimitExceeded
            | SecurityEventType::SecuritySuspiciousActivity
            | SecurityEventType::ComplianceAuditLogAccessed
            | SecurityEventType::ComplianceComplianceCheck
            | SecurityEventType::CompliancePolicyViolation => vec!["A.12.4"],
        }
    }
}

/// Source information for a security event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventSource {
    /// System name (e.g., "mockforge")
    pub system: String,
    /// Component name (e.g., "auth", "api")
    pub component: String,
    /// System version
    pub version: String,
}

impl Default for EventSource {
    fn default() -> Self {
        Self {
            system: "mockforge".to_string(),
            component: "core".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Actor information for a security event (who performed the action)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventActor {
    /// User identifier
    pub user_id: Option<String>,
    /// Username
    pub username: Option<String>,
    /// IP address
    pub ip_address: Option<String>,
    /// User agent
    pub user_agent: Option<String>,
}

/// Target resource information for a security event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTarget {
    /// Resource type (e.g., "api", "workspace", "user")
    pub resource_type: Option<String>,
    /// Resource identifier
    pub resource_id: Option<String>,
    /// HTTP method (if applicable)
    pub method: Option<String>,
}

/// Outcome information for a security event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventOutcome {
    /// Whether the action was successful
    pub success: bool,
    /// Reason for success or failure
    pub reason: Option<String>,
}

/// Compliance mapping for a security event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventCompliance {
    /// SOC 2 Common Criteria
    pub soc2_cc: Vec<String>,
    /// ISO 27001 controls
    pub iso27001: Vec<String>,
}

/// Security event structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Event timestamp (ISO 8601)
    pub timestamp: DateTime<Utc>,
    /// Event type identifier (e.g., "auth.failure")
    pub event_type: String,
    /// Event severity
    pub severity: SecurityEventSeverity,
    /// Source information
    pub source: EventSource,
    /// Actor information (who performed the action)
    pub actor: Option<EventActor>,
    /// Target resource information
    pub target: Option<EventTarget>,
    /// Outcome information
    pub outcome: Option<EventOutcome>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Compliance mappings
    pub compliance: EventCompliance,
}

impl SecurityEvent {
    /// Create a new security event
    pub fn new(
        event_type: SecurityEventType,
        severity: Option<SecurityEventSeverity>,
        source: Option<EventSource>,
    ) -> Self {
        let default_severity = event_type.default_severity();
        let severity = severity.unwrap_or(default_severity);

        Self {
            timestamp: Utc::now(),
            event_type: event_type.as_str().to_string(),
            severity,
            source: source.unwrap_or_default(),
            actor: None,
            target: None,
            outcome: None,
            metadata: HashMap::new(),
            compliance: EventCompliance {
                soc2_cc: event_type.soc2_cc().iter().map(|s| s.to_string()).collect(),
                iso27001: event_type.iso27001().iter().map(|s| s.to_string()).collect(),
            },
        }
    }

    /// Set the actor information
    pub fn with_actor(mut self, actor: EventActor) -> Self {
        self.actor = Some(actor);
        self
    }

    /// Set the target resource information
    pub fn with_target(mut self, target: EventTarget) -> Self {
        self.target = Some(target);
        self
    }

    /// Set the outcome information
    pub fn with_outcome(mut self, outcome: EventOutcome) -> Self {
        self.outcome = Some(outcome);
        self
    }

    /// Add metadata key-value pair
    pub fn with_metadata(mut self, key: String, value: serde_json::Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Convert to JSON value
    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_type_strings() {
        assert_eq!(SecurityEventType::AuthSuccess.as_str(), "auth.success");
        assert_eq!(SecurityEventType::AuthFailure.as_str(), "auth.failure");
        assert_eq!(
            SecurityEventType::AuthzPrivilegeEscalation.as_str(),
            "authz.privilege_escalation"
        );
    }

    #[test]
    fn test_event_severity() {
        assert_eq!(SecurityEventType::AuthSuccess.default_severity(), SecurityEventSeverity::Low);
        assert_eq!(
            SecurityEventType::AuthFailure.default_severity(),
            SecurityEventSeverity::Medium
        );
        assert_eq!(
            SecurityEventType::AuthzPrivilegeEscalation.default_severity(),
            SecurityEventSeverity::High
        );
        assert_eq!(
            SecurityEventType::SecurityVulnerabilityDetected.default_severity(),
            SecurityEventSeverity::Critical
        );
    }

    #[test]
    fn test_compliance_mappings() {
        let auth_success = SecurityEventType::AuthSuccess;
        assert!(auth_success.soc2_cc().contains(&"CC6"));
        assert!(auth_success.iso27001().contains(&"A.9.2"));

        let config_changed = SecurityEventType::ConfigChanged;
        assert!(config_changed.soc2_cc().contains(&"CC7"));
        assert!(config_changed.iso27001().contains(&"A.12.1"));
    }

    #[test]
    fn test_security_event_creation() {
        let event = SecurityEvent::new(SecurityEventType::AuthSuccess, None, None);
        assert_eq!(event.event_type, "auth.success");
        assert_eq!(event.severity, SecurityEventSeverity::Low);
        assert_eq!(event.source.system, "mockforge");
    }

    #[test]
    fn test_security_event_builder() {
        let event = SecurityEvent::new(SecurityEventType::AuthFailure, None, None)
            .with_actor(EventActor {
                user_id: Some("user-123".to_string()),
                username: Some("admin".to_string()),
                ip_address: Some("192.168.1.100".to_string()),
                user_agent: Some("Mozilla/5.0".to_string()),
            })
            .with_target(EventTarget {
                resource_type: Some("api".to_string()),
                resource_id: Some("/api/v1/workspaces".to_string()),
                method: Some("GET".to_string()),
            })
            .with_outcome(EventOutcome {
                success: false,
                reason: Some("Invalid credentials".to_string()),
            })
            .with_metadata("attempt_count".to_string(), serde_json::json!(3));

        assert_eq!(event.event_type, "auth.failure");
        assert!(event.actor.is_some());
        assert!(event.target.is_some());
        assert!(event.outcome.is_some());
        assert_eq!(event.metadata.get("attempt_count"), Some(&serde_json::json!(3)));
    }

    #[test]
    fn test_security_event_serialization() {
        let event = SecurityEvent::new(SecurityEventType::AuthSuccess, None, None);
        let json = event.to_json().unwrap();
        assert!(json.contains("auth.success"));
        assert!(json.contains("mockforge"));
    }
}
