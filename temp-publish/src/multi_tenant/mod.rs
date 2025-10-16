//! Multi-tenant workspace support for MockForge
//!
//! This module provides infrastructure for hosting multiple isolated workspaces
//! in a single MockForge instance, enabling namespace separation and tenant isolation.

pub mod middleware;

mod registry;

pub use middleware::{WorkspaceContext, WorkspaceRouter};
pub use registry::{
    MultiTenantConfig, MultiTenantWorkspaceRegistry, RoutingStrategy, TenantWorkspace,
    WorkspaceStats,
};
