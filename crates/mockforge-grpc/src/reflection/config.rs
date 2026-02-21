//! Configuration for the reflection proxy

use crate::reflection::error_handling::ErrorConfig;
use mockforge_core::{openapi_routes::ValidationMode, overrides::Overrides};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Configuration for the reflection proxy
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProxyConfig {
    /// List of allowed services (if empty, all services are allowed)
    pub allowlist: HashSet<String>,
    /// List of denied services (takes precedence over allowlist)
    pub denylist: HashSet<String>,
    /// Whether to require services to be explicitly allowed
    pub require_explicit_allow: bool,
    /// gRPC port for connection pooling
    pub grpc_port: u16,
    /// Error handling configuration
    pub error_config: Option<ErrorConfig>,
    /// Response transformation configuration
    pub response_transform: ResponseTransformConfig,
    /// Upstream endpoint for request forwarding
    pub upstream_endpoint: Option<String>,
    /// Seed for deterministic mock data generation
    pub mock_seed: Option<u64>,
    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
    /// Admin skip prefixes
    pub admin_skip_prefixes: Vec<String>,
    /// Validation mode overrides
    pub overrides: HashMap<String, ValidationMode>,
    /// Default request mode
    pub request_mode: ValidationMode,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            allowlist: HashSet::new(),
            denylist: HashSet::new(),
            require_explicit_allow: false,
            grpc_port: default_grpc_port(),
            error_config: None,
            response_transform: ResponseTransformConfig::default(),
            upstream_endpoint: None,
            mock_seed: None,
            request_timeout_seconds: default_request_timeout_seconds(),
            admin_skip_prefixes: Vec::new(),
            overrides: HashMap::new(),
            request_mode: ValidationMode::default(),
        }
    }
}

/// Default gRPC port
fn default_grpc_port() -> u16 {
    50051
}

/// Default request timeout in seconds
fn default_request_timeout_seconds() -> u64 {
    30
}

/// Configuration for response transformations
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ResponseTransformConfig {
    /// Enable response transformations
    pub enabled: bool,
    /// Custom headers to add to all responses
    pub custom_headers: HashMap<String, String>,
    /// Response body overrides using the override system
    pub overrides: Option<Overrides>,
    /// Enable response validation
    pub validate_responses: bool,
}

impl ProxyConfig {
    /// Check if a service is allowed
    pub fn is_service_allowed(&self, service_name: &str) -> bool {
        // If service is explicitly denied, it's not allowed
        if self.denylist.contains(service_name) {
            return false;
        }

        // If we require explicit allow and service is not in allowlist, it's not allowed
        if self.require_explicit_allow
            && !self.allowlist.is_empty()
            && !self.allowlist.contains(service_name)
        {
            return false;
        }

        true
    }

    /// Check if a service is denied
    pub fn is_service_denied(&self, service_name: &str) -> bool {
        self.denylist.contains(service_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ProxyConfig Default Tests ====================

    #[test]
    fn test_proxy_config_default() {
        let config = ProxyConfig::default();

        assert!(config.allowlist.is_empty());
        assert!(config.denylist.is_empty());
        assert!(!config.require_explicit_allow);
        assert_eq!(config.grpc_port, 50051);
        assert!(config.error_config.is_none());
        assert!(config.upstream_endpoint.is_none());
        assert!(config.mock_seed.is_none());
        assert_eq!(config.request_timeout_seconds, 30);
        assert!(config.admin_skip_prefixes.is_empty());
        assert!(config.overrides.is_empty());
    }

    #[test]
    fn test_proxy_config_default_grpc_port() {
        assert_eq!(default_grpc_port(), 50051);
    }

    #[test]
    fn test_proxy_config_default_timeout() {
        assert_eq!(default_request_timeout_seconds(), 30);
    }

    // ==================== Service Allowlist Tests ====================

    #[test]
    fn test_is_service_allowed_empty_lists() {
        let config = ProxyConfig::default();

        assert!(config.is_service_allowed("any.service.Name"));
        assert!(config.is_service_allowed("another.Service"));
    }

    #[test]
    fn test_is_service_allowed_in_allowlist() {
        let mut config = ProxyConfig::default();
        config.allowlist.insert("my.allowed.Service".to_string());
        config.require_explicit_allow = true;

        assert!(config.is_service_allowed("my.allowed.Service"));
    }

    #[test]
    fn test_is_service_allowed_not_in_allowlist_explicit() {
        let mut config = ProxyConfig::default();
        config.allowlist.insert("my.allowed.Service".to_string());
        config.require_explicit_allow = true;

        assert!(!config.is_service_allowed("other.Service"));
    }

    #[test]
    fn test_is_service_allowed_not_explicit_mode() {
        let mut config = ProxyConfig::default();
        config.allowlist.insert("my.allowed.Service".to_string());
        config.require_explicit_allow = false;

        // Without explicit allow requirement, all services are allowed
        assert!(config.is_service_allowed("other.Service"));
        assert!(config.is_service_allowed("my.allowed.Service"));
    }

    // ==================== Service Denylist Tests ====================

    #[test]
    fn test_is_service_denied_empty_denylist() {
        let config = ProxyConfig::default();

        assert!(!config.is_service_denied("any.service.Name"));
    }

    #[test]
    fn test_is_service_denied_in_denylist() {
        let mut config = ProxyConfig::default();
        config.denylist.insert("blocked.Service".to_string());

        assert!(config.is_service_denied("blocked.Service"));
        assert!(!config.is_service_denied("other.Service"));
    }

    #[test]
    fn test_denylist_takes_precedence() {
        let mut config = ProxyConfig::default();
        config.allowlist.insert("my.Service".to_string());
        config.denylist.insert("my.Service".to_string());
        config.require_explicit_allow = true;

        // Service is in both lists, denylist takes precedence
        assert!(!config.is_service_allowed("my.Service"));
    }

    #[test]
    fn test_multiple_services_in_denylist() {
        let mut config = ProxyConfig::default();
        config.denylist.insert("blocked1.Service".to_string());
        config.denylist.insert("blocked2.Service".to_string());
        config.denylist.insert("blocked3.Service".to_string());

        assert!(config.is_service_denied("blocked1.Service"));
        assert!(config.is_service_denied("blocked2.Service"));
        assert!(config.is_service_denied("blocked3.Service"));
        assert!(!config.is_service_denied("allowed.Service"));
    }

    // ==================== ResponseTransformConfig Tests ====================

    #[test]
    fn test_response_transform_config_default() {
        let config = ResponseTransformConfig::default();

        assert!(!config.enabled);
        assert!(config.custom_headers.is_empty());
        assert!(config.overrides.is_none());
        assert!(!config.validate_responses);
    }

    #[test]
    fn test_response_transform_config_with_headers() {
        let mut config = ResponseTransformConfig::default();
        config.enabled = true;
        config.custom_headers.insert("X-Custom-Header".to_string(), "value".to_string());

        assert!(config.enabled);
        assert_eq!(config.custom_headers.get("X-Custom-Header"), Some(&"value".to_string()));
    }

    #[test]
    fn test_response_transform_config_with_validation() {
        let mut config = ResponseTransformConfig::default();
        config.validate_responses = true;

        assert!(config.validate_responses);
    }

    // ==================== Serialization Tests ====================

    #[test]
    fn test_proxy_config_serialization() {
        let config = ProxyConfig::default();

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ProxyConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.grpc_port, config.grpc_port);
        assert_eq!(deserialized.require_explicit_allow, config.require_explicit_allow);
        assert_eq!(deserialized.request_timeout_seconds, config.request_timeout_seconds);
    }

    #[test]
    fn test_proxy_config_deserialization() {
        let json = r#"{
            "allowlist": ["service1", "service2"],
            "denylist": ["blocked"],
            "require_explicit_allow": true,
            "grpc_port": 9090,
            "error_config": null,
            "response_transform": {
                "enabled": false,
                "custom_headers": {},
                "overrides": null,
                "validate_responses": false
            },
            "upstream_endpoint": "http://localhost:50051",
            "mock_seed": 12345,
            "request_timeout_seconds": 60,
            "admin_skip_prefixes": ["/admin"],
            "overrides": {},
            "request_mode": "Enforce"
        }"#;

        let config: ProxyConfig = serde_json::from_str(json).unwrap();

        assert_eq!(config.allowlist.len(), 2);
        assert!(config.allowlist.contains("service1"));
        assert!(config.allowlist.contains("service2"));
        assert!(config.denylist.contains("blocked"));
        assert!(config.require_explicit_allow);
        assert_eq!(config.grpc_port, 9090);
        assert_eq!(config.upstream_endpoint, Some("http://localhost:50051".to_string()));
        assert_eq!(config.mock_seed, Some(12345));
        assert_eq!(config.request_timeout_seconds, 60);
    }

    #[test]
    fn test_response_transform_config_serialization() {
        let mut config = ResponseTransformConfig::default();
        config.enabled = true;
        config.custom_headers.insert("X-Test".to_string(), "test".to_string());

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ResponseTransformConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.enabled, config.enabled);
        assert_eq!(deserialized.custom_headers, config.custom_headers);
    }

    // ==================== Clone Tests ====================

    #[test]
    fn test_proxy_config_clone() {
        let mut config = ProxyConfig::default();
        config.allowlist.insert("service1".to_string());
        config.grpc_port = 8080;

        let cloned = config.clone();

        assert_eq!(cloned.allowlist, config.allowlist);
        assert_eq!(cloned.grpc_port, config.grpc_port);
    }

    #[test]
    fn test_response_transform_config_clone() {
        let mut config = ResponseTransformConfig::default();
        config.enabled = true;

        let cloned = config.clone();

        assert_eq!(cloned.enabled, config.enabled);
    }

    // ==================== Edge Cases ====================

    #[test]
    fn test_empty_allowlist_with_explicit_allow() {
        let mut config = ProxyConfig::default();
        config.require_explicit_allow = true;
        // Empty allowlist

        // When allowlist is empty and require_explicit_allow is true,
        // the condition !self.allowlist.is_empty() is false, so all services are allowed
        assert!(config.is_service_allowed("any.Service"));
    }

    #[test]
    fn test_special_characters_in_service_name() {
        let mut config = ProxyConfig::default();
        config.allowlist.insert("com.example.v1.MyService".to_string());
        config.require_explicit_allow = true;

        assert!(config.is_service_allowed("com.example.v1.MyService"));
        assert!(!config.is_service_allowed("com.example.v2.MyService"));
    }

    #[test]
    fn test_case_sensitive_service_names() {
        let mut config = ProxyConfig::default();
        config.allowlist.insert("MyService".to_string());
        config.require_explicit_allow = true;

        assert!(config.is_service_allowed("MyService"));
        assert!(!config.is_service_allowed("myservice"));
        assert!(!config.is_service_allowed("MYSERVICE"));
    }
}
