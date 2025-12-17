//! Federation router
//!
//! Routes requests to appropriate workspaces based on service boundaries.

use crate::federation::Federation;
use crate::service::ServiceBoundary;
use chrono::Utc;
use std::sync::Arc;
use tracing::debug;
use uuid::Uuid;

/// Routing result
#[derive(Debug, Clone)]
pub struct RoutingResult {
    /// Target workspace ID
    pub workspace_id: Uuid,
    /// Service boundary
    pub service: Arc<ServiceBoundary>,
    /// Service-specific path (path with `base_path` stripped)
    pub service_path: String,
}

/// Federation router
///
/// Routes incoming requests to the appropriate workspace based on
/// service boundaries defined in the federation.
pub struct FederationRouter {
    /// Active federation
    federation: Arc<Federation>,
}

impl FederationRouter {
    /// Create a new federation router
    #[must_use]
    pub const fn new(federation: Arc<Federation>) -> Self {
        Self { federation }
    }

    /// Route a request to the appropriate workspace
    ///
    /// Returns the workspace ID and service path for the request.
    pub fn route(&self, path: &str) -> Option<RoutingResult> {
        debug!(path = %path, "Routing request in federation");

        let service = self.federation.find_service_by_path(path)?;

        let service_path = service.extract_service_path(path)?;

        debug!(
            path = %path,
            service = %service.name,
            workspace_id = %service.workspace_id,
            service_path = %service_path,
            "Routed request to service"
        );

        Some(RoutingResult {
            workspace_id: service.workspace_id,
            service: Arc::new(service.clone()),
            service_path,
        })
    }

    /// Get all services in the federation
    #[must_use]
    pub fn services(&self) -> &[ServiceBoundary] {
        &self.federation.services
    }

    /// Get federation ID
    #[must_use]
    pub fn federation_id(&self) -> Uuid {
        self.federation.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::ServiceRealityLevel;

    fn create_test_federation() -> Arc<Federation> {
        Arc::new(Federation {
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
        })
    }

    #[test]
    fn test_router() {
        let federation = create_test_federation();
        let router = FederationRouter::new(federation);
        let result = router.route("/auth/login");

        assert!(result.is_some());
        let routing = result.unwrap();
        assert_eq!(routing.service_path, "/login");
    }

    #[test]
    fn test_router_new() {
        let federation = create_test_federation();
        let router = FederationRouter::new(federation.clone());
        assert_eq!(router.federation_id(), federation.id);
    }

    #[test]
    fn test_router_route_exact_path() {
        let federation = create_test_federation();
        let router = FederationRouter::new(federation);

        let result = router.route("/auth").unwrap();
        assert_eq!(result.service_path, "/");
        assert_eq!(result.service.name, "auth");
    }

    #[test]
    fn test_router_route_nested_path() {
        let federation = create_test_federation();
        let router = FederationRouter::new(federation);

        let result = router.route("/payments/process/order/123").unwrap();
        assert_eq!(result.service_path, "/process/order/123");
        assert_eq!(result.service.name, "payments");
    }

    #[test]
    fn test_router_route_no_match() {
        let federation = create_test_federation();
        let router = FederationRouter::new(federation);

        assert!(router.route("/unknown").is_none());
        assert!(router.route("/api/users").is_none());
        assert!(router.route("").is_none());
    }

    #[test]
    fn test_router_services() {
        let federation = create_test_federation();
        let router = FederationRouter::new(federation);

        let services = router.services();
        assert_eq!(services.len(), 2);
        assert!(services.iter().any(|s| s.name == "auth"));
        assert!(services.iter().any(|s| s.name == "payments"));
    }

    #[test]
    fn test_router_federation_id() {
        let federation = create_test_federation();
        let expected_id = federation.id;
        let router = FederationRouter::new(federation);

        assert_eq!(router.federation_id(), expected_id);
    }

    #[test]
    fn test_routing_result_contains_workspace_id() {
        let federation = create_test_federation();
        let expected_workspace_id = federation.services[0].workspace_id;
        let router = FederationRouter::new(federation);

        let result = router.route("/auth/login").unwrap();
        assert_eq!(result.workspace_id, expected_workspace_id);
    }

    #[test]
    fn test_routing_result_debug() {
        let federation = create_test_federation();
        let router = FederationRouter::new(federation);

        let result = router.route("/auth/login").unwrap();
        let debug = format!("{:?}", result);
        assert!(debug.contains("RoutingResult"));
        assert!(debug.contains("service_path"));
    }

    #[test]
    fn test_routing_result_clone() {
        let federation = create_test_federation();
        let router = FederationRouter::new(federation);

        let result = router.route("/auth/login").unwrap();
        let cloned = result.clone();

        assert_eq!(result.workspace_id, cloned.workspace_id);
        assert_eq!(result.service_path, cloned.service_path);
        assert_eq!(result.service.name, cloned.service.name);
    }

    #[test]
    fn test_router_routes_to_different_services() {
        let federation = create_test_federation();
        let router = FederationRouter::new(federation);

        let auth_result = router.route("/auth/login").unwrap();
        let payment_result = router.route("/payments/process").unwrap();

        assert_eq!(auth_result.service.name, "auth");
        assert_eq!(payment_result.service.name, "payments");
        assert_ne!(auth_result.workspace_id, payment_result.workspace_id);
    }

    #[test]
    fn test_router_with_empty_services() {
        let federation = Arc::new(Federation {
            id: Uuid::new_v4(),
            name: "empty".to_string(),
            description: String::new(),
            org_id: Uuid::new_v4(),
            services: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        let router = FederationRouter::new(federation);
        assert!(router.route("/any/path").is_none());
        assert!(router.services().is_empty());
    }
}
