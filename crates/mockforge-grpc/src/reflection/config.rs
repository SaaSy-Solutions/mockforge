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
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ResponseTransformConfig {
    /// Enable response transformations
    pub enabled: bool,
    /// Custom headers to add to all responses
    pub custom_headers: std::collections::HashMap<String, String>,
    /// Response body overrides using the override system
    pub overrides: Option<Overrides>,
    /// Enable response validation
    pub validate_responses: bool,
}

impl Default for ResponseTransformConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            custom_headers: std::collections::HashMap::new(),
            overrides: None,
            validate_responses: false,
        }
    }
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
    

    #[test]
    fn test_module_compiles() {
        assert!(true);
    }
}
