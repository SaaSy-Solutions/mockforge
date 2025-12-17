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

    // ServiceRealityLevel tests
    #[test]
    fn test_service_reality_level() {
        assert_eq!(ServiceRealityLevel::Real.as_str(), "real");
        assert_eq!(ServiceRealityLevel::from_str("real"), Some(ServiceRealityLevel::Real));
        assert_eq!(ServiceRealityLevel::from_str("MOCK_V3"), Some(ServiceRealityLevel::MockV3));
    }

    #[test]
    fn test_service_reality_level_as_str() {
        assert_eq!(ServiceRealityLevel::Real.as_str(), "real");
        assert_eq!(ServiceRealityLevel::MockV3.as_str(), "mock_v3");
        assert_eq!(ServiceRealityLevel::Blended.as_str(), "blended");
        assert_eq!(ServiceRealityLevel::ChaosDriven.as_str(), "chaos_driven");
    }

    #[test]
    fn test_service_reality_level_from_str_all_variants() {
        assert_eq!(ServiceRealityLevel::from_str("real"), Some(ServiceRealityLevel::Real));
        assert_eq!(ServiceRealityLevel::from_str("mock_v3"), Some(ServiceRealityLevel::MockV3));
        assert_eq!(ServiceRealityLevel::from_str("mockv3"), Some(ServiceRealityLevel::MockV3));
        assert_eq!(ServiceRealityLevel::from_str("blended"), Some(ServiceRealityLevel::Blended));
        assert_eq!(
            ServiceRealityLevel::from_str("chaos_driven"),
            Some(ServiceRealityLevel::ChaosDriven)
        );
        assert_eq!(
            ServiceRealityLevel::from_str("chaosdriven"),
            Some(ServiceRealityLevel::ChaosDriven)
        );
    }

    #[test]
    fn test_service_reality_level_from_str_case_insensitive() {
        assert_eq!(ServiceRealityLevel::from_str("REAL"), Some(ServiceRealityLevel::Real));
        assert_eq!(ServiceRealityLevel::from_str("Real"), Some(ServiceRealityLevel::Real));
        assert_eq!(ServiceRealityLevel::from_str("BLENDED"), Some(ServiceRealityLevel::Blended));
    }

    #[test]
    fn test_service_reality_level_from_str_invalid() {
        assert_eq!(ServiceRealityLevel::from_str("invalid"), None);
        assert_eq!(ServiceRealityLevel::from_str(""), None);
        assert_eq!(ServiceRealityLevel::from_str("unknown"), None);
    }

    #[test]
    fn test_service_reality_level_clone() {
        let level = ServiceRealityLevel::MockV3;
        let cloned = level;
        assert_eq!(level, cloned);
    }

    #[test]
    fn test_service_reality_level_debug() {
        let level = ServiceRealityLevel::ChaosDriven;
        let debug = format!("{:?}", level);
        assert!(debug.contains("ChaosDriven"));
    }

    #[test]
    fn test_service_reality_level_serialize() {
        let level = ServiceRealityLevel::MockV3;
        let json = serde_json::to_string(&level).unwrap();
        assert!(json.contains("mock_v3"));
    }

    #[test]
    fn test_service_reality_level_deserialize() {
        let json = "\"blended\"";
        let level: ServiceRealityLevel = serde_json::from_str(json).unwrap();
        assert_eq!(level, ServiceRealityLevel::Blended);
    }

    // ServiceBoundary tests
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

    #[test]
    fn test_service_boundary_new() {
        let workspace_id = Uuid::new_v4();
        let service = ServiceBoundary::new(
            "payments".to_string(),
            workspace_id,
            "/payments".to_string(),
            ServiceRealityLevel::MockV3,
        );

        assert_eq!(service.name, "payments");
        assert_eq!(service.workspace_id, workspace_id);
        assert_eq!(service.base_path, "/payments");
        assert_eq!(service.reality_level, ServiceRealityLevel::MockV3);
        assert!(service.config.is_empty());
        assert!(service.dependencies.is_empty());
    }

    #[test]
    fn test_service_boundary_with_config() {
        let mut config = HashMap::new();
        config.insert("timeout".to_string(), serde_json::json!(5000));

        let service = ServiceBoundary {
            name: "api".to_string(),
            workspace_id: Uuid::new_v4(),
            base_path: "/api".to_string(),
            reality_level: ServiceRealityLevel::Blended,
            config,
            dependencies: vec!["auth".to_string()],
        };

        assert_eq!(service.config.len(), 1);
        assert_eq!(service.config["timeout"], serde_json::json!(5000));
        assert_eq!(service.dependencies, vec!["auth".to_string()]);
    }

    #[test]
    fn test_service_boundary_clone() {
        let service = ServiceBoundary::new(
            "test".to_string(),
            Uuid::new_v4(),
            "/test".to_string(),
            ServiceRealityLevel::Real,
        );

        let cloned = service.clone();
        assert_eq!(service.name, cloned.name);
        assert_eq!(service.workspace_id, cloned.workspace_id);
        assert_eq!(service.base_path, cloned.base_path);
    }

    #[test]
    fn test_service_boundary_debug() {
        let service = ServiceBoundary::new(
            "test".to_string(),
            Uuid::new_v4(),
            "/test".to_string(),
            ServiceRealityLevel::Real,
        );

        let debug = format!("{:?}", service);
        assert!(debug.contains("test"));
        assert!(debug.contains("ServiceBoundary"));
    }

    #[test]
    fn test_service_boundary_serialize() {
        let service = ServiceBoundary::new(
            "api".to_string(),
            Uuid::parse_str("550e8400-e29b-41d4-a716-446655440000").unwrap(),
            "/api".to_string(),
            ServiceRealityLevel::MockV3,
        );

        let json = serde_json::to_string(&service).unwrap();
        assert!(json.contains("\"name\":\"api\""));
        assert!(json.contains("/api"));
        assert!(json.contains("mock_v3"));
    }

    #[test]
    fn test_extract_service_path_without_leading_slash() {
        let service = ServiceBoundary::new(
            "api".to_string(),
            Uuid::new_v4(),
            "/api".to_string(),
            ServiceRealityLevel::Real,
        );

        // Path that doesn't start with / after stripping base_path
        assert_eq!(service.extract_service_path("/api"), Some("/".to_string()));
    }

    #[test]
    fn test_matches_path_empty_base() {
        let service = ServiceBoundary::new(
            "root".to_string(),
            Uuid::new_v4(),
            "".to_string(),
            ServiceRealityLevel::Real,
        );

        assert!(service.matches_path(""));
        assert!(service.matches_path("/anything"));
    }

    #[test]
    fn test_matches_path_with_nested_paths() {
        let service = ServiceBoundary::new(
            "nested".to_string(),
            Uuid::new_v4(),
            "/api/v1".to_string(),
            ServiceRealityLevel::Real,
        );

        assert!(service.matches_path("/api/v1"));
        assert!(service.matches_path("/api/v1/users"));
        assert!(!service.matches_path("/api/v2"));
    }
}
