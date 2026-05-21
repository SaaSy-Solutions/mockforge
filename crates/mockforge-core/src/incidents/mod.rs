//! Incident management for drift and contract violations
//!
//! This module provides functionality for creating, storing, and managing incidents
//! related to contract drift and breaking changes.
//!
//! Issue #562 phase 9 moved the AI-coupled pieces (`semantic_manager`,
//! `integrations` config + Slack/Jira formatters) into
//! `mockforge_intelligence::incidents`. The structural pieces below
//! (`manager`, `store`) and the shared types (re-exported from
//! `mockforge_foundation::incidents_types` via `types.rs`) stay in core —
//! they have no AI dependencies.

pub mod manager;
pub mod store;
pub mod types;

// The AI-coupled pieces now live in `mockforge-intelligence`; these
// re-exports keep existing `mockforge_core::incidents::{...}` callers
// resolving unchanged.
pub use mockforge_intelligence::incidents::{integrations, semantic_manager};

// Integration formatters (always available, not behind feature flag for simplicity).
// Same `#[path]` aliasing the legacy module used, but now pointing at the
// intelligence-side mod aliases of the same name.
pub use mockforge_intelligence::incidents::{jira_formatter, slack_formatter};

pub use manager::IncidentManager;
pub use mockforge_intelligence::incidents::{SemanticIncident, SemanticIncidentManager};
pub use store::IncidentStore;
pub use types::{
    DriftIncident, ExternalTicket, IncidentQuery, IncidentSeverity, IncidentStatus, IncidentType,
};
