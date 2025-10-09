//! Multi-tenant workspace support for MockForge
//!
//! This module provides infrastructure for hosting multiple isolated workspaces
//! in a single MockForge instance, enabling namespace separation and tenant isolation.

use crate::workspace::{EntityId, Workspace};
use crate::routing::RouteRegistry;
use crate::request_logger::CentralizedRequestLogger;
use crate::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Multi-tenant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiTenantConfig {
    /// Enable multi-tenant mode
    pub enabled: bool,
    /// Routing strategy (path-based, port-based, or both)
    pub routing_strategy: RoutingStrategy,
    /// Workspace path prefix (e.g., "/workspace" or "/w")
    pub workspace_prefix: String,
    /// Default workspace ID (used when no workspace specified in request)
    pub default_workspace: String,
    /// Maximum number of workspaces allowed
    pub max_workspaces: Option<usize>,
    /// Workspace-specific port mappings (for port-based routing)
    #[serde(default)]
    pub workspace_ports: HashMap<String, u16>,
    /// Enable workspace auto-discovery from config directory
    pub auto_discover: bool,
    /// Configuration directory for workspace configs
    pub config_directory: Option<String>,
}

impl Default for MultiTenantConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            routing_strategy: RoutingStrategy::Path,
            workspace_prefix: "/workspace".to_string(),
            default_workspace: "default".to_string(),
            max_workspaces: None,
            workspace_ports: HashMap::new(),
            auto_discover: false,
            config_directory: None,
        }
    }
}

/// Routing strategy for multi-tenant workspaces
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RoutingStrategy {
    /// Path-based routing: /workspace/{id}/path
    Path,
    /// Port-based routing: different port per workspace
    Port,
    /// Both path and port-based routing
    Both,
}

/// Tenant workspace wrapper with isolated resources
#[derive(Debug, Clone)]
pub struct TenantWorkspace {
    /// Workspace metadata and configuration
    pub workspace: Workspace,
    /// Workspace-specific route registry
    pub route_registry: Arc<RwLock<RouteRegistry>>,
    /// Last access timestamp
    pub last_accessed: DateTime<Utc>,
    /// Whether this workspace is enabled
    pub enabled: bool,
    /// Workspace-specific statistics
    pub stats: WorkspaceStats,
}

/// Statistics for a tenant workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceStats {
    /// Total number of requests handled
    pub total_requests: u64,
    /// Total number of active routes
    pub active_routes: usize,
    /// Last request timestamp
    pub last_request_at: Option<DateTime<Utc>>,
    /// Created at timestamp
    pub created_at: DateTime<Utc>,
    /// Average response time in milliseconds
    pub avg_response_time_ms: f64,
}

impl Default for WorkspaceStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            active_routes: 0,
            last_request_at: None,
            created_at: Utc::now(),
            avg_response_time_ms: 0.0,
        }
    }
}

/// Multi-tenant workspace registry for managing multiple isolated workspaces
#[derive(Debug, Clone)]
pub struct MultiTenantWorkspaceRegistry {
    /// Tenant workspaces indexed by ID
    workspaces: Arc<RwLock<HashMap<EntityId, TenantWorkspace>>>,
    /// Default workspace ID
    default_workspace_id: EntityId,
    /// Configuration
    config: MultiTenantConfig,
    /// Global request logger (for aggregated logging)
    global_logger: Arc<CentralizedRequestLogger>,
}

impl MultiTenantWorkspaceRegistry {
    /// Create a new multi-tenant workspace registry
    pub fn new(config: MultiTenantConfig) -> Self {
        let default_workspace_id = config.default_workspace.clone();

        Self {
            workspaces: Arc::new(RwLock::new(HashMap::new())),
            default_workspace_id,
            config,
            global_logger: Arc::new(CentralizedRequestLogger::new(10000)), // Keep last 10000 requests
        }
    }

    /// Create with default configuration
    pub fn with_default_workspace(workspace_name: String) -> Self {
        let mut config = MultiTenantConfig::default();
        config.default_workspace = "default".to_string();

        let mut registry = Self::new(config);

        // Create and register default workspace
        let default_workspace = Workspace::new(workspace_name);
        let _ = registry.register_workspace("default".to_string(), default_workspace);

        registry
    }

    /// Register a new workspace
    pub fn register_workspace(
        &mut self,
        workspace_id: String,
        workspace: Workspace,
    ) -> Result<()> {
        // Check max workspaces limit
        if let Some(max) = self.config.max_workspaces {
            let current_count = self.workspaces.read()
                .map_err(|e| Error::generic(format!("Failed to read workspaces: {}", e)))?
                .len();

            if current_count >= max {
                return Err(Error::generic(format!(
                    "Maximum number of workspaces ({}) exceeded",
                    max
                )));
            }
        }

        let tenant_workspace = TenantWorkspace {
            workspace,
            route_registry: Arc::new(RwLock::new(RouteRegistry::new())),
            last_accessed: Utc::now(),
            enabled: true,
            stats: WorkspaceStats::default(),
        };

        self.workspaces.write()
            .map_err(|e| Error::generic(format!("Failed to write workspaces: {}", e)))?
            .insert(workspace_id, tenant_workspace);

        Ok(())
    }

    /// Get a workspace by ID
    pub fn get_workspace(&self, workspace_id: &str) -> Result<TenantWorkspace> {
        let workspaces = self.workspaces.read()
            .map_err(|e| Error::generic(format!("Failed to read workspaces: {}", e)))?;

        workspaces
            .get(workspace_id)
            .cloned()
            .ok_or_else(|| Error::generic(format!("Workspace '{}' not found", workspace_id)))
    }

    /// Get the default workspace
    pub fn get_default_workspace(&self) -> Result<TenantWorkspace> {
        self.get_workspace(&self.default_workspace_id)
    }

    /// Update workspace
    pub fn update_workspace(
        &mut self,
        workspace_id: &str,
        workspace: Workspace,
    ) -> Result<()> {
        let mut workspaces = self.workspaces.write()
            .map_err(|e| Error::generic(format!("Failed to write workspaces: {}", e)))?;

        if let Some(tenant_workspace) = workspaces.get_mut(workspace_id) {
            tenant_workspace.workspace = workspace;
            Ok(())
        } else {
            Err(Error::generic(format!("Workspace '{}' not found", workspace_id)))
        }
    }

    /// Remove a workspace
    pub fn remove_workspace(&mut self, workspace_id: &str) -> Result<()> {
        // Prevent removing default workspace
        if workspace_id == self.default_workspace_id {
            return Err(Error::generic("Cannot remove default workspace".to_string()));
        }

        self.workspaces.write()
            .map_err(|e| Error::generic(format!("Failed to write workspaces: {}", e)))?
            .remove(workspace_id)
            .ok_or_else(|| Error::generic(format!("Workspace '{}' not found", workspace_id)))?;

        Ok(())
    }

    /// List all workspaces
    pub fn list_workspaces(&self) -> Result<Vec<(String, TenantWorkspace)>> {
        let workspaces = self.workspaces.read()
            .map_err(|e| Error::generic(format!("Failed to read workspaces: {}", e)))?;

        Ok(workspaces
            .iter()
            .map(|(id, ws)| (id.clone(), ws.clone()))
            .collect())
    }

    /// Get workspace by ID or default
    pub fn resolve_workspace(&self, workspace_id: Option<&str>) -> Result<TenantWorkspace> {
        if let Some(id) = workspace_id {
            self.get_workspace(id)
        } else {
            self.get_default_workspace()
        }
    }

    /// Update workspace last accessed time
    pub fn touch_workspace(&mut self, workspace_id: &str) -> Result<()> {
        let mut workspaces = self.workspaces.write()
            .map_err(|e| Error::generic(format!("Failed to write workspaces: {}", e)))?;

        if let Some(tenant_workspace) = workspaces.get_mut(workspace_id) {
            tenant_workspace.last_accessed = Utc::now();
            Ok(())
        } else {
            Err(Error::generic(format!("Workspace '{}' not found", workspace_id)))
        }
    }

    /// Update workspace statistics
    pub fn update_workspace_stats(
        &mut self,
        workspace_id: &str,
        response_time_ms: f64,
    ) -> Result<()> {
        let mut workspaces = self.workspaces.write()
            .map_err(|e| Error::generic(format!("Failed to write workspaces: {}", e)))?;

        if let Some(tenant_workspace) = workspaces.get_mut(workspace_id) {
            tenant_workspace.stats.total_requests += 1;
            tenant_workspace.stats.last_request_at = Some(Utc::now());

            // Update average response time using running average
            let n = tenant_workspace.stats.total_requests as f64;
            tenant_workspace.stats.avg_response_time_ms =
                ((tenant_workspace.stats.avg_response_time_ms * (n - 1.0)) + response_time_ms) / n;

            Ok(())
        } else {
            Err(Error::generic(format!("Workspace '{}' not found", workspace_id)))
        }
    }

    /// Get workspace count
    pub fn workspace_count(&self) -> Result<usize> {
        let workspaces = self.workspaces.read()
            .map_err(|e| Error::generic(format!("Failed to read workspaces: {}", e)))?;

        Ok(workspaces.len())
    }

    /// Check if workspace exists
    pub fn workspace_exists(&self, workspace_id: &str) -> bool {
        self.workspaces.read()
            .map(|ws| ws.contains_key(workspace_id))
            .unwrap_or(false)
    }

    /// Enable/disable a workspace
    pub fn set_workspace_enabled(&mut self, workspace_id: &str, enabled: bool) -> Result<()> {
        let mut workspaces = self.workspaces.write()
            .map_err(|e| Error::generic(format!("Failed to write workspaces: {}", e)))?;

        if let Some(tenant_workspace) = workspaces.get_mut(workspace_id) {
            tenant_workspace.enabled = enabled;
            Ok(())
        } else {
            Err(Error::generic(format!("Workspace '{}' not found", workspace_id)))
        }
    }

    /// Get the global request logger
    pub fn global_logger(&self) -> &Arc<CentralizedRequestLogger> {
        &self.global_logger
    }

    /// Get configuration
    pub fn config(&self) -> &MultiTenantConfig {
        &self.config
    }

    /// Extract workspace ID from request path
    pub fn extract_workspace_id_from_path(&self, path: &str) -> Option<String> {
        if !self.config.enabled {
            return None;
        }

        let prefix = &self.config.workspace_prefix;

        // Check if path starts with workspace prefix
        if !path.starts_with(prefix) {
            return None;
        }

        // Extract workspace ID from path: /workspace/{id}/...
        let remaining = &path[prefix.len()..];

        // Skip leading slash
        let remaining = remaining.strip_prefix('/').unwrap_or(remaining);

        // Get first path segment (workspace ID)
        remaining
            .split('/')
            .next()
            .filter(|id| !id.is_empty())
            .map(|id| id.to_string())
    }

    /// Strip workspace prefix from path
    pub fn strip_workspace_prefix(&self, path: &str, workspace_id: &str) -> String {
        if !self.config.enabled {
            return path.to_string();
        }

        let prefix = format!("{}/{}", self.config.workspace_prefix, workspace_id);

        if path.starts_with(&prefix) {
            let remaining = &path[prefix.len()..];
            if remaining.is_empty() {
                "/".to_string()
            } else {
                remaining.to_string()
            }
        } else {
            path.to_string()
        }
    }
}

impl TenantWorkspace {
    /// Create a new tenant workspace
    pub fn new(workspace: Workspace) -> Self {
        Self {
            workspace,
            route_registry: Arc::new(RwLock::new(RouteRegistry::new())),
            last_accessed: Utc::now(),
            enabled: true,
            stats: WorkspaceStats::default(),
        }
    }

    /// Get workspace ID
    pub fn id(&self) -> &str {
        &self.workspace.id
    }

    /// Get workspace name
    pub fn name(&self) -> &str {
        &self.workspace.name
    }

    /// Get route registry
    pub fn route_registry(&self) -> &Arc<RwLock<RouteRegistry>> {
        &self.route_registry
    }

    /// Get workspace statistics
    pub fn stats(&self) -> &WorkspaceStats {
        &self.stats
    }

    /// Rebuild route registry from workspace routes
    pub fn rebuild_routes(&mut self) -> Result<()> {
        let routes = self.workspace.get_routes();

        let mut registry = self.route_registry.write()
            .map_err(|e| Error::generic(format!("Failed to write route registry: {}", e)))?;

        // Clear existing routes
        *registry = RouteRegistry::new();

        // Add all routes from workspace
        for route in routes {
            registry.add_http_route(route)?;
        }

        // Update stats - count total number of routes
        self.stats.active_routes = self.workspace.requests.len()
            + self.workspace.folders.iter()
                .map(|f| Self::count_folder_requests(f))
                .sum::<usize>();

        Ok(())
    }

    /// Count requests in a folder recursively
    fn count_folder_requests(folder: &crate::workspace::Folder) -> usize {
        folder.requests.len() + folder.folders.iter().map(Self::count_folder_requests).sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_tenant_config_default() {
        let config = MultiTenantConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.routing_strategy, RoutingStrategy::Path);
        assert_eq!(config.workspace_prefix, "/workspace");
        assert_eq!(config.default_workspace, "default");
    }

    #[test]
    fn test_multi_tenant_registry_creation() {
        let config = MultiTenantConfig::default();
        let registry = MultiTenantWorkspaceRegistry::new(config);
        assert_eq!(registry.workspace_count().unwrap(), 0);
    }

    #[test]
    fn test_register_workspace() {
        let config = MultiTenantConfig::default();
        let mut registry = MultiTenantWorkspaceRegistry::new(config);

        let workspace = Workspace::new("Test Workspace".to_string());
        registry.register_workspace("test".to_string(), workspace).unwrap();

        assert_eq!(registry.workspace_count().unwrap(), 1);
        assert!(registry.workspace_exists("test"));
    }

    #[test]
    fn test_max_workspaces_limit() {
        let mut config = MultiTenantConfig::default();
        config.max_workspaces = Some(2);

        let mut registry = MultiTenantWorkspaceRegistry::new(config);

        // Register first workspace
        registry.register_workspace("ws1".to_string(), Workspace::new("WS1".to_string())).unwrap();

        // Register second workspace
        registry.register_workspace("ws2".to_string(), Workspace::new("WS2".to_string())).unwrap();

        // Third should fail
        let result = registry.register_workspace("ws3".to_string(), Workspace::new("WS3".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_workspace_id_from_path() {
        let mut config = MultiTenantConfig::default();
        config.enabled = true;

        let registry = MultiTenantWorkspaceRegistry::new(config);

        // Test valid path
        let workspace_id = registry.extract_workspace_id_from_path("/workspace/project-a/api/users");
        assert_eq!(workspace_id, Some("project-a".to_string()));

        // Test path without workspace
        let workspace_id = registry.extract_workspace_id_from_path("/api/users");
        assert_eq!(workspace_id, None);

        // Test root workspace path
        let workspace_id = registry.extract_workspace_id_from_path("/workspace/test");
        assert_eq!(workspace_id, Some("test".to_string()));
    }

    #[test]
    fn test_strip_workspace_prefix() {
        let mut config = MultiTenantConfig::default();
        config.enabled = true;

        let registry = MultiTenantWorkspaceRegistry::new(config);

        // Test stripping prefix
        let stripped = registry.strip_workspace_prefix("/workspace/project-a/api/users", "project-a");
        assert_eq!(stripped, "/api/users");

        // Test path without prefix
        let stripped = registry.strip_workspace_prefix("/api/users", "project-a");
        assert_eq!(stripped, "/api/users");

        // Test root path
        let stripped = registry.strip_workspace_prefix("/workspace/project-a", "project-a");
        assert_eq!(stripped, "/");
    }

    #[test]
    fn test_workspace_stats_update() {
        let config = MultiTenantConfig::default();
        let mut registry = MultiTenantWorkspaceRegistry::new(config);

        let workspace = Workspace::new("Test Workspace".to_string());
        registry.register_workspace("test".to_string(), workspace).unwrap();

        // Update stats with response time
        registry.update_workspace_stats("test", 100.0).unwrap();

        let tenant_ws = registry.get_workspace("test").unwrap();
        assert_eq!(tenant_ws.stats.total_requests, 1);
        assert_eq!(tenant_ws.stats.avg_response_time_ms, 100.0);

        // Update again with different response time
        registry.update_workspace_stats("test", 200.0).unwrap();

        let tenant_ws = registry.get_workspace("test").unwrap();
        assert_eq!(tenant_ws.stats.total_requests, 2);
        assert_eq!(tenant_ws.stats.avg_response_time_ms, 150.0);
    }

    #[test]
    fn test_cannot_remove_default_workspace() {
        let mut config = MultiTenantConfig::default();
        config.default_workspace = "default".to_string();

        let mut registry = MultiTenantWorkspaceRegistry::new(config);

        registry.register_workspace("default".to_string(), Workspace::new("Default".to_string())).unwrap();

        // Try to remove default workspace
        let result = registry.remove_workspace("default");
        assert!(result.is_err());
    }
}
