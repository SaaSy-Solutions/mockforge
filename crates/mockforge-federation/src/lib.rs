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
