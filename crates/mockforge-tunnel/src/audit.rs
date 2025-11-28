//! Audit logging for tunnel operations
//!
//! This module provides structured audit logging for all tunnel operations,
//! including creation, deletion, access, and errors.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use tracing::{error, info, warn};

/// Audit event type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuditEventType {
    /// Tunnel created
    TunnelCreated,
    /// Tunnel deleted
    TunnelDeleted,
    /// Tunnel accessed
    TunnelAccessed,
    /// Tunnel status checked
    TunnelStatusChecked,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Authentication failed
    AuthenticationFailed,
    /// Authorization failed
    AuthorizationFailed,
    /// Error occurred
    Error,
    /// Configuration changed
    ConfigChanged,
}

/// Audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Event timestamp
    pub timestamp: chrono::DateTime<Utc>,
    /// Event type
    pub event_type: AuditEventType,
    /// Tunnel ID (if applicable)
    pub tunnel_id: Option<String>,
    /// Client IP address
    pub client_ip: Option<IpAddr>,
    /// User/principal (if authenticated)
    pub principal: Option<String>,
    /// Action performed
    pub action: String,
    /// Resource affected
    pub resource: Option<String>,
    /// Success status
    pub success: bool,
    /// Error message (if failed)
    pub error_message: Option<String>,
    /// Additional metadata
    pub metadata: Option<serde_json::Value>,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(event_type: AuditEventType, action: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            event_type,
            tunnel_id: None,
            client_ip: None,
            principal: None,
            action: action.into(),
            resource: None,
            success: true,
            error_message: None,
            metadata: None,
        }
    }

    /// Set tunnel ID
    pub fn with_tunnel_id(mut self, tunnel_id: impl Into<String>) -> Self {
        self.tunnel_id = Some(tunnel_id.into());
        self
    }

    /// Set client IP
    pub fn with_client_ip(mut self, ip: IpAddr) -> Self {
        self.client_ip = Some(ip);
        self
    }

    /// Set principal
    pub fn with_principal(mut self, principal: impl Into<String>) -> Self {
        self.principal = Some(principal.into());
        self
    }

    /// Set resource
    pub fn with_resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    /// Mark as failed
    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.success = false;
        self.error_message = Some(error.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Log the audit event
    pub fn log(&self) {
        let json = serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string());

        if self.success {
            match self.event_type {
                AuditEventType::RateLimitExceeded
                | AuditEventType::AuthenticationFailed
                | AuditEventType::AuthorizationFailed
                | AuditEventType::Error => {
                    warn!("AUDIT: {}", json);
                }
                _ => {
                    info!("AUDIT: {}", json);
                }
            }
        } else {
            error!("AUDIT: {}", json);
        }
    }
}

/// Audit logger for tunnel operations
pub struct AuditLogger;

impl AuditLogger {
    /// Log tunnel creation
    pub fn log_tunnel_created(
        tunnel_id: &str,
        client_ip: Option<IpAddr>,
        local_url: &str,
        public_url: &str,
    ) {
        AuditEvent::new(AuditEventType::TunnelCreated, "create_tunnel")
            .with_tunnel_id(tunnel_id)
            .with_client_ip(client_ip.unwrap_or_else(|| "0.0.0.0".parse().unwrap()))
            .with_resource(format!("tunnel:{}", tunnel_id))
            .with_metadata(serde_json::json!({
                "local_url": local_url,
                "public_url": public_url,
            }))
            .log();
    }

    /// Log tunnel deletion
    pub fn log_tunnel_deleted(tunnel_id: &str, client_ip: Option<IpAddr>, principal: Option<&str>) {
        let mut event = AuditEvent::new(AuditEventType::TunnelDeleted, "delete_tunnel")
            .with_tunnel_id(tunnel_id)
            .with_resource(format!("tunnel:{}", tunnel_id));

        if let Some(ip) = client_ip {
            event = event.with_client_ip(ip);
        }

        if let Some(p) = principal {
            event = event.with_principal(p);
        }

        event.log();
    }

    /// Log tunnel access
    pub fn log_tunnel_accessed(
        tunnel_id: &str,
        client_ip: IpAddr,
        method: &str,
        path: &str,
        status_code: u16,
    ) {
        AuditEvent::new(AuditEventType::TunnelAccessed, "access_tunnel")
            .with_tunnel_id(tunnel_id)
            .with_client_ip(client_ip)
            .with_resource(format!("tunnel:{}", tunnel_id))
            .with_metadata(serde_json::json!({
                "method": method,
                "path": path,
                "status_code": status_code,
            }))
            .log();
    }

    /// Log rate limit exceeded
    pub fn log_rate_limit_exceeded(client_ip: IpAddr, endpoint: &str) {
        AuditEvent::new(AuditEventType::RateLimitExceeded, "rate_limit_exceeded")
            .with_client_ip(client_ip)
            .with_resource(endpoint)
            .with_error("Rate limit exceeded")
            .log();
    }

    /// Log authentication failure
    pub fn log_auth_failed(client_ip: Option<IpAddr>, reason: &str) {
        let mut event = AuditEvent::new(AuditEventType::AuthenticationFailed, "authentication")
            .with_error(reason);

        if let Some(ip) = client_ip {
            event = event.with_client_ip(ip);
        }

        event.log();
    }

    /// Log error
    pub fn log_error(
        event_type: AuditEventType,
        action: &str,
        error: &str,
        client_ip: Option<IpAddr>,
    ) {
        let mut event = AuditEvent::new(event_type, action).with_error(error);

        if let Some(ip) = client_ip {
            event = event.with_client_ip(ip);
        }

        event.log();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_event_creation() {
        let event = AuditEvent::new(AuditEventType::TunnelCreated, "create_tunnel")
            .with_tunnel_id("test-tunnel")
            .with_client_ip("127.0.0.1".parse().unwrap())
            .with_resource("tunnel:test-tunnel");

        assert_eq!(event.event_type, AuditEventType::TunnelCreated);
        assert_eq!(event.action, "create_tunnel");
        assert!(event.success);
    }

    #[test]
    fn test_audit_event_with_error() {
        let event = AuditEvent::new(AuditEventType::Error, "operation").with_error("Test error");

        assert!(!event.success);
        assert!(event.error_message.is_some());
    }
}
