//! Incident management for drift and contract violations
//!
//! This module provides functionality for creating, storing, and managing incidents
//! related to contract drift and breaking changes.
//!
//! NOTE: `semantic_manager` remains in `mockforge-core` as it depends on
//! `ai_contract_diff::semantic_analyzer` which is being extracted in Phase 4.

pub mod integrations;
pub mod manager;
pub mod store;
pub mod types;

// Integration formatters (always available)
#[path = "integrations/slack.rs"]
pub mod slack_formatter;

#[path = "integrations/jira.rs"]
pub mod jira_formatter;

pub use manager::IncidentManager;
pub use store::IncidentStore;
pub use types::{
    DriftIncident, ExternalTicket, IncidentQuery, IncidentSeverity, IncidentStatus, IncidentType,
};
