//! Multi-tenant workspace configuration types
//!
//! Pure data types extracted from `mockforge-core::multi_tenant::registry`
//! so consumers do not need to depend on deprecated core modules. The
//! `MultiTenantWorkspaceRegistry` and `WorkspaceRouter` impls remain in core
//! because they depend on `Workspace`, `RouteRegistry`, and
//! `CentralizedRequestLogger`.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Multi-tenant configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
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
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum RoutingStrategy {
    /// Path-based routing: /workspace/{id}/path
    Path,
    /// Port-based routing: different port per workspace
    Port,
    /// Both path and port-based routing
    Both,
}
