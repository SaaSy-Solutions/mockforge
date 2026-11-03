//! Authentication audit logging
//!
//! This module provides comprehensive audit logging for all authentication events

use super::types::AuthResult;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use std::path::PathBuf;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tracing::{error, info};

/// Authentication audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthAuditEvent {
    /// Event timestamp
    pub timestamp: DateTime<Utc>,
    /// Source IP address
    pub ip_address: String,
    /// User agent string
    pub user_agent: Option<String>,
    /// Authentication method attempted
    pub auth_method: AuthMethod,
    /// Authentication result
    pub result: AuthAuditResult,
    /// Username (if available)
    pub username: Option<String>,
    /// Failure reason (if failed)
    pub failure_reason: Option<String>,
    /// Request path
    pub path: Option<String>,
    /// Request method
    pub http_method: Option<String>,
}

/// Authentication method
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    /// JWT bearer token
    Jwt,
    /// OAuth2 token
    OAuth2,
    /// API key
    ApiKey,
    /// Basic authentication
    Basic,
    /// No authentication provided
    None,
}

impl std::fmt::Display for AuthMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthMethod::Jwt => write!(f, "jwt"),
            AuthMethod::OAuth2 => write!(f, "oauth2"),
            AuthMethod::ApiKey => write!(f, "api_key"),
            AuthMethod::Basic => write!(f, "basic"),
            AuthMethod::None => write!(f, "none"),
        }
    }
}

/// Authentication audit result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthAuditResult {
    /// Authentication succeeded
    Success,
    /// Authentication failed
    Failure,
    /// Token expired
    Expired,
    /// Invalid token/credentials
    Invalid,
    /// Network error during auth
    NetworkError,
    /// Server error during auth
    ServerError,
    /// No authentication provided
    NoAuth,
}

impl From<&AuthResult> for AuthAuditResult {
    fn from(result: &AuthResult) -> Self {
        match result {
            AuthResult::Success(_) => AuthAuditResult::Success,
            AuthResult::Failure(_) => AuthAuditResult::Failure,
            AuthResult::TokenExpired => AuthAuditResult::Expired,
            AuthResult::TokenInvalid(_) => AuthAuditResult::Invalid,
            AuthResult::NetworkError(_) => AuthAuditResult::NetworkError,
            AuthResult::ServerError(_) => AuthAuditResult::ServerError,
            AuthResult::None => AuthAuditResult::NoAuth,
        }
    }
}

/// Audit logger configuration
#[derive(Debug, Clone)]
pub struct AuditLogConfig {
    /// Enable audit logging
    pub enabled: bool,
    /// Log file path
    pub file_path: PathBuf,
    /// Log successful authentications
    pub log_success: bool,
    /// Log failed authentications
    pub log_failures: bool,
    /// Log to structured JSON
    pub json_format: bool,
}

impl Default for AuditLogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            file_path: PathBuf::from("/var/log/mockforge/auth-audit.log"),
            log_success: true,
            log_failures: true,
            json_format: true,
        }
    }
}

/// Audit logger
pub struct AuthAuditLogger {
    config: AuditLogConfig,
}

impl AuthAuditLogger {
    /// Create a new audit logger
    pub fn new(config: AuditLogConfig) -> Self {
        Self { config }
    }

    /// Log an authentication event
    pub async fn log_event(&self, event: AuthAuditEvent) {
        if !self.config.enabled {
            return;
        }

        // Check if we should log this event
        let should_log = match event.result {
            AuthAuditResult::Success => self.config.log_success,
            _ => self.config.log_failures,
        };

        if !should_log {
            return;
        }

        // Log to tracing
        match event.result {
            AuthAuditResult::Success => {
                info!(
                    ip = %event.ip_address,
                    method = %event.auth_method,
                    username = ?event.username,
                    path = ?event.path,
                    "Authentication successful"
                );
            }
            _ => {
                info!(
                    ip = %event.ip_address,
                    method = %event.auth_method,
                    result = ?event.result,
                    reason = ?event.failure_reason,
                    path = ?event.path,
                    "Authentication failed"
                );
            }
        }

        // Write to file
        if let Err(e) = self.write_to_file(&event).await {
            error!("Failed to write audit log: {}", e);
        }
    }

    /// Write event to file
    async fn write_to_file(&self, event: &AuthAuditEvent) -> std::io::Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = self.config.file_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Open file in append mode
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.config.file_path)
            .await?;

        // Format log entry
        let log_entry = if self.config.json_format {
            serde_json::to_string(event).unwrap_or_else(|_| {
                format!(
                    "{{\"timestamp\":\"{}\",\"error\":\"Failed to serialize event\"}}",
                    event.timestamp.to_rfc3339()
                )
            })
        } else {
            format!(
                "[{}] {} {} {} -> {:?} (user: {:?}, reason: {:?})\n",
                event.timestamp.to_rfc3339(),
                event.ip_address,
                event.auth_method,
                event.http_method.as_deref().unwrap_or("?"),
                event.result,
                event.username,
                event.failure_reason
            )
        };

        // Write to file
        file.write_all(log_entry.as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        Ok(())
    }

    /// Create an audit event
    pub fn create_event(
        ip: IpAddr,
        user_agent: Option<String>,
        method: AuthMethod,
        result: &AuthResult,
        path: Option<String>,
        http_method: Option<String>,
    ) -> AuthAuditEvent {
        let (username, failure_reason) = match result {
            AuthResult::Success(claims) => (claims.username.clone(), None),
            AuthResult::Failure(reason) => (None, Some(reason.clone())),
            AuthResult::TokenInvalid(reason) => (None, Some(reason.clone())),
            AuthResult::NetworkError(reason) => (None, Some(reason.clone())),
            AuthResult::ServerError(reason) => (None, Some(reason.clone())),
            AuthResult::TokenExpired => (None, Some("Token expired".to_string())),
            AuthResult::None => (None, None),
        };

        AuthAuditEvent {
            timestamp: Utc::now(),
            ip_address: ip.to_string(),
            user_agent,
            auth_method: method,
            result: AuthAuditResult::from(result),
            username,
            failure_reason,
            path,
            http_method,
        }
    }
}

/// Builder for auth audit events
pub struct AuthAuditEventBuilder {
    event: AuthAuditEvent,
}

impl AuthAuditEventBuilder {
    /// Create a new builder
    pub fn new(ip: IpAddr, method: AuthMethod) -> Self {
        Self {
            event: AuthAuditEvent {
                timestamp: Utc::now(),
                ip_address: ip.to_string(),
                user_agent: None,
                auth_method: method,
                result: AuthAuditResult::NoAuth,
                username: None,
                failure_reason: None,
                path: None,
                http_method: None,
            },
        }
    }

    /// Set user agent
    pub fn user_agent(mut self, ua: String) -> Self {
        self.event.user_agent = Some(ua);
        self
    }

    /// Set result
    pub fn result(mut self, result: &AuthResult) -> Self {
        self.event.result = AuthAuditResult::from(result);

        match result {
            AuthResult::Success(claims) => {
                self.event.username = claims.username.clone();
            }
            AuthResult::Failure(reason) => {
                self.event.failure_reason = Some(reason.clone());
            }
            AuthResult::TokenInvalid(reason) => {
                self.event.failure_reason = Some(reason.clone());
            }
            AuthResult::NetworkError(reason) => {
                self.event.failure_reason = Some(reason.clone());
            }
            AuthResult::ServerError(reason) => {
                self.event.failure_reason = Some(reason.clone());
            }
            AuthResult::TokenExpired => {
                self.event.failure_reason = Some("Token expired".to_string());
            }
            AuthResult::None => {}
        }

        self
    }

    /// Set request path
    pub fn path(mut self, path: String) -> Self {
        self.event.path = Some(path);
        self
    }

    /// Set HTTP method
    pub fn http_method(mut self, method: String) -> Self {
        self.event.http_method = Some(method);
        self
    }

    /// Build the event
    pub fn build(self) -> AuthAuditEvent {
        self.event
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::AuthClaims;
    use tempfile::TempDir;

    #[test]
    fn test_auth_method_display() {
        assert_eq!(AuthMethod::Jwt.to_string(), "jwt");
        assert_eq!(AuthMethod::OAuth2.to_string(), "oauth2");
        assert_eq!(AuthMethod::ApiKey.to_string(), "api_key");
        assert_eq!(AuthMethod::Basic.to_string(), "basic");
        assert_eq!(AuthMethod::None.to_string(), "none");
    }

    #[test]
    fn test_auth_audit_result_from_auth_result() {
        let result = AuthResult::Success(AuthClaims::new());
        assert!(matches!(AuthAuditResult::from(&result), AuthAuditResult::Success));

        let result = AuthResult::Failure("test".to_string());
        assert!(matches!(AuthAuditResult::from(&result), AuthAuditResult::Failure));

        let result = AuthResult::TokenExpired;
        assert!(matches!(AuthAuditResult::from(&result), AuthAuditResult::Expired));
    }

    #[test]
    fn test_event_builder() {
        let ip = "127.0.0.1".parse().unwrap();
        let event = AuthAuditEventBuilder::new(ip, AuthMethod::Jwt)
            .user_agent("Mozilla/5.0".to_string())
            .path("/api/test".to_string())
            .http_method("GET".to_string())
            .result(&AuthResult::Success(AuthClaims::new()))
            .build();

        assert_eq!(event.ip_address, "127.0.0.1");
        assert_eq!(event.user_agent, Some("Mozilla/5.0".to_string()));
        assert_eq!(event.path, Some("/api/test".to_string()));
        assert_eq!(event.http_method, Some("GET".to_string()));
        assert!(matches!(event.result, AuthAuditResult::Success));
    }

    #[tokio::test]
    async fn test_audit_logger_creation() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("audit.log");

        let config = AuditLogConfig {
            enabled: true,
            file_path: log_path.clone(),
            log_success: true,
            log_failures: true,
            json_format: true,
        };

        let logger = AuthAuditLogger::new(config);
        let ip = "192.168.1.1".parse().unwrap();
        let event = AuthAuditEventBuilder::new(ip, AuthMethod::ApiKey)
            .result(&AuthResult::Success(AuthClaims::new()))
            .build();

        logger.log_event(event).await;

        // Check that file was created
        assert!(log_path.exists());
    }

    #[tokio::test]
    async fn test_audit_logger_json_format() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("audit-json.log");

        let config = AuditLogConfig {
            enabled: true,
            file_path: log_path.clone(),
            log_success: true,
            log_failures: true,
            json_format: true,
        };

        let logger = AuthAuditLogger::new(config);
        let ip = "10.0.0.1".parse().unwrap();
        let mut claims = AuthClaims::new();
        claims.username = Some("testuser".to_string());

        let event = AuthAuditEventBuilder::new(ip, AuthMethod::Basic)
            .result(&AuthResult::Success(claims))
            .build();

        logger.log_event(event).await;

        // Read the file and verify JSON format
        let content = tokio::fs::read_to_string(&log_path).await.unwrap();
        assert!(content.contains("\"ip_address\":\"10.0.0.1\""));
        assert!(content.contains("\"auth_method\":\"basic\""));
        assert!(content.contains("\"username\":\"testuser\""));
    }

    #[tokio::test]
    async fn test_audit_logger_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let log_path = temp_dir.path().join("audit-disabled.log");

        let config = AuditLogConfig {
            enabled: false, // Disabled
            file_path: log_path.clone(),
            log_success: true,
            log_failures: true,
            json_format: true,
        };

        let logger = AuthAuditLogger::new(config);
        let ip = "172.16.0.1".parse().unwrap();
        let event = AuthAuditEventBuilder::new(ip, AuthMethod::Jwt)
            .result(&AuthResult::Success(AuthClaims::new()))
            .build();

        logger.log_event(event).await;

        // File should not be created when disabled
        assert!(!log_path.exists());
    }
}
