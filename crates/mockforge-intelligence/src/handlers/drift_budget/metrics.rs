//! Incident-derived metrics and lookback selection.

use super::DriftBudgetStateProvider;
use crate::handlers::structural_incidents::StructuralIncidents;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use mockforge_foundation::incidents_types::{IncidentQuery, IncidentType};
use serde::Deserialize;
use std::collections::HashMap;

/// Get drift metrics over time
///
/// GET /api/v1/drift/metrics?endpoint=/api/users&method=GET&days=30
#[derive(Debug, Deserialize)]
pub struct GetMetricsQuery {
    /// Optional endpoint filter
    pub endpoint: Option<String>,
    /// Optional HTTP method filter
    pub method: Option<String>,
    /// Optional workspace ID filter
    pub workspace_id: Option<String>,
    /// Lookback window in days
    pub days: Option<u32>,
}

/// Get drift metrics
///
/// GET /api/v1/drift/metrics
pub async fn get_drift_metrics<S: DriftBudgetStateProvider>(
    State(state): State<S>,
    Query(params): Query<GetMetricsQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Query incidents for metrics
    let mut query = IncidentQuery {
        endpoint: params.endpoint,
        method: params.method,
        workspace_id: params.workspace_id,
        ..IncidentQuery::default()
    };

    // Filter by date range if days specified
    if let Some(days) = params.days {
        let start_date = chrono::Utc::now()
            .checked_sub_signed(chrono::Duration::days(days as i64))
            .map(|dt| dt.timestamp())
            .unwrap_or(0);
        query.start_date = Some(start_date);
    }

    let incidents = state.incident_manager().query_incidents(query).await;

    // Calculate metrics
    let total_incidents = incidents.len();
    let breaking_changes = incidents
        .iter()
        .filter(|i| matches!(i.incident_type, IncidentType::BreakingChange))
        .count();
    let threshold_exceeded = total_incidents - breaking_changes;

    let by_severity: HashMap<String, usize> =
        incidents.iter().fold(HashMap::new(), |mut acc, inc| {
            let key = format!("{:?}", inc.severity).to_lowercase();
            *acc.entry(key).or_insert(0) += 1;
            acc
        });

    Ok(Json(serde_json::json!({
        "total_incidents": total_incidents,
        "breaking_changes": breaking_changes,
        "threshold_exceeded": threshold_exceeded,
        "by_severity": by_severity,
        "incidents": incidents.iter().take(100).collect::<Vec<_>>(), // Limit to first 100
    })))
}
