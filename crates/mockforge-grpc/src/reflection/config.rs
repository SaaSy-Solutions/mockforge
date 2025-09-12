//! Configuration for the reflection proxy

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Configuration for the reflection proxy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyConfig {
    /// List of allowed services (if empty, all services are allowed)
    #[serde(default)]
    pub allowlist: HashSet<String>,
    /// List of denied services (takes precedence over allowlist)
    #[serde(default)]
    pub denylist: HashSet<String>,
    /// Whether to require services to be explicitly allowed
    #[serde(default)]
    pub require_explicit_allow: bool,
    /// gRPC port for connection pooling
    #[serde(default = "default_grpc_port")]
    pub grpc_port: u16,
}

/// Default gRPC port
fn default_grpc_port() -> u16 {
    50051
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            allowlist: HashSet::new(),
            denylist: HashSet::new(),
            require_explicit_allow: false,
            grpc_port: default_grpc_port(),
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
        if self.require_explicit_allow && !self.allowlist.is_empty() && !self.allowlist.contains(service_name) {
            return false;
        }

        true
    }

    /// Check if a service is denied
    pub fn is_service_denied(&self, service_name: &str) -> bool {
        self.denylist.contains(service_name)
    }
}
