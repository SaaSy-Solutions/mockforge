//! Jira webhook formatter for drift incidents
//!
//! This module provides formatting for Jira issue creation when drift incidents are detected.

use crate::incidents::types::{DriftIncident, IncidentSeverity, IncidentType};
use serde_json::{json, Value};

/// Format a drift incident as a Jira issue creation payload
pub fn format_jira_issue(incident: &DriftIncident, project_key: &str, issue_type: &str) -> Value {
    let priority = match incident.severity {
        IncidentSeverity::Critical => "Highest",
        IncidentSeverity::High => "High",
        IncidentSeverity::Medium => "Medium",
        IncidentSeverity::Low => "Low",
    };

    let summary = format!(
        "Drift Incident: {} {} - {:?}",
        incident.method, incident.endpoint, incident.incident_type
    );

    // Build description with incident details
    let mut description = format!(
        "Drift incident detected on endpoint {{code}}{} {}{{code}}\n\n",
        incident.method, incident.endpoint
    );

    description.push_str(&format!("*Type:* {}\n", format!("{:?}", incident.incident_type)));
    description.push_str(&format!("*Severity:* {}\n", format!("{:?}", incident.severity)));
    description.push_str(&format!("*Status:* {}\n", format!("{:?}", incident.status)));

    if let Some(breaking_changes) = incident.details.get("breaking_changes") {
        description.push_str(&format!("*Breaking Changes:* {}\n", breaking_changes));
    }

    if let Some(non_breaking_changes) = incident.details.get("non_breaking_changes") {
        description.push_str(&format!("*Non-Breaking Changes:* {}\n", non_breaking_changes));
    }

    if let Some(budget_exceeded) = incident.details.get("budget_exceeded") {
        description.push_str(&format!(
            "*Budget Exceeded:* {}\n",
            if budget_exceeded.as_bool().unwrap_or(false) {
                "Yes"
            } else {
                "No"
            }
        ));
    }

    if let Some(workspace_id) = &incident.workspace_id {
        description.push_str(&format!("*Workspace:* {}\n", workspace_id));
    }

    if let Some(sync_cycle_id) = &incident.sync_cycle_id {
        description.push_str(&format!("*Sync Cycle:* {}\n", sync_cycle_id));
    }

    if let Some(contract_diff_id) = &incident.contract_diff_id {
        description.push_str(&format!("*Contract Diff ID:* {}\n", contract_diff_id));
    }

    description.push_str("\n*Incident Details:*\n");
    description.push_str(&format!(
        "{{code:json}}\n{}\n{{code}}",
        serde_json::to_string_pretty(&incident.details).unwrap_or_else(|_| "N/A".to_string())
    ));

    // Add before/after samples if available
    if let Some(before_sample) = &incident.before_sample {
        description.push_str("\n\n*Before Sample:*\n");
        description.push_str(&format!(
            "{{code:json}}\n{}\n{{code}}",
            serde_json::to_string_pretty(before_sample).unwrap_or_else(|_| "N/A".to_string())
        ));
    }

    if let Some(after_sample) = &incident.after_sample {
        description.push_str("\n\n*After Sample:*\n");
        description.push_str(&format!(
            "{{code:json}}\n{}\n{{code}}",
            serde_json::to_string_pretty(after_sample).unwrap_or_else(|_| "N/A".to_string())
        ));
    }

    json!({
        "fields": {
            "project": {
                "key": project_key
            },
            "summary": summary,
            "description": description,
            "issuetype": {
                "name": issue_type
            },
            "priority": {
                "name": priority
            },
            "labels": [
                "drift-incident",
                format!("severity-{:?}", incident.severity).to_lowercase(),
                format!("type-{:?}", incident.incident_type).to_lowercase(),
            ]
        }
    })
}

/// Format a Jira webhook payload (for incoming webhooks)
pub fn format_jira_webhook(incident: &DriftIncident) -> Value {
    json!({
        "summary": format!("Drift Incident: {} {}", incident.method, incident.endpoint),
        "description": format!(
            "Drift incident detected:\n- Type: {:?}\n- Severity: {:?}\n- Endpoint: {} {}\n- Incident ID: {}",
            incident.incident_type,
            incident.severity,
            incident.method,
            incident.endpoint,
            incident.id
        ),
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
}

