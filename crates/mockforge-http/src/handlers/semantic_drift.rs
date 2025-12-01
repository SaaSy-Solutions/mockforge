//! HTTP handlers for semantic drift incidents
//!
//! This module provides endpoints for managing semantic drift incidents.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::{DateTime, Utc};
use mockforge_core::ai_contract_diff::{ContractDiffAnalyzer, ContractDiffConfig};
use mockforge_core::incidents::semantic_manager::{SemanticIncident, SemanticIncidentManager};
use mockforge_core::openapi::OpenApiSpec;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::database::Database;

/// Helper function to map database row to SemanticIncident
#[cfg(feature = "database")]
fn map_row_to_semantic_incident(
    row: &sqlx::postgres::PgRow,
) -> Result<SemanticIncident, sqlx::Error> {
    use mockforge_core::ai_contract_diff::semantic_analyzer::SemanticChangeType;
    use mockforge_core::incidents::types::{IncidentSeverity, IncidentStatus};
    use sqlx::Row;

    let id: uuid::Uuid = row.try_get("id")?;
    let workspace_id: Option<uuid::Uuid> = row.try_get("workspace_id").ok();
    let endpoint: String = row.try_get("endpoint")?;
    let method: String = row.try_get("method")?;
    let semantic_change_type_str: String = row.try_get("semantic_change_type")?;
    let severity_str: String = row.try_get("severity")?;
    let status_str: String = row.try_get("status")?;
    let semantic_confidence: f64 = row.try_get("semantic_confidence")?;
    let soft_breaking_score: f64 = row.try_get("soft_breaking_score")?;
    let llm_analysis: serde_json::Value = row.try_get("llm_analysis").unwrap_or_default();
    let before_semantic_state: serde_json::Value =
        row.try_get("before_semantic_state").unwrap_or_default();
    let after_semantic_state: serde_json::Value =
        row.try_get("after_semantic_state").unwrap_or_default();
    let details_json: serde_json::Value = row.try_get("details").unwrap_or_default();
    let related_drift_incident_id: Option<uuid::Uuid> =
        row.try_get("related_drift_incident_id").ok();
    let contract_diff_id: Option<String> = row.try_get("contract_diff_id").ok();
    let external_ticket_id: Option<String> = row.try_get("external_ticket_id").ok();
    let external_ticket_url: Option<String> = row.try_get("external_ticket_url").ok();
    let detected_at: DateTime<Utc> = row.try_get("detected_at")?;
    let created_at: DateTime<Utc> = row.try_get("created_at")?;
    let acknowledged_at: Option<DateTime<Utc>> = row.try_get("acknowledged_at").ok();
    let resolved_at: Option<DateTime<Utc>> = row.try_get("resolved_at").ok();
    let closed_at: Option<DateTime<Utc>> = row.try_get("closed_at").ok();
    let updated_at: DateTime<Utc> = row.try_get("updated_at")?;

    // Parse semantic change type
    let semantic_change_type = match semantic_change_type_str.as_str() {
        "description_change" => SemanticChangeType::DescriptionChange,
        "enum_narrowing" => SemanticChangeType::EnumNarrowing,
        "nullability_change" => SemanticChangeType::NullabilityChange,
        "error_code_removed" => SemanticChangeType::ErrorCodeRemoved,
        "meaning_shift" => SemanticChangeType::MeaningShift,
        _ => SemanticChangeType::MeaningShift, // Default fallback
    };

    // Parse severity
    let severity = match severity_str.as_str() {
        "low" => IncidentSeverity::Low,
        "medium" => IncidentSeverity::Medium,
        "high" => IncidentSeverity::High,
        "critical" => IncidentSeverity::Critical,
        _ => IncidentSeverity::Medium, // Default fallback
    };

    // Parse status
    let status = match status_str.as_str() {
        "open" => IncidentStatus::Open,
        "acknowledged" => IncidentStatus::Acknowledged,
        "resolved" => IncidentStatus::Resolved,
        "closed" => IncidentStatus::Closed,
        _ => IncidentStatus::Open, // Default fallback
    };

    Ok(SemanticIncident {
        id: id.to_string(),
        workspace_id: workspace_id.map(|u| u.to_string()),
        endpoint,
        method,
        semantic_change_type,
        severity,
        status,
        semantic_confidence,
        soft_breaking_score,
        llm_analysis,
        before_semantic_state,
        after_semantic_state,
        details: details_json,
        related_drift_incident_id: related_drift_incident_id.map(|u| u.to_string()),
        contract_diff_id,
        external_ticket_id,
        external_ticket_url,
        detected_at: detected_at.timestamp(),
        created_at: created_at.timestamp(),
        acknowledged_at: acknowledged_at.map(|dt| dt.timestamp()),
        resolved_at: resolved_at.map(|dt| dt.timestamp()),
        closed_at: closed_at.map(|dt| dt.timestamp()),
        updated_at: updated_at.timestamp(),
    })
}

/// State for semantic drift handlers
#[derive(Clone)]
pub struct SemanticDriftState {
    /// Semantic incident manager
    pub manager: Arc<SemanticIncidentManager>,
    /// Database connection (optional)
    pub database: Option<Database>,
}

/// Query parameters for listing semantic incidents
#[derive(Debug, Deserialize)]
pub struct ListSemanticIncidentsQuery {
    /// Workspace ID filter
    pub workspace_id: Option<String>,
    /// Endpoint filter
    pub endpoint: Option<String>,
    /// Method filter
    pub method: Option<String>,
    /// Status filter
    pub status: Option<String>,
    /// Limit results
    pub limit: Option<usize>,
}

/// Response for semantic incident list
#[derive(Debug, Serialize)]
pub struct SemanticIncidentListResponse {
    /// Incidents
    pub incidents: Vec<SemanticIncident>,
    /// Total count
    pub total: usize,
}

/// List semantic drift incidents
///
/// GET /api/v1/semantic-drift/incidents
pub async fn list_semantic_incidents(
    State(state): State<SemanticDriftState>,
    Query(params): Query<ListSemanticIncidentsQuery>,
) -> Result<Json<SemanticIncidentListResponse>, StatusCode> {
    // Try database first, fallback to in-memory
    #[cfg(feature = "database")]
    if let Some(pool) = state.database.as_ref().and_then(|db| db.pool()) {
        let mut query = String::from(
            "SELECT id, workspace_id, endpoint, method, semantic_change_type, severity, status,
             semantic_confidence, soft_breaking_score, llm_analysis, before_semantic_state,
             after_semantic_state, details, related_drift_incident_id, contract_diff_id,
             external_ticket_id, external_ticket_url, detected_at, created_at, acknowledged_at,
             resolved_at, closed_at, updated_at
             FROM semantic_drift_incidents WHERE 1=1",
        );

        let mut bind_index = 1;

        if let Some(ws_id) = &params.workspace_id {
            query.push_str(&format!(" AND workspace_id = ${}", bind_index));
            bind_index += 1;
        }

        if let Some(ep) = &params.endpoint {
            query.push_str(&format!(" AND endpoint = ${}", bind_index));
            bind_index += 1;
        }

        if let Some(m) = &params.method {
            query.push_str(&format!(" AND method = ${}", bind_index));
            bind_index += 1;
        }

        if let Some(status_str) = &params.status {
            query.push_str(&format!(" AND status = ${}", bind_index));
            bind_index += 1;
        }

        let limit = params.limit.unwrap_or(100);
        query.push_str(&format!(" ORDER BY detected_at DESC LIMIT {}", limit));

        // Execute query - use fetch_all for SELECT queries
        let rows = sqlx::query(&query).fetch_all(pool).await.map_err(|e| {
            tracing::error!("Failed to query semantic incidents: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        // Map rows to SemanticIncident
        let mut incidents = Vec::new();
        for row in rows {
            match map_row_to_semantic_incident(&row) {
                Ok(incident) => incidents.push(incident),
                Err(e) => {
                    tracing::warn!("Failed to map semantic incident row: {}", e);
                    continue;
                }
            }
        }
        if !incidents.is_empty() {
            return Ok(Json(SemanticIncidentListResponse {
                total: incidents.len(),
                incidents,
            }));
        }
        // Fall through to in-memory manager if no database results
    }

    // Fallback to in-memory manager
    let status = params.status.as_deref().and_then(|s| match s {
        "open" => Some(mockforge_core::incidents::types::IncidentStatus::Open),
        "acknowledged" => Some(mockforge_core::incidents::types::IncidentStatus::Acknowledged),
        "resolved" => Some(mockforge_core::incidents::types::IncidentStatus::Resolved),
        "closed" => Some(mockforge_core::incidents::types::IncidentStatus::Closed),
        _ => None,
    });

    let incidents = state
        .manager
        .list_incidents(
            params.workspace_id.as_deref(),
            params.endpoint.as_deref(),
            params.method.as_deref(),
            status,
            params.limit,
        )
        .await;

    Ok(Json(SemanticIncidentListResponse {
        total: incidents.len(),
        incidents,
    }))
}

/// Get a specific semantic incident
///
/// GET /api/v1/semantic-drift/incidents/{id}
pub async fn get_semantic_incident(
    State(state): State<SemanticDriftState>,
    Path(id): Path<String>,
) -> Result<Json<SemanticIncident>, StatusCode> {
    // Try database first
    #[cfg(feature = "database")]
    if let Some(pool) = state.database.as_ref().and_then(|db| db.pool()) {
        let row = sqlx::query("SELECT * FROM semantic_drift_incidents WHERE id = $1")
            .bind(&id)
            .fetch_optional(pool)
            .await
            .map_err(|e| {
                tracing::error!("Failed to query semantic incident: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        if let Some(row) = row {
            match map_row_to_semantic_incident(&row) {
                Ok(incident) => return Ok(Json(incident)),
                Err(e) => {
                    tracing::warn!("Failed to map semantic incident: {}", e);
                    // Fall through to in-memory
                }
            }
        }
    }

    // Fallback to in-memory manager
    match state.manager.get_incident(&id).await {
        Some(incident) => Ok(Json(incident)),
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Request to analyze semantic drift
#[derive(Debug, Deserialize)]
pub struct AnalyzeSemanticDriftRequest {
    /// Before spec (OpenAPI YAML/JSON)
    pub before_spec: String,
    /// After spec (OpenAPI YAML/JSON)
    pub after_spec: String,
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Workspace ID (optional)
    pub workspace_id: Option<String>,
}

/// Analyze semantic drift between two specs
///
/// POST /api/v1/semantic-drift/analyze
pub async fn analyze_semantic_drift(
    State(state): State<SemanticDriftState>,
    Json(request): Json<AnalyzeSemanticDriftRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Parse specs
    let before_spec = OpenApiSpec::from_string(&request.before_spec, None)
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let after_spec =
        OpenApiSpec::from_string(&request.after_spec, None).map_err(|_| StatusCode::BAD_REQUEST)?;

    // Create analyzer
    let config = ContractDiffConfig::default();
    let analyzer =
        ContractDiffAnalyzer::new(config).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Run semantic analysis
    let semantic_result = analyzer
        .compare_specs(&before_spec, &after_spec, &request.endpoint, &request.method)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(result) = semantic_result {
        // Create semantic incident if threshold met
        if result.semantic_confidence >= 0.65 {
            let incident = state
                .manager
                .create_incident(
                    &result,
                    request.endpoint.clone(),
                    request.method.clone(),
                    request.workspace_id.clone(),
                    None, // related_drift_incident_id
                    None, // contract_diff_id
                )
                .await;

            // Store in database if available
            #[cfg(feature = "database")]
            if let Some(pool) = state.database.as_ref().and_then(|db| db.pool()) {
                if let Err(e) = store_semantic_incident(pool, &incident).await {
                    tracing::warn!("Failed to store semantic incident in database: {}", e);
                }
            }

            return Ok(Json(serde_json::json!({
                "success": true,
                "semantic_drift_detected": true,
                "incident_id": incident.id,
                "semantic_confidence": result.semantic_confidence,
                "soft_breaking_score": result.soft_breaking_score,
                "change_type": format!("{:?}", result.change_type),
            })));
        }
    }

    Ok(Json(serde_json::json!({
        "success": true,
        "semantic_drift_detected": false,
        "message": "No significant semantic drift detected"
    })))
}

/// Store semantic incident in database
#[cfg(feature = "database")]
async fn store_semantic_incident(
    pool: &sqlx::PgPool,
    incident: &SemanticIncident,
) -> Result<(), sqlx::Error> {
    let id = Uuid::parse_str(&incident.id).unwrap_or_else(|_| Uuid::new_v4());
    let workspace_uuid = incident.workspace_id.as_ref().and_then(|id| Uuid::parse_str(id).ok());
    let related_uuid = incident
        .related_drift_incident_id
        .as_ref()
        .and_then(|id| Uuid::parse_str(id).ok());

    sqlx::query(
        r#"
        INSERT INTO semantic_drift_incidents (
            id, workspace_id, endpoint, method, semantic_change_type, severity, status,
            semantic_confidence, soft_breaking_score, llm_analysis, before_semantic_state,
            after_semantic_state, details, related_drift_incident_id, contract_diff_id,
            external_ticket_id, external_ticket_url, detected_at, created_at, updated_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20
        )
        ON CONFLICT (id) DO UPDATE SET
            status = EXCLUDED.status,
            acknowledged_at = EXCLUDED.acknowledged_at,
            resolved_at = EXCLUDED.resolved_at,
            closed_at = EXCLUDED.closed_at,
            updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(id)
    .bind(workspace_uuid)
    .bind(&incident.endpoint)
    .bind(&incident.method)
    .bind(format!("{:?}", incident.semantic_change_type))
    .bind(format!("{:?}", incident.severity))
    .bind(format!("{:?}", incident.status))
    .bind(incident.semantic_confidence)
    .bind(incident.soft_breaking_score)
    .bind(&incident.llm_analysis)
    .bind(&incident.before_semantic_state)
    .bind(&incident.after_semantic_state)
    .bind(&incident.details)
    .bind(related_uuid)
    .bind(incident.contract_diff_id.as_deref())
    .bind(incident.external_ticket_id.as_deref())
    .bind(incident.external_ticket_url.as_deref())
    .bind(DateTime::<Utc>::from_timestamp(incident.detected_at, 0).unwrap_or_else(Utc::now))
    .bind(DateTime::<Utc>::from_timestamp(incident.created_at, 0).unwrap_or_else(Utc::now))
    .bind(DateTime::<Utc>::from_timestamp(incident.updated_at, 0).unwrap_or_else(Utc::now))
    .execute(pool)
    .await?;

    Ok(())
}

/// Create router for semantic drift endpoints
pub fn semantic_drift_router(state: SemanticDriftState) -> axum::Router {
    use axum::routing::{get, post};
    use axum::Router;

    Router::new()
        .route("/api/v1/semantic-drift/incidents", get(list_semantic_incidents))
        .route("/api/v1/semantic-drift/incidents/{id}", get(get_semantic_incident))
        .route("/api/v1/semantic-drift/analyze", post(analyze_semantic_drift))
        .with_state(state)
}
