//! Multi-tenant workspace routing middleware
//!
//! This module provides middleware for routing requests to the appropriate workspace
//! based on path-based or port-based routing strategies.

use super::{MultiTenantWorkspaceRegistry, TenantWorkspace};
use crate::{Error, Result};
use std::sync::Arc;

/// Workspace routing context extracted from request
#[derive(Debug, Clone)]
pub struct WorkspaceContext {
    /// Workspace ID
    pub workspace_id: String,
    /// Original request path
    pub original_path: String,
    /// Path with workspace prefix stripped
    pub stripped_path: String,
    /// Tenant workspace
    pub workspace: TenantWorkspace,
}

/// Workspace router for handling multi-tenant routing
#[derive(Debug, Clone)]
pub struct WorkspaceRouter {
    /// Multi-tenant registry
    registry: Arc<MultiTenantWorkspaceRegistry>,
}

impl WorkspaceRouter {
    /// Create a new workspace router
    pub fn new(registry: Arc<MultiTenantWorkspaceRegistry>) -> Self {
        Self { registry }
    }

    /// Extract workspace context from request path
    pub fn extract_workspace_context(&self, path: &str) -> Result<WorkspaceContext> {
        let config = self.registry.config();

        // If multi-tenant is disabled, use default workspace
        if !config.enabled {
            let workspace = self.registry.get_default_workspace()?;
            return Ok(WorkspaceContext {
                workspace_id: config.default_workspace.clone(),
                original_path: path.to_string(),
                stripped_path: path.to_string(),
                workspace,
            });
        }

        // Try to extract workspace ID from path
        if let Some(workspace_id) = self.registry.extract_workspace_id_from_path(path) {
            // Verify workspace exists and is enabled
            let workspace = self.registry.get_workspace(&workspace_id)?;

            if !workspace.enabled {
                return Err(Error::generic(format!("Workspace '{}' is disabled", workspace_id)));
            }

            let stripped_path = self.registry.strip_workspace_prefix(path, &workspace_id);

            Ok(WorkspaceContext {
                workspace_id: workspace_id.clone(),
                original_path: path.to_string(),
                stripped_path,
                workspace,
            })
        } else {
            // No workspace ID in path, use default workspace
            let workspace = self.registry.get_default_workspace()?;

            Ok(WorkspaceContext {
                workspace_id: config.default_workspace.clone(),
                original_path: path.to_string(),
                stripped_path: path.to_string(),
                workspace,
            })
        }
    }

    /// Get the multi-tenant registry
    pub fn registry(&self) -> &Arc<MultiTenantWorkspaceRegistry> {
        &self.registry
    }

    /// Get workspace by ID
    pub fn get_workspace(&self, workspace_id: &str) -> Result<TenantWorkspace> {
        self.registry.get_workspace(workspace_id)
    }

    /// Check if multi-tenant mode is enabled
    pub fn is_multi_tenant_enabled(&self) -> bool {
        self.registry.config().enabled
    }

    /// Get workspace prefix
    pub fn workspace_prefix(&self) -> &str {
        &self.registry.config().workspace_prefix
    }
}

/// Workspace middleware layer for Axum
pub mod axum_middleware {
    use super::*;
    use ::axum::http::StatusCode;
    use ::axum::{
        extract::Request,
        middleware::Next,
        response::{IntoResponse, Response},
    };

    /// Axum middleware for workspace routing
    pub async fn workspace_middleware(
        router: Arc<WorkspaceRouter>,
        mut request: Request,
        next: Next,
    ) -> Response {
        let path = request.uri().path();

        // Extract workspace context
        let context = match router.extract_workspace_context(path) {
            Ok(ctx) => ctx,
            Err(e) => {
                return (StatusCode::NOT_FOUND, format!("Workspace error: {}", e)).into_response();
            }
        };

        // Store workspace context in request extensions
        request.extensions_mut().insert(context.clone());

        // Update request URI with stripped path
        if context.original_path != context.stripped_path {
            let mut parts = request.uri().clone().into_parts();
            parts.path_and_query = context.stripped_path.parse().ok().or(parts.path_and_query);

            if let Ok(uri) = ::axum::http::Uri::from_parts(parts) {
                *request.uri_mut() = uri;
            }
        }

        // Continue with the request
        next.run(request).await
    }

    /// Extension trait for extracting workspace context from Axum requests
    pub trait WorkspaceContextExt {
        /// Get the workspace context from the request
        fn workspace_context(&self) -> Option<&WorkspaceContext>;

        /// Get the workspace ID
        fn workspace_id(&self) -> Option<&str>;

        /// Get the stripped path
        fn stripped_path(&self) -> Option<&str>;
    }

    impl WorkspaceContextExt for Request {
        fn workspace_context(&self) -> Option<&WorkspaceContext> {
            self.extensions().get::<WorkspaceContext>()
        }

        fn workspace_id(&self) -> Option<&str> {
            self.workspace_context().map(|ctx| ctx.workspace_id.as_str())
        }

        fn stripped_path(&self) -> Option<&str> {
            self.workspace_context().map(|ctx| ctx.stripped_path.as_str())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multi_tenant::{MultiTenantConfig, MultiTenantWorkspaceRegistry};
    use crate::workspace::Workspace;

    fn create_test_router() -> WorkspaceRouter {
        let mut config = MultiTenantConfig::default();
        config.enabled = true;

        let mut registry = MultiTenantWorkspaceRegistry::new(config);

        // Create default workspace
        let default_ws = Workspace::new("Default".to_string());
        registry.register_workspace("default".to_string(), default_ws).unwrap();

        // Create test workspace
        let test_ws = Workspace::new("Test Workspace".to_string());
        registry.register_workspace("test".to_string(), test_ws).unwrap();

        WorkspaceRouter::new(Arc::new(registry))
    }

    #[test]
    fn test_extract_workspace_context_with_prefix() {
        let router = create_test_router();

        let context = router.extract_workspace_context("/workspace/test/api/users").unwrap();

        assert_eq!(context.workspace_id, "test");
        assert_eq!(context.original_path, "/workspace/test/api/users");
        assert_eq!(context.stripped_path, "/api/users");
        assert_eq!(context.workspace.name(), "Test Workspace");
    }

    #[test]
    fn test_extract_workspace_context_default() {
        let router = create_test_router();

        let context = router.extract_workspace_context("/api/users").unwrap();

        assert_eq!(context.workspace_id, "default");
        assert_eq!(context.original_path, "/api/users");
        assert_eq!(context.stripped_path, "/api/users");
        assert_eq!(context.workspace.name(), "Default");
    }

    #[test]
    fn test_extract_workspace_context_nonexistent() {
        let router = create_test_router();

        let result = router.extract_workspace_context("/workspace/nonexistent/api/users");

        assert!(result.is_err());
    }

    #[test]
    fn test_multi_tenant_disabled() {
        let config = MultiTenantConfig {
            enabled: false,
            ..Default::default()
        };

        let mut registry = MultiTenantWorkspaceRegistry::new(config);

        let default_ws = Workspace::new("Default".to_string());
        registry.register_workspace("default".to_string(), default_ws).unwrap();

        let router = WorkspaceRouter::new(Arc::new(registry));

        let context = router.extract_workspace_context("/workspace/test/api/users").unwrap();

        // Should use default workspace when multi-tenant is disabled
        assert_eq!(context.workspace_id, "default");
        assert_eq!(context.stripped_path, "/workspace/test/api/users");
    }
}
