//! Unified contract health timeline
//!
//! This module provides a unified endpoint that combines:
//! - Structural drift incidents
//! - Semantic drift incidents
//! - Threat assessments
//! - Forecast predictions

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// State for contract health handlers
#[derive(Clone)]
pub struct ContractHealthState {
    /// Incident manager for structural incidents
    pub incident_manager: Arc<mockforge_core::incidents::IncidentManager>,
    /// Semantic incident manager
    pub semantic_manager: Arc<mockforge_core::incidents::SemanticIncidentManager>,
    /// Database connection (optional)
    pub database: Option<crate::database::Database>,
}

/// Query parameters for timeline
#[derive(Debug, Deserialize)]
pub struct TimelineQuery {
    /// Workspace ID filter
    pub workspace_id: Option<String>,
    /// Endpoint filter
    pub endpoint: Option<String>,
    /// Method filter
    pub method: Option<String>,
    /// Start date (ISO 8601)
    pub start_date: Option<String>,
    /// End date (ISO 8601)
    pub end_date: Option<String>,
    /// Filter by type: "structural", "semantic", "threat", "forecast", or "all"
    pub event_type: Option<String>,
    /// Limit results
    pub limit: Option<usize>,
}

/// Timeline event
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TimelineEvent {
    /// Structural drift incident
    #[serde(rename = "structural_drift")]
    StructuralDrift {
        /// Incident ID
        id: String,
        /// Endpoint path
        endpoint: String,
        /// HTTP method
        method: String,
        /// Type of incident
        incident_type: String,
        /// Severity level
        severity: String,
        /// Current status
        status: String,
        /// Detection timestamp
        detected_at: i64,
        /// Additional details
        details: serde_json::Value,
    },
    /// Semantic drift incident
    #[serde(rename = "semantic_drift")]
    SemanticDrift {
        /// Incident ID
        id: String,
        /// Endpoint path
        endpoint: String,
        /// HTTP method
        method: String,
        /// Type of semantic change
        change_type: String,
        /// Severity level
        severity: String,
        /// Current status
        status: String,
        /// Semantic confidence score
        semantic_confidence: f64,
        /// Soft-breaking score
        soft_breaking_score: f64,
        /// Detection timestamp
        detected_at: i64,
        /// Additional details
        details: serde_json::Value,
    },
    /// Threat assessment
    #[serde(rename = "threat_assessment")]
    ThreatAssessment {
        /// Assessment ID
        id: String,
        /// Endpoint path (optional for workspace-level)
        endpoint: Option<String>,
        /// HTTP method (optional for workspace-level)
        method: Option<String>,
        /// Threat level
        threat_level: String,
        /// Threat score (0.0-1.0)
        threat_score: f64,
        /// Assessment timestamp
        assessed_at: i64,
        /// Number of findings
        findings_count: usize,
    },
    /// Forecast prediction
    #[serde(rename = "forecast")]
    Forecast {
        /// Forecast ID
        id: String,
        /// Endpoint path
        endpoint: String,
        /// HTTP method
        method: String,
        /// Forecast window in days
        window_days: u32,
        /// Change probability (0.0-1.0)
        change_probability: f64,
        /// Break probability (0.0-1.0)
        break_probability: f64,
        /// Next expected change timestamp (optional)
        next_expected_change: Option<i64>,
        /// Confidence score (0.0-1.0)
        confidence: f64,
        /// Prediction timestamp
        predicted_at: i64,
    },
}

/// Timeline response
#[derive(Debug, Serialize)]
pub struct TimelineResponse {
    /// Timeline events
    pub events: Vec<TimelineEvent>,
    /// Total count
    pub total: usize,
}

/// Get unified contract health timeline
///
/// GET /api/v1/contract-health/timeline
pub async fn get_timeline(
    State(state): State<ContractHealthState>,
    Query(params): Query<TimelineQuery>,
) -> Result<Json<TimelineResponse>, StatusCode> {
    let mut events = Vec::new();

    let event_type_filter = params.event_type.as_deref().unwrap_or("all");

    // Get structural drift incidents
    if event_type_filter == "all" || event_type_filter == "structural" {
        let mut query = mockforge_core::incidents::types::IncidentQuery::default();
        query.workspace_id = params.workspace_id.clone();
        query.endpoint = params.endpoint.clone();
        query.method = params.method.clone();

        let incidents = state.incident_manager.query_incidents(query).await;
        for incident in incidents {
            events.push(TimelineEvent::StructuralDrift {
                id: incident.id,
                endpoint: incident.endpoint,
                method: incident.method,
                incident_type: format!("{:?}", incident.incident_type),
                severity: format!("{:?}", incident.severity),
                status: format!("{:?}", incident.status),
                detected_at: incident.detected_at,
                details: incident.details,
            });
        }
    }

    // Get semantic drift incidents
    if event_type_filter == "all" || event_type_filter == "semantic" {
        let status = None; // Get all statuses for timeline
        let semantic_incidents = state
            .semantic_manager
            .list_incidents(
                params.workspace_id.as_deref(),
                params.endpoint.as_deref(),
                params.method.as_deref(),
                status,
                params.limit,
            )
            .await;

        for incident in semantic_incidents {
            events.push(TimelineEvent::SemanticDrift {
                id: incident.id,
                endpoint: incident.endpoint,
                method: incident.method,
                change_type: format!("{:?}", incident.semantic_change_type),
                severity: format!("{:?}", incident.severity),
                status: format!("{:?}", incident.status),
                semantic_confidence: incident.semantic_confidence,
                soft_breaking_score: incident.soft_breaking_score,
                detected_at: incident.detected_at,
                details: incident.details,
            });
        }
    }

    // Add threat assessments and forecasts from database
    #[cfg(feature = "database")]
    {
        use sqlx::Row;
        if let Some(pool) = state.database.as_ref().and_then(|db| db.pool()) {
            // Query threat assessments
            if let Ok(ta_rows) = sqlx::query(
            "SELECT id, workspace_id, service_id, service_name, endpoint, method, aggregation_level,
             threat_level, threat_score, threat_categories, findings, remediation_suggestions, assessed_at
             FROM contract_threat_assessments
             WHERE workspace_id = $1 OR workspace_id IS NULL
             ORDER BY assessed_at DESC LIMIT 50"
        )
        .bind(params.workspace_id.as_deref())
        .fetch_all(pool)
        .await
        {
            use mockforge_core::contract_drift::threat_modeling::{ThreatLevel, AggregationLevel};
            for row in ta_rows {
                let id: uuid::Uuid = match row.try_get("id") {
                    Ok(id) => id,
                    Err(_) => continue,
                };
                let threat_level_str: String = match row.try_get("threat_level") {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let threat_score: f64 = match row.try_get("threat_score") {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let assessed_at: DateTime<Utc> = match row.try_get("assessed_at") {
                    Ok(dt) => dt,
                    Err(_) => continue,
                };
                let endpoint: Option<String> = row.try_get("endpoint").ok();
                let method: Option<String> = row.try_get("method").ok();

                let threat_level = match threat_level_str.as_str() {
                    "low" => ThreatLevel::Low,
                    "medium" => ThreatLevel::Medium,
                    "high" => ThreatLevel::High,
                    "critical" => ThreatLevel::Critical,
                    _ => continue,
                };

                // Count findings from the findings JSON field
                let findings_count = row.try_get::<serde_json::Value, _>("findings")
                    .ok()
                    .and_then(|v| v.as_array().map(|arr| arr.len()))
                    .unwrap_or(0);

                events.push(TimelineEvent::ThreatAssessment {
                    id: id.to_string(),
                    endpoint,
                    method,
                    threat_level: format!("{:?}", threat_level),
                    threat_score,
                    assessed_at: assessed_at.timestamp(),
                    findings_count,
                });
            }
        }

            // Query forecasts
            if let Ok(forecast_rows) = sqlx::query(
                "SELECT id, service_id, service_name, endpoint, method, forecast_window_days,
             predicted_change_probability, predicted_break_probability, next_expected_change_date,
             confidence, predicted_at
             FROM api_change_forecasts
             WHERE workspace_id = $1 OR workspace_id IS NULL
             ORDER BY predicted_at DESC LIMIT 50",
            )
            .bind(params.workspace_id.as_deref())
            .fetch_all(pool)
            .await
            {
                use sqlx::Row;
                for row in forecast_rows {
                    let id: uuid::Uuid = match row.try_get("id") {
                        Ok(id) => id,
                        Err(_) => continue,
                    };
                    let endpoint: String = match row.try_get("endpoint") {
                        Ok(e) => e,
                        Err(_) => continue,
                    };
                    let method: String = match row.try_get("method") {
                        Ok(m) => m,
                        Err(_) => continue,
                    };
                    let forecast_window_days: i32 = match row.try_get("forecast_window_days") {
                        Ok(d) => d,
                        Err(_) => continue,
                    };
                    let predicted_change_probability: f64 =
                        match row.try_get("predicted_change_probability") {
                            Ok(p) => p,
                            Err(_) => continue,
                        };
                    let predicted_break_probability: f64 =
                        match row.try_get("predicted_break_probability") {
                            Ok(p) => p,
                            Err(_) => continue,
                        };
                    let next_expected_change_date: Option<DateTime<Utc>> =
                        row.try_get("next_expected_change_date").ok();
                    let predicted_at: DateTime<Utc> = match row.try_get("predicted_at") {
                        Ok(dt) => dt,
                        Err(_) => continue,
                    };
                    let confidence: f64 = match row.try_get("confidence") {
                        Ok(c) => c,
                        Err(_) => continue,
                    };

                    events.push(TimelineEvent::Forecast {
                        id: id.to_string(),
                        endpoint,
                        method,
                        window_days: forecast_window_days as u32,
                        change_probability: predicted_change_probability,
                        break_probability: predicted_break_probability,
                        next_expected_change: next_expected_change_date.map(|d| d.timestamp()),
                        confidence,
                        predicted_at: predicted_at.timestamp(),
                    });
                }
            }
        }
    }

    // Sort by timestamp (most recent first)
    events.sort_by_key(|e| {
        std::cmp::Reverse(match e {
            TimelineEvent::StructuralDrift { detected_at, .. } => *detected_at,
            TimelineEvent::SemanticDrift { detected_at, .. } => *detected_at,
            TimelineEvent::ThreatAssessment { assessed_at, .. } => *assessed_at,
            TimelineEvent::Forecast { predicted_at, .. } => *predicted_at,
        })
    });

    // Apply limit
    let total = events.len();
    if let Some(limit) = params.limit {
        events.truncate(limit);
    }

    Ok(Json(TimelineResponse { events, total }))
}

/// Create router for contract health endpoints
pub fn contract_health_router(state: ContractHealthState) -> axum::Router {
    use axum::routing::get;
    use axum::Router;

    Router::new()
        .route("/api/v1/contract-health/timeline", get(get_timeline))
        .with_state(state)
}
