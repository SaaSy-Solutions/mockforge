//! Incident manager for creating and managing drift incidents
//!
//! This module provides high-level functionality for incident lifecycle management.

use crate::incidents::store::IncidentStore;
use crate::incidents::types::{
    DriftIncident, IncidentQuery, IncidentSeverity, IncidentStatus, IncidentType,
};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Manager for drift incidents
#[derive(Debug, Clone)]
pub struct IncidentManager {
    store: Arc<IncidentStore>,
    /// Webhook configurations for incident notifications
    webhook_configs: Vec<crate::incidents::integrations::WebhookConfig>,
}

impl IncidentManager {
    /// Create a new incident manager
    pub fn new(store: Arc<IncidentStore>) -> Self {
        Self {
            store,
            webhook_configs: Vec::new(),
        }
    }

    /// Create a new incident manager with webhook configurations
    pub fn new_with_webhooks(
        store: Arc<IncidentStore>,
        webhook_configs: Vec<crate::incidents::integrations::WebhookConfig>,
    ) -> Self {
        Self {
            store,
            webhook_configs,
        }
    }

    /// Add webhook configuration
    pub fn add_webhook(&mut self, config: crate::incidents::integrations::WebhookConfig) {
        self.webhook_configs.push(config);
    }

    /// Create a new incident from drift result
    #[allow(clippy::too_many_arguments)]
    pub async fn create_incident(
        &self,
        endpoint: String,
        method: String,
        incident_type: IncidentType,
        severity: IncidentSeverity,
        details: serde_json::Value,
        budget_id: Option<String>,
        workspace_id: Option<String>,
    ) -> DriftIncident {
        self.create_incident_with_samples(
            endpoint,
            method,
            incident_type,
            severity,
            details,
            budget_id,
            workspace_id,
            None, // sync_cycle_id
            None, // contract_diff_id
            None, // before_sample
            None, // after_sample
            None, // fitness_test_results
            None, // affected_consumers
            None, // protocol
        )
        .await
    }

    /// Create a new incident with before/after samples and traceability
    #[allow(clippy::too_many_arguments)]
    pub async fn create_incident_with_samples(
        &self,
        endpoint: String,
        method: String,
        incident_type: IncidentType,
        severity: IncidentSeverity,
        details: serde_json::Value,
        budget_id: Option<String>,
        workspace_id: Option<String>,
        sync_cycle_id: Option<String>,
        contract_diff_id: Option<String>,
        before_sample: Option<serde_json::Value>,
        after_sample: Option<serde_json::Value>,
        fitness_test_results: Option<Vec<crate::contract_drift::fitness::FitnessTestResult>>,
        affected_consumers: Option<crate::contract_drift::consumer_mapping::ConsumerImpact>,
        protocol: Option<crate::protocol_abstraction::Protocol>,
    ) -> DriftIncident {
        let id = Uuid::new_v4().to_string();
        let mut incident =
            DriftIncident::new(id, endpoint, method, incident_type, severity, details);
        incident.budget_id = budget_id;
        incident.workspace_id = workspace_id;
        incident.sync_cycle_id = sync_cycle_id;
        incident.contract_diff_id = contract_diff_id;
        incident.before_sample = before_sample;
        incident.after_sample = after_sample;
        incident.fitness_test_results = fitness_test_results.unwrap_or_default();
        incident.affected_consumers = affected_consumers;
        incident.protocol = protocol;

        self.store.store(incident.clone()).await;

        // Trigger webhook notifications for incident.created event
        self.trigger_webhooks("incident.created", &incident).await;

        incident
    }

    /// Get an incident by ID
    pub async fn get_incident(&self, id: &str) -> Option<DriftIncident> {
        self.store.get(id).await
    }

    /// Update an incident
    pub async fn update_incident(&self, incident: DriftIncident) {
        self.store.update(incident).await;
    }

    /// Acknowledge an incident
    pub async fn acknowledge_incident(&self, id: &str) -> Option<DriftIncident> {
        let mut incident = self.store.get(id).await?;
        incident.acknowledge();
        self.store.update(incident.clone()).await;
        Some(incident)
    }

    /// Resolve an incident
    pub async fn resolve_incident(&self, id: &str) -> Option<DriftIncident> {
        let mut incident = self.store.get(id).await?;
        incident.resolve();
        self.store.update(incident.clone()).await;
        Some(incident)
    }

    /// Close an incident
    pub async fn close_incident(&self, id: &str) -> Option<DriftIncident> {
        let mut incident = self.store.get(id).await?;
        incident.close();
        self.store.update(incident.clone()).await;
        Some(incident)
    }

    /// Link an external ticket to an incident
    pub async fn link_external_ticket(
        &self,
        id: &str,
        ticket_id: String,
        ticket_url: Option<String>,
    ) -> Option<DriftIncident> {
        let mut incident = self.store.get(id).await?;
        incident.link_external_ticket(ticket_id, ticket_url);
        self.store.update(incident.clone()).await;
        Some(incident)
    }

    /// Query incidents
    pub async fn query_incidents(&self, query: IncidentQuery) -> Vec<DriftIncident> {
        self.store.query(query).await
    }

    /// Get all open incidents
    pub async fn get_open_incidents(&self) -> Vec<DriftIncident> {
        self.store.get_by_status(IncidentStatus::Open).await
    }

    /// Get incident statistics
    pub async fn get_statistics(&self) -> HashMap<IncidentStatus, usize> {
        self.store.count_by_status().await
    }

    /// Clean up old resolved incidents
    pub async fn cleanup_old_incidents(&self, retention_days: u32) {
        self.store.cleanup_old_resolved(retention_days).await;
    }

    /// Trigger webhook notifications for an event
    async fn trigger_webhooks(&self, event_type: &str, incident: &DriftIncident) {
        use crate::incidents::integrations::send_webhook;
        use serde_json::json;

        for webhook in &self.webhook_configs {
            if !webhook.enabled {
                continue;
            }

            // Check if webhook is subscribed to this event
            if !webhook.events.is_empty() && !webhook.events.contains(&event_type.to_string()) {
                continue;
            }

            // Determine webhook format based on URL or headers
            let payload = if webhook.url.contains("slack.com")
                || webhook.url.contains("hooks.slack.com")
            {
                // Format as Slack message
                use crate::incidents::slack_formatter::format_slack_webhook;
                format_slack_webhook(incident)
            } else if webhook.url.contains("jira") || webhook.headers.contains_key("X-Jira-Project")
            {
                // Format as Jira webhook
                use crate::incidents::jira_formatter::format_jira_webhook;
                format_jira_webhook(incident)
            } else {
                // Generic webhook format
                json!({
                    "event": event_type,
                    "incident": {
                        "id": incident.id,
                        "endpoint": incident.endpoint,
                        "method": incident.method,
                        "type": format!("{:?}", incident.incident_type),
                        "severity": format!("{:?}", incident.severity),
                        "status": format!("{:?}", incident.status),
                        "details": incident.details,
                        "created_at": incident.created_at,
                    }
                })
            };

            // Send webhook asynchronously (fire and forget)
            let webhook_clone = webhook.clone();
            tokio::spawn(async move {
                if let Err(e) = send_webhook(&webhook_clone, &payload).await {
                    tracing::warn!("Failed to send webhook to {}: {}", webhook_clone.url, e);
                }
            });
        }
    }
}
