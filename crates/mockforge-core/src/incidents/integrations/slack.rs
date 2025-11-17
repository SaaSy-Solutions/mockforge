//! Slack webhook formatter for drift incidents
//!
//! This module provides formatting for Slack webhook messages when drift incidents are created.

use crate::incidents::types::{DriftIncident, IncidentSeverity, IncidentType};
use serde_json::{json, Value};

/// Format a drift incident as a Slack message
pub fn format_slack_message(incident: &DriftIncident) -> Value {
    let color = match incident.severity {
        IncidentSeverity::Critical => "#FF0000", // Red
        IncidentSeverity::High => "#FF8C00",     // Orange
        IncidentSeverity::Medium => "#FFD700",   // Gold
        IncidentSeverity::Low => "#32CD32",      // Green
    };

    let emoji = match incident.incident_type {
        IncidentType::BreakingChange => "🚨",
        IncidentType::ThresholdExceeded => "⚠️",
    };

    let title = format!(
        "{} Drift Incident: {} {}",
        emoji,
        incident.method,
        incident.endpoint
    );

    let mut fields = vec![
        json!({
            "title": "Type",
            "value": format!("{:?}", incident.incident_type),
            "short": true
        }),
        json!({
            "title": "Severity",
            "value": format!("{:?}", incident.severity),
            "short": true
        }),
        json!({
            "title": "Status",
            "value": format!("{:?}", incident.status),
            "short": true
        }),
    ];

    // Add details from incident.details if available
    if let Some(breaking_changes) = incident.details.get("breaking_changes") {
        fields.push(json!({
            "title": "Breaking Changes",
            "value": breaking_changes,
            "short": true
        }));
    }

    if let Some(non_breaking_changes) = incident.details.get("non_breaking_changes") {
        fields.push(json!({
            "title": "Non-Breaking Changes",
            "value": non_breaking_changes,
            "short": true
        }));
    }

    if let Some(budget_exceeded) = incident.details.get("budget_exceeded") {
        fields.push(json!({
            "title": "Budget Exceeded",
            "value": if budget_exceeded.as_bool().unwrap_or(false) { "Yes" } else { "No" },
            "short": true
        }));
    }

    // Add workspace ID if available
    if let Some(workspace_id) = &incident.workspace_id {
        fields.push(json!({
            "title": "Workspace",
            "value": workspace_id,
            "short": true
        }));
    }

    // Add sync cycle ID or contract diff ID if available
    if let Some(sync_cycle_id) = &incident.sync_cycle_id {
        fields.push(json!({
            "title": "Sync Cycle",
            "value": sync_cycle_id,
            "short": true
        }));
    }

    if let Some(contract_diff_id) = &incident.contract_diff_id {
        fields.push(json!({
            "title": "Contract Diff",
            "value": contract_diff_id,
            "short": true
        }));
    }

    // Build the Slack message with blocks
    json!({
        "blocks": [
            {
                "type": "header",
                "text": {
                    "type": "plain_text",
                    "text": title
                }
            },
            {
                "type": "section",
                "fields": fields
            },
            {
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": format!("*Incident ID:* `{}`\n*Detected:* <!date^{}|{{date_pretty}} {{time}}|{}>", 
                        incident.id,
                        incident.detected_at,
                        chrono::DateTime::from_timestamp(incident.detected_at, 0)
                            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                            .unwrap_or_else(|| "Unknown".to_string())
                    )
                }
            }
        ],
        "attachments": [
            {
                "color": color,
                "blocks": [
                    {
                        "type": "section",
                        "text": {
                            "type": "mrkdwn",
                            "text": format!("*Details:*\n```{}```", 
                                serde_json::to_string_pretty(&incident.details).unwrap_or_else(|_| "N/A".to_string())
                            )
                        }
                    }
                ]
            }
        ]
    })
}

/// Format a Slack webhook payload
pub fn format_slack_webhook(incident: &DriftIncident) -> Value {
    json!({
        "text": format!("Drift Incident: {} {}", incident.method, incident.endpoint),
        "blocks": format_slack_message(incident).get("blocks").cloned().unwrap_or_else(|| json!([]))
    })
}

