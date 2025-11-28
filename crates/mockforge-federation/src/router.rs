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

    #[test]
    fn test_router() {
        let federation = Arc::new(Federation {
            id: Uuid::new_v4(),
            name: "test".to_string(),
            description: String::new(),
            org_id: Uuid::new_v4(),
            services: vec![ServiceBoundary::new(
                "auth".to_string(),
                Uuid::new_v4(),
                "/auth".to_string(),
                ServiceRealityLevel::Real,
            )],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

        let router = FederationRouter::new(federation);
        let result = router.route("/auth/login");

        assert!(result.is_some());
        let routing = result.unwrap();
        assert_eq!(routing.service_path, "/login");
    }
}
