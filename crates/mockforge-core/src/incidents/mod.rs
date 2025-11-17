//! Incident management for drift and contract violations
//!
//! This module provides functionality for creating, storing, and managing incidents
//! related to contract drift and breaking changes.

pub mod integrations;
pub mod manager;
pub mod store;
pub mod types;

// Integration formatters (always available, not behind feature flag for simplicity)
#[path = "integrations/slack.rs"]
pub mod slack_formatter;

#[path = "integrations/jira.rs"]
pub mod jira_formatter;

pub use manager::IncidentManager;
pub use store::IncidentStore;
pub use types::{
    DriftIncident, ExternalTicket, IncidentQuery, IncidentSeverity, IncidentStatus, IncidentType,
};
