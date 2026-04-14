//! Core types for incident management
//!
//! Re-exported from `mockforge-foundation::incidents_types` so
//! `mockforge-core` and `mockforge-contracts` share the same incident types.

pub use mockforge_foundation::incidents_types::{
    DriftIncident, ExternalTicket, IncidentQuery, IncidentSeverity, IncidentStatus, IncidentType,
};
