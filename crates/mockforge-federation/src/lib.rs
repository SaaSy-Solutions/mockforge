//! # `MockForge` Federation
//!
//! Multi-workspace federation for `MockForge`.
//!
//! This crate enables composing multiple mock workspaces into a single federated
//! "virtual system" for large organizations with microservices architectures.
//!
//! ## Overview
//!
//! Federation allows you to:
//!
//! - Define service boundaries and map services to workspaces
//! - Compose multiple workspaces into one federated virtual system
//! - Run system-wide scenarios that span multiple services
//! - Control reality level per service independently
//!
//! ## Example Federation
//!
//! ```yaml
//! federation:
//!   name: "e-commerce-platform"
//!   services:
//!     - name: "auth"
//!       workspace_id: "workspace-auth-123"
//!       base_path: "/auth"
//!       reality_level: "real"  # Use real upstream
//!
//!     - name: "payments"
//!       workspace_id: "workspace-payments-456"
//!       base_path: "/payments"
//!       reality_level: "mock_v3"
//!
//!     - name: "inventory"
//!       workspace_id: "workspace-inventory-789"
//!       base_path: "/inventory"
//!       reality_level: "blended"  # Mix of mock and real
//!
//!     - name: "shipping"
//!       workspace_id: "workspace-shipping-012"
//!       base_path: "/shipping"
//!       reality_level: "chaos_driven"  # Chaos testing mode
//! ```
//!
//! ## Features
//!
//! - **Service Registry**: Define services and their workspace mappings
//! - **Federation Router**: Route requests to appropriate workspace based on service
//! - **Virtual System Manager**: Compose workspaces into unified system
//! - **Per-Service Reality Level**: Control reality level independently per service
//! - **System-Wide Scenarios**: Define scenarios that span multiple services

pub mod database;
pub mod federation;
pub mod router;
pub mod service;

pub use database::FederationDatabase;
pub use federation::{Federation, FederationConfig, FederationService};
pub use router::{FederationRouter, RoutingResult};
pub use service::{ServiceBoundary, ServiceRealityLevel};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use uuid::Uuid;

    // Integration tests to ensure the public API works as expected

    #[test]
    fn test_service_reality_level_public_api() {
        // Test that ServiceRealityLevel enum is accessible and usable
        let level = ServiceRealityLevel::Real;
        assert_eq!(level.as_str(), "real");

        let parsed = ServiceRealityLevel::from_str("mock_v3");
        assert_eq!(parsed, Some(ServiceRealityLevel::MockV3));
    }

    #[test]
    fn test_service_boundary_public_api() {
        // Test that ServiceBoundary can be created and used
        let workspace_id = Uuid::new_v4();
        let service = ServiceBoundary::new(
            "test-service".to_string(),
            workspace_id,
            "/api".to_string(),
            ServiceRealityLevel::Blended,
        );

        assert_eq!(service.name, "test-service");
        assert_eq!(service.workspace_id, workspace_id);
        assert_eq!(service.base_path, "/api");
        assert_eq!(service.reality_level, ServiceRealityLevel::Blended);
        assert!(service.matches_path("/api/users"));
        assert!(!service.matches_path("/other"));
    }

    #[test]
    fn test_federation_config_public_api() {
        // Test that FederationConfig can be created and serialized
        let config = FederationConfig {
            name: "test-fed".to_string(),
            description: "Test federation".to_string(),
            services: vec![FederationService {
                name: "service1".to_string(),
                workspace_id: Uuid::new_v4().to_string(),
                base_path: "/service1".to_string(),
                reality_level: "real".to_string(),
                config: HashMap::new(),
                dependencies: Vec::new(),
            }],
        };

        assert_eq!(config.name, "test-fed");
        assert_eq!(config.services.len(), 1);

        // Test serialization
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test-fed"));
    }

    #[test]
    fn test_federation_public_api() {
        // Test that Federation can be created from config
        let org_id = Uuid::new_v4();
        let config = FederationConfig {
            name: "my-federation".to_string(),
            description: "My test federation".to_string(),
            services: vec![FederationService {
                name: "auth".to_string(),
                workspace_id: Uuid::new_v4().to_string(),
                base_path: "/auth".to_string(),
                reality_level: "real".to_string(),
                config: HashMap::new(),
                dependencies: Vec::new(),
            }],
        };

        let federation = Federation::from_config(org_id, config).unwrap();

        assert_eq!(federation.name, "my-federation");
        assert_eq!(federation.org_id, org_id);
        assert_eq!(federation.services.len(), 1);
        assert_eq!(federation.services[0].name, "auth");
    }

    #[test]
    fn test_federation_router_public_api() {
        // Test that FederationRouter can route requests
        let org_id = Uuid::new_v4();
        let workspace_id = Uuid::new_v4();

        let config = FederationConfig {
            name: "router-test".to_string(),
            description: String::new(),
            services: vec![
                FederationService {
                    name: "auth".to_string(),
                    workspace_id: workspace_id.to_string(),
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
                    dependencies: Vec::new(),
                },
            ],
        };

        let federation = Federation::from_config(org_id, config).unwrap();
        let router = FederationRouter::new(Arc::new(federation));

        // Test routing
        let result = router.route("/auth/login").unwrap();
        assert_eq!(result.service.name, "auth");
        assert_eq!(result.workspace_id, workspace_id);
        assert_eq!(result.service_path, "/login");

        // Test no match
        assert!(router.route("/unknown").is_none());
    }

    #[test]
    fn test_routing_result_public_api() {
        // Test that RoutingResult is accessible and usable
        let workspace_id = Uuid::new_v4();
        let service = Arc::new(ServiceBoundary::new(
            "test".to_string(),
            workspace_id,
            "/test".to_string(),
            ServiceRealityLevel::Real,
        ));

        let result = RoutingResult {
            workspace_id,
            service: service.clone(),
            service_path: "/path".to_string(),
        };

        assert_eq!(result.workspace_id, workspace_id);
        assert_eq!(result.service.name, "test");
        assert_eq!(result.service_path, "/path");

        // Test that it can be cloned
        let cloned = result.clone();
        assert_eq!(cloned.workspace_id, result.workspace_id);
    }

    #[test]
    fn test_federation_service_with_config() {
        // Test FederationService with complex config
        let mut config = HashMap::new();
        config.insert("timeout".to_string(), serde_json::json!(3000));
        config.insert("retries".to_string(), serde_json::json!(5));

        let service = FederationService {
            name: "api".to_string(),
            workspace_id: Uuid::new_v4().to_string(),
            base_path: "/api".to_string(),
            reality_level: "blended".to_string(),
            config: config.clone(),
            dependencies: vec!["auth".to_string(), "db".to_string()],
        };

        assert_eq!(service.config.len(), 2);
        assert_eq!(service.dependencies.len(), 2);

        // Test serialization/deserialization
        let json = serde_json::to_string(&service).unwrap();
        let deserialized: FederationService = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, service.name);
        assert_eq!(deserialized.config.len(), service.config.len());
    }

    #[test]
    fn test_all_reality_levels_exposed() {
        // Ensure all reality levels are accessible through public API
        let _real = ServiceRealityLevel::Real;
        let _mock = ServiceRealityLevel::MockV3;
        let _blended = ServiceRealityLevel::Blended;
        let _chaos = ServiceRealityLevel::ChaosDriven;

        // Test conversion
        assert_eq!(ServiceRealityLevel::Real.as_str(), "real");
        assert_eq!(ServiceRealityLevel::MockV3.as_str(), "mock_v3");
        assert_eq!(ServiceRealityLevel::Blended.as_str(), "blended");
        assert_eq!(ServiceRealityLevel::ChaosDriven.as_str(), "chaos_driven");
    }

    #[test]
    fn test_federation_mutation_methods() {
        // Test that Federation mutation methods work
        let org_id = Uuid::new_v4();
        let config = FederationConfig {
            name: "mutable-fed".to_string(),
            description: String::new(),
            services: vec![FederationService {
                name: "initial".to_string(),
                workspace_id: Uuid::new_v4().to_string(),
                base_path: "/initial".to_string(),
                reality_level: "real".to_string(),
                config: HashMap::new(),
                dependencies: Vec::new(),
            }],
        };

        let mut federation = Federation::from_config(org_id, config).unwrap();
        assert_eq!(federation.services.len(), 1);

        // Add a service
        federation.add_service(ServiceBoundary::new(
            "added".to_string(),
            Uuid::new_v4(),
            "/added".to_string(),
            ServiceRealityLevel::MockV3,
        ));
        assert_eq!(federation.services.len(), 2);

        // Remove a service
        let removed = federation.remove_service("initial");
        assert!(removed);
        assert_eq!(federation.services.len(), 1);
        assert_eq!(federation.services[0].name, "added");
    }

    #[test]
    fn test_service_path_extraction() {
        // Test that service path extraction works through public API
        let service = ServiceBoundary::new(
            "api".to_string(),
            Uuid::new_v4(),
            "/api/v1".to_string(),
            ServiceRealityLevel::Real,
        );

        assert_eq!(service.extract_service_path("/api/v1/users"), Some("/users".to_string()));
        assert_eq!(service.extract_service_path("/api/v1"), Some("/".to_string()));
        assert_eq!(service.extract_service_path("/other"), None);
    }

    #[test]
    fn test_federation_service_lookup() {
        // Test finding services in federation
        let org_id = Uuid::new_v4();
        let config = FederationConfig {
            name: "lookup-test".to_string(),
            description: String::new(),
            services: vec![
                FederationService {
                    name: "service-a".to_string(),
                    workspace_id: Uuid::new_v4().to_string(),
                    base_path: "/a".to_string(),
                    reality_level: "real".to_string(),
                    config: HashMap::new(),
                    dependencies: Vec::new(),
                },
                FederationService {
                    name: "service-b".to_string(),
                    workspace_id: Uuid::new_v4().to_string(),
                    base_path: "/b".to_string(),
                    reality_level: "mock_v3".to_string(),
                    config: HashMap::new(),
                    dependencies: Vec::new(),
                },
            ],
        };

        let federation = Federation::from_config(org_id, config).unwrap();

        // Get by name
        let service_a = federation.get_service("service-a");
        assert!(service_a.is_some());
        assert_eq!(service_a.unwrap().base_path, "/a");

        // Find by path
        let service_b = federation.find_service_by_path("/b/endpoint");
        assert!(service_b.is_some());
        assert_eq!(service_b.unwrap().name, "service-b");
    }

    #[test]
    fn test_complete_workflow() {
        // Integration test of a complete workflow
        let org_id = Uuid::new_v4();

        // 1. Create configuration
        let config = FederationConfig {
            name: "e-commerce".to_string(),
            description: "E-commerce platform federation".to_string(),
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
                    name: "catalog".to_string(),
                    workspace_id: Uuid::new_v4().to_string(),
                    base_path: "/catalog".to_string(),
                    reality_level: "mock_v3".to_string(),
                    config: HashMap::new(),
                    dependencies: vec!["auth".to_string()],
                },
            ],
        };

        // 2. Create federation from config
        let federation = Federation::from_config(org_id, config).unwrap();
        assert_eq!(federation.services.len(), 2);

        // 3. Create router
        let router = FederationRouter::new(Arc::new(federation));

        // 4. Route requests
        let auth_route = router.route("/auth/login").unwrap();
        assert_eq!(auth_route.service.name, "auth");
        assert_eq!(auth_route.service.reality_level, ServiceRealityLevel::Real);

        let catalog_route = router.route("/catalog/products/123").unwrap();
        assert_eq!(catalog_route.service.name, "catalog");
        assert_eq!(catalog_route.service_path, "/products/123");
        assert_eq!(catalog_route.service.reality_level, ServiceRealityLevel::MockV3);

        // 5. Verify service dependencies
        let catalog_service = router.services().iter().find(|s| s.name == "catalog").unwrap();
        assert_eq!(catalog_service.dependencies, vec!["auth".to_string()]);
    }

    #[test]
    fn test_yaml_config_roundtrip() {
        // Test that config can be serialized/deserialized with YAML
        let config = FederationConfig {
            name: "yaml-test".to_string(),
            description: "Testing YAML".to_string(),
            services: vec![FederationService {
                name: "test".to_string(),
                workspace_id: Uuid::new_v4().to_string(),
                base_path: "/test".to_string(),
                reality_level: "real".to_string(),
                config: HashMap::new(),
                dependencies: Vec::new(),
            }],
        };

        let yaml = serde_yaml::to_string(&config).unwrap();
        let deserialized: FederationConfig = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(deserialized.name, config.name);
        assert_eq!(deserialized.description, config.description);
        assert_eq!(deserialized.services.len(), config.services.len());
    }

    #[test]
    fn test_json_config_roundtrip() {
        // Test that config can be serialized/deserialized with JSON
        let mut service_config = HashMap::new();
        service_config.insert("key".to_string(), serde_json::json!("value"));

        let config = FederationConfig {
            name: "json-test".to_string(),
            description: "Testing JSON".to_string(),
            services: vec![FederationService {
                name: "test".to_string(),
                workspace_id: Uuid::new_v4().to_string(),
                base_path: "/test".to_string(),
                reality_level: "blended".to_string(),
                config: service_config,
                dependencies: vec!["dep1".to_string()],
            }],
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: FederationConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.name, config.name);
        assert_eq!(deserialized.services[0].config.len(), 1);
        assert_eq!(deserialized.services[0].dependencies.len(), 1);
    }
}
