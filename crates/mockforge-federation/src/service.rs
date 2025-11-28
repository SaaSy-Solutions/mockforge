//! Service definitions and boundaries
//!
//! Services represent individual microservices in a federated system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Service reality level
///
/// Controls how a service behaves in the federation:
/// - `Real`: Use real upstream (no mocking)
/// - `MockV3`: Use mock with reality level 3
/// - `Blended`: Mix of mock and real data
/// - `ChaosDriven`: Chaos testing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceRealityLevel {
    /// Use real upstream (no mocking)
    Real,
    /// Use mock with reality level 3
    MockV3,
    /// Mix of mock and real data
    Blended,
    /// Chaos testing mode
    ChaosDriven,
}

impl ServiceRealityLevel {
    /// Convert to string
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Real => "real",
            Self::MockV3 => "mock_v3",
            Self::Blended => "blended",
            Self::ChaosDriven => "chaos_driven",
        }
    }

    /// Parse from string
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "real" => Some(Self::Real),
            "mock_v3" | "mockv3" => Some(Self::MockV3),
            "blended" => Some(Self::Blended),
            "chaos_driven" | "chaosdriven" => Some(Self::ChaosDriven),
            _ => None,
        }
    }
}

/// Service boundary definition
///
/// Defines a service in the federation, including its workspace mapping,
/// base path, and reality level configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceBoundary {
    /// Service name
    pub name: String,
    /// Workspace ID this service maps to
    pub workspace_id: Uuid,
    /// Base path for this service (e.g., "/auth", "/payments")
    pub base_path: String,
    /// Reality level for this service
    pub reality_level: ServiceRealityLevel,
    /// Service-specific configuration
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
    /// Inter-service dependencies
    #[serde(default)]
    pub dependencies: Vec<String>,
}

impl ServiceBoundary {
    /// Create a new service boundary
    #[must_use]
    pub fn new(
        name: String,
        workspace_id: Uuid,
        base_path: String,
        reality_level: ServiceRealityLevel,
    ) -> Self {
        Self {
            name,
            workspace_id,
            base_path,
            reality_level,
            config: HashMap::new(),
            dependencies: Vec::new(),
        }
    }

    /// Check if a path matches this service
    #[must_use]
    pub fn matches_path(&self, path: &str) -> bool {
        path.starts_with(&self.base_path)
    }

    /// Extract service-specific path from full path
    #[must_use]
    pub fn extract_service_path(&self, full_path: &str) -> Option<String> {
        if full_path.starts_with(&self.base_path) {
            let service_path = full_path.strip_prefix(&self.base_path)?;
            Some(if service_path.is_empty() {
                "/".to_string()
            } else if !service_path.starts_with('/') {
                format!("/{service_path}")
            } else {
                service_path.to_string()
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_reality_level() {
        assert_eq!(ServiceRealityLevel::Real.as_str(), "real");
        assert_eq!(ServiceRealityLevel::from_str("real"), Some(ServiceRealityLevel::Real));
        assert_eq!(ServiceRealityLevel::from_str("MOCK_V3"), Some(ServiceRealityLevel::MockV3));
    }

    #[test]
    fn test_service_boundary_path_matching() {
        let service = ServiceBoundary::new(
            "auth".to_string(),
            Uuid::new_v4(),
            "/auth".to_string(),
            ServiceRealityLevel::Real,
        );

        assert!(service.matches_path("/auth"));
        assert!(service.matches_path("/auth/login"));
        assert!(service.matches_path("/auth/users/123"));
        assert!(!service.matches_path("/payments"));
    }

    #[test]
    fn test_extract_service_path() {
        let service = ServiceBoundary::new(
            "auth".to_string(),
            Uuid::new_v4(),
            "/auth".to_string(),
            ServiceRealityLevel::Real,
        );

        assert_eq!(service.extract_service_path("/auth"), Some("/".to_string()));
        assert_eq!(service.extract_service_path("/auth/login"), Some("/login".to_string()));
        assert_eq!(service.extract_service_path("/auth/users/123"), Some("/users/123".to_string()));
        assert_eq!(service.extract_service_path("/payments"), None);
    }
}
