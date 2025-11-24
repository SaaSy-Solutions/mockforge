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
    fn test_find_service_by_path() {
        let mut federation = Federation {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            description: String::new(),
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
        };

        assert!(federation.find_service_by_path("/auth").is_some());
        assert!(federation.find_service_by_path("/payments").is_some());
        assert!(federation.find_service_by_path("/unknown").is_none());
    }
}
