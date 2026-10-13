//! Structural incident access used by Intelligence handlers without a Core dependency.

use async_trait::async_trait;
use mockforge_foundation::incidents_types::{DriftIncident, IncidentQuery, IncidentStatus};
use std::collections::HashMap;

/// The existing incident manager's read and lifecycle operations.
#[async_trait]
pub trait StructuralIncidents: Send + Sync {
    /// Query persisted incidents using the manager's existing filters.
    async fn query_incidents(&self, query: IncidentQuery) -> Vec<DriftIncident>;
    /// Find an incident by its identifier.
    async fn get_incident(&self, id: &str) -> Option<DriftIncident>;
    /// Acknowledge an existing incident.
    async fn acknowledge_incident(&self, id: &str) -> Option<DriftIncident>;
    /// Resolve an existing incident.
    async fn resolve_incident(&self, id: &str) -> Option<DriftIncident>;
    /// Close an existing incident.
    async fn close_incident(&self, id: &str) -> Option<DriftIncident>;
    /// Attach an external ticket to an existing incident.
    async fn link_external_ticket(
        &self,
        id: &str,
        ticket_id: String,
        ticket_url: Option<String>,
    ) -> Option<DriftIncident>;
    /// Count incidents by their current lifecycle status.
    async fn get_statistics(&self) -> HashMap<IncidentStatus, usize>;
}
