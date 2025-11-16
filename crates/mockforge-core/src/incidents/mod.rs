//! Incident management for drift and contract violations
//!
//! This module provides functionality for creating, storing, and managing incidents
//! related to contract drift and breaking changes.

pub mod integrations;
pub mod manager;
pub mod store;
pub mod types;

pub use manager::IncidentManager;
pub use store::IncidentStore;
pub use types::{
    DriftIncident, ExternalTicket, IncidentQuery, IncidentSeverity, IncidentStatus, IncidentType,
};
