//! Federation definition and management
//!
//! A federation is a collection of services that form a virtual system.

use crate::service::{ServiceBoundary, ServiceRealityLevel};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Federation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    /// Federation name
    pub name: String,
    /// Federation description
    #[serde(default)]
    pub description: String,
    /// Services in this federation
    pub services: Vec<FederationService>,
}

/// Service definition in federation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationService {
    /// Service name
    pub name: String,
    /// Workspace ID
    pub workspace_id: String, // UUID as string for YAML
    /// Base path
    pub base_path: String,
    /// Reality level
    pub reality_level: String,
    /// Service-specific config
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
    /// Dependencies
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// Federation metadata (stored in database)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Federation {
    /// Federation ID
    pub id: Uuid,
    /// Federation name
    pub name: String,
    /// Federation description
    pub description: String,
    /// Organization ID
    pub org_id: Uuid,
    /// Service boundaries
    pub services: Vec<ServiceBoundary>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl Federation {
    /// Create a new federation from config
    pub fn from_config(org_id: Uuid, config: FederationConfig) -> Result<Self, String> {
        let mut services = Vec::new();

        for service_config in config.services {
            let workspace_id = Uuid::parse_str(&service_config.workspace_id)
                .map_err(|_| format!("Invalid workspace_id: {}", service_config.workspace_id))?;

            let reality_level = ServiceRealityLevel::from_str(&service_config.reality_level)
                .ok_or_else(|| {
                    format!("Invalid reality_level: {}", service_config.reality_level)
                })?;

            let mut service = ServiceBoundary::new(
                service_config.name.clone(),
                workspace_id,
                service_config.base_path.clone(),
                reality_level,
            );

            service.config = service_config.config;
            service.dependencies = service_config.dependencies;

            services.push(service);
        }

        let now = Utc::now();

        Ok(Self {
            id: Uuid::new_v4(),
            name: config.name,
            description: config.description,
            org_id,
            services,
            created_at: now,
            updated_at: now,
        })
    }

    /// Find service by path
    #[must_use]
    pub fn find_service_by_path(&self, path: &str) -> Option<&ServiceBoundary> {
        // Find the longest matching base_path (most specific match)
        self.services
            .iter()
            .filter(|s| s.matches_path(path))
            .max_by_key(|s| s.base_path.len())
    }

    /// Get service by name
    #[must_use]
    pub fn get_service(&self, name: &str) -> Option<&ServiceBoundary> {
        self.services.iter().find(|s| s.name == name)
    }

    /// Add a service to the federation
    pub fn add_service(&mut self, service: ServiceBoundary) {
        self.services.push(service);
        self.updated_at = Utc::now();
    }

    /// Remove a service from the federation
    pub fn remove_service(&mut self, name: &str) -> bool {
        let len_before = self.services.len();
        self.services.retain(|s| s.name != name);
        let removed = self.services.len() < len_before;
        if removed {
            self.updated_at = Utc::now();
        }
        removed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_federation() -> Federation {
        Federation {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            description: "Test federation".to_string(),
            org_id: Uuid::new_v4(),
            services: vec![
                ServiceBoundary::new(
                    "auth".to_string(),
                    Uuid::new_v4(),
                    "/auth".to_string(),
                    ServiceRealityLevel::Real,
                ),
                ServiceBoundary::new(
                    "payments".to_string(),
                    Uuid::new_v4(),
                    "/payments".to_string(),
                    ServiceRealityLevel::MockV3,
                ),
            ],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_federation_from_config() {
        let config = FederationConfig {
            name: "test-federation".to_string(),
            description: "Test".to_string(),
            services: vec![FederationService {
                name: "auth".to_string(),
                workspace_id: Uuid::new_v4().to_string(),
                base_path: "/auth".to_string(),
                reality_level: "real".to_string(),
                config: HashMap::new(),
                dependencies: Vec::new(),
            }],
        };

        let federation = Federation::from_config(Uuid::new_v4(), config).unwrap();
        assert_eq!(federation.services.len(), 1);
        assert_eq!(federation.services[0].name, "auth");
    }

    #[test]
    fn test_federation_from_config_multiple_services() {
        let config = FederationConfig {
            name: "multi-service".to_string(),
            description: "Multiple services".to_string(),
            services: vec![
                FederationService {
                    name: "auth".to_string(),
                    workspace_id: Uuid::new_v4().to_string(),
                    base_path: "/auth".to_string(),
                    reality_level: "real".to_string(),
                    config: HashMap::new(),
                    dependencies: Vec::new(),
                },
                FederationService {
                    name: "payments".to_string(),
                    workspace_id: Uuid::new_v4().to_string(),
                    base_path: "/payments".to_string(),
                    reality_level: "mock_v3".to_string(),
                    config: HashMap::new(),
                    dependencies: vec!["auth".to_string()],
                },
            ],
        };

        let federation = Federation::from_config(Uuid::new_v4(), config).unwrap();
        assert_eq!(federation.services.len(), 2);
        assert_eq!(federation.services[1].dependencies, vec!["auth".to_string()]);
    }

    #[test]
    fn test_federation_from_config_invalid_workspace_id() {
        let config = FederationConfig {
            name: "test".to_string(),
            description: String::new(),
            services: vec![FederationService {
                name: "auth".to_string(),
                workspace_id: "invalid-uuid".to_string(),
                base_path: "/auth".to_string(),
                reality_level: "real".to_string(),
                config: HashMap::new(),
                dependencies: Vec::new(),
            }],
        };

        let result = Federation::from_config(Uuid::new_v4(), config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid workspace_id"));
    }

    #[test]
    fn test_federation_from_config_invalid_reality_level() {
        let config = FederationConfig {
            name: "test".to_string(),
            description: String::new(),
            services: vec![FederationService {
                name: "auth".to_string(),
                workspace_id: Uuid::new_v4().to_string(),
                base_path: "/auth".to_string(),
                reality_level: "invalid_level".to_string(),
                config: HashMap::new(),
                dependencies: Vec::new(),
            }],
        };

        let result = Federation::from_config(Uuid::new_v4(), config);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid reality_level"));
    }

    #[test]
    fn test_find_service_by_path() {
        let federation = create_test_federation();

        assert!(federation.find_service_by_path("/auth").is_some());
        assert!(federation.find_service_by_path("/payments").is_some());
        assert!(federation.find_service_by_path("/unknown").is_none());
    }

    #[test]
    fn test_find_service_by_path_nested() {
        let federation = create_test_federation();

        let service = federation.find_service_by_path("/auth/login").unwrap();
        assert_eq!(service.name, "auth");

        let service = federation.find_service_by_path("/payments/process").unwrap();
        assert_eq!(service.name, "payments");
    }

    #[test]
    fn test_find_service_by_path_longest_match() {
        let mut federation = create_test_federation();
        federation.services.push(ServiceBoundary::new(
            "auth-admin".to_string(),
            Uuid::new_v4(),
            "/auth/admin".to_string(),
            ServiceRealityLevel::MockV3,
        ));

        // Should match the longer /auth/admin path
        let service = federation.find_service_by_path("/auth/admin/users").unwrap();
        assert_eq!(service.name, "auth-admin");

        // Should match /auth (shorter path)
        let service = federation.find_service_by_path("/auth/login").unwrap();
        assert_eq!(service.name, "auth");
    }

    #[test]
    fn test_get_service() {
        let federation = create_test_federation();

        let service = federation.get_service("auth").unwrap();
        assert_eq!(service.name, "auth");

        let service = federation.get_service("payments").unwrap();
        assert_eq!(service.name, "payments");

        assert!(federation.get_service("nonexistent").is_none());
    }

    #[test]
    fn test_add_service() {
        let mut federation = create_test_federation();
        let initial_count = federation.services.len();

        federation.add_service(ServiceBoundary::new(
            "inventory".to_string(),
            Uuid::new_v4(),
            "/inventory".to_string(),
            ServiceRealityLevel::Blended,
        ));

        assert_eq!(federation.services.len(), initial_count + 1);
        assert!(federation.get_service("inventory").is_some());
    }

    #[test]
    fn test_add_service_updates_timestamp() {
        let mut federation = create_test_federation();
        let original_updated = federation.updated_at;

        // Small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(10));

        federation.add_service(ServiceBoundary::new(
            "new".to_string(),
            Uuid::new_v4(),
            "/new".to_string(),
            ServiceRealityLevel::Real,
        ));

        assert!(federation.updated_at > original_updated);
    }

    #[test]
    fn test_remove_service() {
        let mut federation = create_test_federation();
        let initial_count = federation.services.len();

        assert!(federation.remove_service("auth"));
        assert_eq!(federation.services.len(), initial_count - 1);
        assert!(federation.get_service("auth").is_none());
    }

    #[test]
    fn test_remove_service_not_found() {
        let mut federation = create_test_federation();
        let initial_count = federation.services.len();

        assert!(!federation.remove_service("nonexistent"));
        assert_eq!(federation.services.len(), initial_count);
    }

    #[test]
    fn test_remove_service_updates_timestamp() {
        let mut federation = create_test_federation();
        let original_updated = federation.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));

        federation.remove_service("auth");
        assert!(federation.updated_at > original_updated);
    }

    // FederationConfig tests
    #[test]
    fn test_federation_config_serialize() {
        let config = FederationConfig {
            name: "test".to_string(),
            description: "Test federation".to_string(),
            services: vec![],
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"name\":\"test\""));
        assert!(json.contains("\"description\":\"Test federation\""));
    }

    #[test]
    fn test_federation_config_deserialize() {
        let json = r#"{"name":"test","description":"","services":[]}"#;
        let config: FederationConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "test");
        assert!(config.services.is_empty());
    }

    // FederationService tests
    #[test]
    fn test_federation_service_debug() {
        let service = FederationService {
            name: "auth".to_string(),
            workspace_id: Uuid::new_v4().to_string(),
            base_path: "/auth".to_string(),
            reality_level: "real".to_string(),
            config: HashMap::new(),
            dependencies: Vec::new(),
        };

        let debug = format!("{:?}", service);
        assert!(debug.contains("auth"));
    }

    // Federation serialization tests
    #[test]
    fn test_federation_serialize() {
        let federation = create_test_federation();
        let json = serde_json::to_string(&federation).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("auth"));
        assert!(json.contains("payments"));
    }

    #[test]
    fn test_federation_clone() {
        let federation = create_test_federation();
        let cloned = federation.clone();

        assert_eq!(federation.id, cloned.id);
        assert_eq!(federation.name, cloned.name);
        assert_eq!(federation.services.len(), cloned.services.len());
    }
}
