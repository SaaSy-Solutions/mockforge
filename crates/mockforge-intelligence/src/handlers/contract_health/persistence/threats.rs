//! Threat assessment selection and best-effort decoding of existing rows.

use super::super::TimelineEvent;
use chrono::{DateTime, Utc};
use mockforge_foundation::threat_modeling_types::ThreatLevel;
use sqlx::{postgres::PgRow, Row};

pub(super) async fn events(
    pool: &sqlx::PgPool,
    workspace: Option<uuid::Uuid>,
) -> Vec<TimelineEvent> {
    sqlx::query(
        "SELECT id, workspace_id, service_id, service_name, endpoint, method, aggregation_level,
         threat_level, threat_score, threat_categories, findings, remediation_suggestions, assessed_at
         FROM contract_threat_assessments
         WHERE workspace_id = $1 OR workspace_id IS NULL
         ORDER BY assessed_at DESC LIMIT 50",
    )
    .bind(workspace)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .filter_map(event)
    .collect()
}

// Invalid or incomplete stored records remain omitted, matching the existing
// timeline's best-effort behavior. These typed reads prevent default values
// from disguising a schema mismatch.
fn event(row: PgRow) -> Option<TimelineEvent> {
    let id: uuid::Uuid = row.try_get("id").ok()?;
    let level: String = row.try_get("threat_level").ok()?;
    let threat_score: f64 = row.try_get("threat_score").ok()?;
    let assessed_at: DateTime<Utc> = row.try_get("assessed_at").ok()?;
    let threat_level = match level.as_str() {
        "low" => ThreatLevel::Low,
        "medium" => ThreatLevel::Medium,
        "high" => ThreatLevel::High,
        "critical" => ThreatLevel::Critical,
        _ => return None,
    };
    let findings_count = row
        .try_get::<serde_json::Value, _>("findings")
        .ok()
        .and_then(|v| v.as_array().map(Vec::len))
        .unwrap_or(0);
    Some(TimelineEvent::ThreatAssessment {
        id: id.to_string(),
        endpoint: row.try_get("endpoint").ok(),
        method: row.try_get("method").ok(),
        threat_level: format!("{:?}", threat_level),
        threat_score,
        assessed_at: assessed_at.timestamp(),
        findings_count,
    })
}
