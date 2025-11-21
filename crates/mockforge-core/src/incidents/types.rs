//! Core types for incident management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Status of a drift incident
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum IncidentStatus {
    /// Incident is open and needs attention
    Open,
    /// Incident has been acknowledged
    Acknowledged,
    /// Incident has been resolved
    Resolved,
    /// Incident has been closed
    Closed,
}

/// Type of incident
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IncidentType {
    /// Breaking change detected
    BreakingChange,
    /// Drift budget threshold exceeded
    ThresholdExceeded,
}

/// Severity level for incidents
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "lowercase")]
pub enum IncidentSeverity {
    /// Critical severity
    Critical,
    /// High severity
    High,
    /// Medium severity
    Medium,
    /// Low severity
    Low,
}

/// A drift incident
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftIncident {
    /// Unique identifier for the incident
    pub id: String,
    /// Associated budget ID (if any)
    pub budget_id: Option<String>,
    /// Workspace ID (for multi-tenant support)
    pub workspace_id: Option<String>,
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Type of incident
    pub incident_type: IncidentType,
    /// Severity of the incident
    pub severity: IncidentSeverity,
    /// Current status
    pub status: IncidentStatus,
    /// When the incident was detected
    pub detected_at: i64,
    /// When the incident was resolved (if resolved)
    pub resolved_at: Option<i64>,
    /// Additional details (JSON)
    pub details: serde_json::Value,
    /// External ticket ID (Jira, Linear, etc.)
    pub external_ticket_id: Option<String>,
    /// External ticket URL
    pub external_ticket_url: Option<String>,
    /// When the incident was created
    pub created_at: i64,
    /// When the incident was last updated
    pub updated_at: i64,
    /// Sync cycle ID that triggered this incident (if from API sync)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sync_cycle_id: Option<String>,
    /// Contract diff ID that triggered this incident (if from contract diff)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract_diff_id: Option<String>,
    /// Before sample - contract/spec state before drift was detected
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_sample: Option<serde_json::Value>,
    /// After sample - contract/spec state after drift was detected
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after_sample: Option<serde_json::Value>,
    /// Results from fitness function tests
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fitness_test_results: Vec<crate::contract_drift::fitness::FitnessTestResult>,
    /// Consumer impact analysis (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affected_consumers: Option<crate::contract_drift::consumer_mapping::ConsumerImpact>,
    /// Protocol type (HTTP, gRPC, WebSocket, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<crate::protocol_abstraction::Protocol>,
}

impl DriftIncident {
    /// Create a new drift incident
    pub fn new(
        id: String,
        endpoint: String,
        method: String,
        incident_type: IncidentType,
        severity: IncidentSeverity,
        details: serde_json::Value,
    ) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            id,
            budget_id: None,
            workspace_id: None,
            endpoint,
            method,
            incident_type,
            severity,
            status: IncidentStatus::Open,
            detected_at: now,
            resolved_at: None,
            details,
            external_ticket_id: None,
            external_ticket_url: None,
            created_at: now,
            updated_at: now,
            sync_cycle_id: None,
            contract_diff_id: None,
            before_sample: None,
            after_sample: None,
            fitness_test_results: Vec::new(),
            affected_consumers: None,
            protocol: None,
        }
    }

    /// Mark the incident as acknowledged
    pub fn acknowledge(&mut self) {
        if self.status == IncidentStatus::Open {
            self.status = IncidentStatus::Acknowledged;
            self.updated_at = chrono::Utc::now().timestamp();
        }
    }

    /// Mark the incident as resolved
    pub fn resolve(&mut self) {
        if self.status != IncidentStatus::Closed {
            self.status = IncidentStatus::Resolved;
            self.resolved_at = Some(chrono::Utc::now().timestamp());
            self.updated_at = chrono::Utc::now().timestamp();
        }
    }

    /// Close the incident
    pub fn close(&mut self) {
        self.status = IncidentStatus::Closed;
        if self.resolved_at.is_none() {
            self.resolved_at = Some(chrono::Utc::now().timestamp());
        }
        self.updated_at = chrono::Utc::now().timestamp();
    }

    /// Link an external ticket to this incident
    pub fn link_external_ticket(&mut self, ticket_id: String, ticket_url: Option<String>) {
        self.external_ticket_id = Some(ticket_id);
        self.external_ticket_url = ticket_url;
        self.updated_at = chrono::Utc::now().timestamp();
    }
}

/// External ticket information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalTicket {
    /// Ticket ID in the external system
    pub ticket_id: String,
    /// URL to the ticket
    pub ticket_url: Option<String>,
    /// External system type (jira, linear, etc.)
    pub system_type: String,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Query filters for incidents
#[derive(Debug, Clone, Default)]
pub struct IncidentQuery {
    /// Filter by status
    pub status: Option<IncidentStatus>,
    /// Filter by severity
    pub severity: Option<IncidentSeverity>,
    /// Filter by endpoint
    pub endpoint: Option<String>,
    /// Filter by method
    pub method: Option<String>,
    /// Filter by incident type
    pub incident_type: Option<IncidentType>,
    /// Filter by workspace ID
    pub workspace_id: Option<String>,
    /// Filter by date range (start)
    pub start_date: Option<i64>,
    /// Filter by date range (end)
    pub end_date: Option<i64>,
    /// Limit number of results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}
