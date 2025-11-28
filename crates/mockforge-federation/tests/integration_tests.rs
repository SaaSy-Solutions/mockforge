//! Integration tests for MockOps Federation
//!
//! Tests that verify federation routing, service boundaries, and
//! multi-workspace composition work correctly end-to-end.

use mockforge_federation::{
    federation::{Federation, FederationConfig, FederationService},
    router::{FederationRouter, RoutingResult},
    service::{ServiceBoundary, ServiceRealityLevel},
};
use std::collections::HashMap;
use uuid::Uuid;

/// Test federation creation from config
#[tokio::test]
async fn test_federation_creation() {
    let config = FederationConfig {
        name: "test-federation".to_string(),
        description: "Test federation".to_string(),
        services: vec![FederationService {
            name: "auth".to_string(),
            workspace_id: Uuid::new_v4().to_string(),
            base_path: "/auth".to_string(),
            reality_level: "real".to_string(),
            config: HashMap::new(),
            dependencies: Vec::new(),
        }],
    };

    let org_id = Uuid::new_v4();
    let federation = Federation::from_config(org_id, config).expect("Failed to create federation");

    assert_eq!(federation.name, "test-federation");
    assert_eq!(federation.org_id, org_id);
    assert_eq!(federation.services.len(), 1);
    assert_eq!(federation.services[0].name, "auth");
}

/// Test service path matching
#[tokio::test]
async fn test_service_path_matching() {
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
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // Test path matching
    let auth_service = federation.find_service_by_path("/auth");
    assert!(auth_service.is_some());
    assert_eq!(auth_service.unwrap().name, "auth");

    let payments_service = federation.find_service_by_path("/payments/process");
    assert!(payments_service.is_some());
    assert_eq!(payments_service.unwrap().name, "payments");

    let unknown_service = federation.find_service_by_path("/unknown");
    assert!(unknown_service.is_none());
}

/// Test service path extraction
#[tokio::test]
async fn test_service_path_extraction() {
    let service = ServiceBoundary::new(
        "auth".to_string(),
        Uuid::new_v4(),
        "/auth".to_string(),
        ServiceRealityLevel::Real,
    );

    // Test path extraction
    assert_eq!(service.extract_service_path("/auth"), Some("/".to_string()));
    assert_eq!(service.extract_service_path("/auth/login"), Some("/login".to_string()));
    assert_eq!(service.extract_service_path("/auth/users/123"), Some("/users/123".to_string()));
    assert_eq!(service.extract_service_path("/payments"), None);
}

/// Test federation router
#[tokio::test]
async fn test_federation_router() {
    let federation = Federation {
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
                "api".to_string(),
                Uuid::new_v4(),
                "/api".to_string(),
                ServiceRealityLevel::MockV3,
            ),
        ],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    let router = FederationRouter::new(std::sync::Arc::new(federation));

    // Test routing
    let auth_result = router.route("/auth/login");
    assert!(auth_result.is_some());
    if let Some(routing) = auth_result {
        assert_eq!(routing.service.name, "auth");
        assert_eq!(routing.service_path, "/login");
    }

    let api_result = router.route("/api/users");
    assert!(api_result.is_some());
    if let Some(routing) = api_result {
        assert_eq!(routing.service.name, "api");
        assert_eq!(routing.service_path, "/users");
    }

    let not_found_result = router.route("/unknown");
    assert!(not_found_result.is_none());
}

/// Test service reality levels
#[tokio::test]
async fn test_service_reality_levels() {
    // Test all reality level variants
    assert_eq!(ServiceRealityLevel::from_str("real"), Some(ServiceRealityLevel::Real));
    assert_eq!(ServiceRealityLevel::from_str("mock_v3"), Some(ServiceRealityLevel::MockV3));
    assert_eq!(ServiceRealityLevel::from_str("blended"), Some(ServiceRealityLevel::Blended));
    assert_eq!(
        ServiceRealityLevel::from_str("chaos_driven"),
        Some(ServiceRealityLevel::ChaosDriven)
    );

    // Test case insensitivity
    assert_eq!(ServiceRealityLevel::from_str("MOCK_V3"), Some(ServiceRealityLevel::MockV3));
    assert_eq!(ServiceRealityLevel::from_str("REAL"), Some(ServiceRealityLevel::Real));

    // Test invalid reality level
    assert_eq!(ServiceRealityLevel::from_str("invalid"), None);
}

/// Test federation with multiple services
#[tokio::test]
async fn test_federation_multiple_services() {
    let config = FederationConfig {
        name: "multi-service-federation".to_string(),
        description: "Federation with multiple services".to_string(),
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
            FederationService {
                name: "inventory".to_string(),
                workspace_id: Uuid::new_v4().to_string(),
                base_path: "/inventory".to_string(),
                reality_level: "blended".to_string(),
                config: HashMap::new(),
                dependencies: vec!["payments".to_string()],
            },
        ],
    };

    let org_id = Uuid::new_v4();
    let federation = Federation::from_config(org_id, config).expect("Failed to create federation");

    assert_eq!(federation.services.len(), 3);
    assert_eq!(federation.services[1].dependencies.len(), 1);
    assert_eq!(federation.services[2].dependencies.len(), 1);

    // Test service lookup
    let auth_service = federation.get_service("auth");
    assert!(auth_service.is_some());
    assert_eq!(auth_service.unwrap().reality_level, ServiceRealityLevel::Real);

    let payments_service = federation.get_service("payments");
    assert!(payments_service.is_some());
    assert_eq!(payments_service.unwrap().reality_level, ServiceRealityLevel::MockV3);
}

/// Test longest path matching (most specific service wins)
#[tokio::test]
async fn test_longest_path_matching() {
    let mut federation = Federation {
        id: Uuid::new_v4(),
        name: "test".to_string(),
        description: String::new(),
        org_id: Uuid::new_v4(),
        services: vec![
            ServiceBoundary::new(
                "api".to_string(),
                Uuid::new_v4(),
                "/api".to_string(),
                ServiceRealityLevel::Real,
            ),
            ServiceBoundary::new(
                "api-v2".to_string(),
                Uuid::new_v4(),
                "/api/v2".to_string(),
                ServiceRealityLevel::MockV3,
            ),
        ],
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    // /api/v2 should match the more specific service
    let service = federation.find_service_by_path("/api/v2/users");
    assert!(service.is_some());
    assert_eq!(service.unwrap().name, "api-v2");

    // /api/v1 should match the general api service
    let service = federation.find_service_by_path("/api/v1/users");
    assert!(service.is_some());
    assert_eq!(service.unwrap().name, "api");
}
