//! Semantic drift incident manager
//!
//! This module provides management for semantic drift incidents,
//! which are separate from structural drift incidents but can be
//! cross-linked for unified contract health tracking.

use crate::ai_contract_diff::semantic_analyzer::{SemanticChangeType, SemanticDriftResult};
use crate::incidents::types::{IncidentSeverity, IncidentStatus};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Semantic drift incident
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticIncident {
    /// Unique identifier
    pub id: String,
    /// Workspace ID
    pub workspace_id: Option<String>,
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Semantic change type
    pub semantic_change_type: SemanticChangeType,
    /// Severity
    pub severity: IncidentSeverity,
    /// Status
    pub status: IncidentStatus,
    /// Semantic confidence score (0.0-1.0)
    pub semantic_confidence: f64,
    /// Soft-breaking score (0.0-1.0)
    pub soft_breaking_score: f64,
    /// Full LLM analysis
    pub llm_analysis: serde_json::Value,
    /// Before semantic state
    pub before_semantic_state: serde_json::Value,
    /// After semantic state
    pub after_semantic_state: serde_json::Value,
    /// Additional details
    pub details: serde_json::Value,
    /// Link to related structural drift incident
    pub related_drift_incident_id: Option<String>,
    /// Contract diff ID that triggered this
    pub contract_diff_id: Option<String>,
    /// External ticket tracking
    pub external_ticket_id: Option<String>,
    /// External ticket URL (e.g., Jira, GitHub issue)
    pub external_ticket_url: Option<String>,
    /// Timestamps
    pub detected_at: i64,
    /// Creation timestamp
    pub created_at: i64,
    /// Acknowledgment timestamp
    pub acknowledged_at: Option<i64>,
    /// Resolution timestamp
    pub resolved_at: Option<i64>,
    /// Closure timestamp
    pub closed_at: Option<i64>,
    /// Last update timestamp
    pub updated_at: i64,
}

impl SemanticIncident {
    /// Create a new semantic incident from a semantic drift result
    pub fn from_drift_result(
        result: &SemanticDriftResult,
        endpoint: String,
        method: String,
        workspace_id: Option<String>,
        related_drift_incident_id: Option<String>,
        contract_diff_id: Option<String>,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();

        // Determine severity based on soft-breaking score and confidence
        let severity = if result.soft_breaking_score >= 0.8 && result.semantic_confidence >= 0.8 {
            IncidentSeverity::Critical
        } else if result.soft_breaking_score >= 0.65 || result.semantic_confidence >= 0.75 {
            IncidentSeverity::High
        } else if result.soft_breaking_score >= 0.5 || result.semantic_confidence >= 0.65 {
            IncidentSeverity::Medium
        } else {
            IncidentSeverity::Low
        };

        let details = serde_json::json!({
            "change_type": result.change_type,
            "semantic_confidence": result.semantic_confidence,
            "soft_breaking_score": result.soft_breaking_score,
            "mismatch_count": result.semantic_mismatches.len(),
        });

        Self {
            id,
            workspace_id,
            endpoint,
            method,
            semantic_change_type: result.change_type.clone(),
            severity,
            status: IncidentStatus::Open,
            semantic_confidence: result.semantic_confidence,
            soft_breaking_score: result.soft_breaking_score,
            llm_analysis: result.llm_analysis.clone(),
            before_semantic_state: result.before_semantic_state.clone(),
            after_semantic_state: result.after_semantic_state.clone(),
            details,
            related_drift_incident_id,
            contract_diff_id,
            external_ticket_id: None,
            external_ticket_url: None,
            detected_at: now,
            created_at: now,
            acknowledged_at: None,
            resolved_at: None,
            closed_at: None,
            updated_at: now,
        }
    }

    /// Mark the incident as acknowledged
    pub fn acknowledge(&mut self) {
        if self.status == IncidentStatus::Open {
            self.status = IncidentStatus::Acknowledged;
            self.acknowledged_at = Some(Utc::now().timestamp());
            self.updated_at = Utc::now().timestamp();
        }
    }

    /// Mark the incident as resolved
    pub fn resolve(&mut self) {
        if self.status != IncidentStatus::Closed {
            self.status = IncidentStatus::Resolved;
            self.resolved_at = Some(Utc::now().timestamp());
            self.updated_at = Utc::now().timestamp();
        }
    }

    /// Mark the incident as closed
    pub fn close(&mut self) {
        self.status = IncidentStatus::Closed;
        self.closed_at = Some(Utc::now().timestamp());
        self.updated_at = Utc::now().timestamp();
    }
}

/// Semantic incident manager
pub struct SemanticIncidentManager {
    /// In-memory store of incidents
    incidents: std::sync::Arc<tokio::sync::RwLock<HashMap<String, SemanticIncident>>>,
    /// Webhook configurations for notifications
    webhook_configs: Vec<crate::incidents::integrations::WebhookConfig>,
}

impl SemanticIncidentManager {
    /// Create a new semantic incident manager
    pub fn new() -> Self {
        Self {
            incidents: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            webhook_configs: Vec::new(),
        }
    }

    /// Create a new semantic incident manager with webhooks
    pub fn new_with_webhooks(
        webhook_configs: Vec<crate::incidents::integrations::WebhookConfig>,
    ) -> Self {
        Self {
            incidents: std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::new())),
            webhook_configs,
        }
    }

    /// Add webhook configuration
    pub fn add_webhook(&mut self, config: crate::incidents::integrations::WebhookConfig) {
        self.webhook_configs.push(config);
    }

    /// Create a semantic incident from a drift result
    pub async fn create_incident(
        &self,
        result: &SemanticDriftResult,
        endpoint: String,
        method: String,
        workspace_id: Option<String>,
        related_drift_incident_id: Option<String>,
        contract_diff_id: Option<String>,
    ) -> SemanticIncident {
        let incident = SemanticIncident::from_drift_result(
            result,
            endpoint,
            method,
            workspace_id,
            related_drift_incident_id,
            contract_diff_id,
        );

        let id = incident.id.clone();
        let mut incidents = self.incidents.write().await;
        incidents.insert(id, incident.clone());

        // Trigger webhook notifications
        self.trigger_webhooks("semantic.incident.created", &incident).await;

        incident
    }

    /// Trigger webhook notifications for an event
    async fn trigger_webhooks(&self, event_type: &str, incident: &SemanticIncident) {
        use crate::incidents::integrations::send_webhook;
        use serde_json::json;

        for webhook in &self.webhook_configs {
            if !webhook.enabled {
                continue;
            }

            if !webhook.events.is_empty() && !webhook.events.contains(&event_type.to_string()) {
                continue;
            }

            let payload = json!({
                "event": event_type,
                "incident": {
                    "id": incident.id,
                    "endpoint": incident.endpoint,
                    "method": incident.method,
                    "semantic_change_type": format!("{:?}", incident.semantic_change_type),
                    "severity": format!("{:?}", incident.severity),
                    "status": format!("{:?}", incident.status),
                    "semantic_confidence": incident.semantic_confidence,
                    "soft_breaking_score": incident.soft_breaking_score,
                    "details": incident.details,
                    "created_at": incident.created_at,
                }
            });

            let webhook_clone = webhook.clone();
            tokio::spawn(async move {
                if let Err(e) = send_webhook(&webhook_clone, &payload).await {
                    tracing::warn!("Failed to send webhook: {}", e);
                }
            });
        }
    }

    /// Get an incident by ID
    pub async fn get_incident(&self, id: &str) -> Option<SemanticIncident> {
        let incidents = self.incidents.read().await;
        incidents.get(id).cloned()
    }

    /// Update an incident
    pub async fn update_incident(&self, incident: SemanticIncident) {
        let mut incidents = self.incidents.write().await;
        incidents.insert(incident.id.clone(), incident);
    }

    /// List incidents with optional filters
    pub async fn list_incidents(
        &self,
        workspace_id: Option<&str>,
        endpoint: Option<&str>,
        method: Option<&str>,
        status: Option<IncidentStatus>,
        limit: Option<usize>,
    ) -> Vec<SemanticIncident> {
        let incidents = self.incidents.read().await;
        let mut filtered: Vec<_> = incidents
            .values()
            .filter(|inc| {
                if let Some(ws_id) = workspace_id {
                    if inc.workspace_id.as_deref() != Some(ws_id) {
                        return false;
                    }
                }
                if let Some(ep) = endpoint {
                    if inc.endpoint != ep {
                        return false;
                    }
                }
                if let Some(m) = method {
                    if inc.method != m {
                        return false;
                    }
                }
                if let Some(s) = status {
                    if inc.status != s {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        // Sort by detected_at descending
        filtered.sort_by_key(|inc| std::cmp::Reverse(inc.detected_at));

        if let Some(limit) = limit {
            filtered.truncate(limit);
        }

        filtered
    }

    /// Acknowledge an incident
    pub async fn acknowledge_incident(&self, id: &str) -> Option<SemanticIncident> {
        let mut incidents = self.incidents.write().await;
        if let Some(incident) = incidents.get_mut(id) {
            incident.acknowledge();
            Some(incident.clone())
        } else {
            None
        }
    }

    /// Resolve an incident
    pub async fn resolve_incident(&self, id: &str) -> Option<SemanticIncident> {
        let mut incidents = self.incidents.write().await;
        if let Some(incident) = incidents.get_mut(id) {
            incident.resolve();
            Some(incident.clone())
        } else {
            None
        }
    }
}

impl Default for SemanticIncidentManager {
    fn default() -> Self {
        Self::new()
    }
}
