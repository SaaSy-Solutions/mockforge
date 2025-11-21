//! HTTP handlers for contract threat modeling
//!
//! This module provides endpoints for security threat assessments.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use chrono::{DateTime, Utc};
use mockforge_core::contract_drift::threat_modeling::{
    ThreatAssessment, ThreatAnalyzer, ThreatModelingConfig,
};
use mockforge_core::openapi::OpenApiSpec;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::database::Database;

/// Helper function to map database row to ThreatAssessment
#[cfg(feature = "database")]
fn map_row_to_threat_assessment(row: &sqlx::postgres::PgRow) -> Result<ThreatAssessment, sqlx::Error> {
    use mockforge_core::contract_drift::threat_modeling::{
        AggregationLevel, ThreatCategory, ThreatFinding, ThreatLevel, RemediationSuggestion,
    };

    // Parse basic fields
    let workspace_id: Option<uuid::Uuid> = row.try_get("workspace_id")?;
    let service_id: Option<String> = row.try_get("service_id")?;
    let service_name: Option<String> = row.try_get("service_name")?;
    let endpoint: Option<String> = row.try_get("endpoint")?;
    let method: Option<String> = row.try_get("method")?;
    let aggregation_level_str: String = row.try_get("aggregation_level")?;
    let threat_level_str: String = row.try_get("threat_level")?;
    let threat_score: f64 = row.try_get("threat_score")?;
    let assessed_at: DateTime<Utc> = row.try_get("assessed_at")?;

    // Parse aggregation level
    let aggregation_level = match aggregation_level_str.as_str() {
        "workspace" => AggregationLevel::Workspace,
        "service" => AggregationLevel::Service,
        "endpoint" => AggregationLevel::Endpoint,
        _ => return Err(sqlx::Error::Decode("Invalid aggregation_level".into())),
    };

    // Parse threat level
    let threat_level = match threat_level_str.as_str() {
        "low" => ThreatLevel::Low,
        "medium" => ThreatLevel::Medium,
        "high" => ThreatLevel::High,
        "critical" => ThreatLevel::Critical,
        _ => return Err(sqlx::Error::Decode("Invalid threat_level".into())),
    };

    // Parse JSONB columns
    let threat_categories_json: serde_json::Value = row.try_get("threat_categories")?;
    let threat_categories: Vec<ThreatCategory> = serde_json::from_value(threat_categories_json)
        .unwrap_or_default();

    let findings_json: serde_json::Value = row.try_get("findings")?;
    let findings: Vec<ThreatFinding> = serde_json::from_value(findings_json)
        .unwrap_or_default();

    let remediations_json: serde_json::Value = row.try_get("remediation_suggestions")?;
    let remediation_suggestions: Vec<RemediationSuggestion> = serde_json::from_value(remediations_json)
        .unwrap_or_default();

    Ok(ThreatAssessment {
        workspace_id: workspace_id.map(|u| u.to_string()),
        service_id,
        service_name,
        endpoint,
        method,
        aggregation_level,
        threat_level,
        threat_score,
        threat_categories,
        findings,
        remediation_suggestions,
        assessed_at,
    })
}

/// State for threat modeling handlers
#[derive(Clone)]
pub struct ThreatModelingState {
    /// Threat analyzer
    pub analyzer: Arc<ThreatAnalyzer>,
    /// Webhook configs for notifications (optional)
    pub webhook_configs: Vec<mockforge_core::incidents::integrations::WebhookConfig>,
    /// Database connection (optional)
    pub database: Option<Database>,
}

/// Get workspace-level threat assessment
///
/// GET /api/v1/threats/workspace/{workspace_id}
#[cfg(feature = "database")]
pub async fn get_workspace_threats(
    State(state): State<ThreatModelingState>,
    Path(workspace_id): Path<String>,
) -> Result<Json<ThreatAssessment>, StatusCode> {
    let pool = match state.database.as_ref().and_then(|db| db.pool()) {
        Some(pool) => pool,
        None => return Err(StatusCode::SERVICE_UNAVAILABLE),
    };

    // Query latest assessment from database
    let row = sqlx::query(
        "SELECT * FROM contract_threat_assessments 
         WHERE workspace_id = $1 AND aggregation_level = 'workspace'
         ORDER BY assessed_at DESC LIMIT 1",
    )
    .bind(&workspace_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query workspace threats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match row {
        Some(row) => {
            match map_row_to_threat_assessment(&row) {
                Ok(assessment) => Ok(Json(assessment)),
                Err(e) => {
                    tracing::error!("Failed to map threat assessment: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Get workspace-level threat assessment (no database)
///
/// GET /api/v1/threats/workspace/{workspace_id}
#[cfg(not(feature = "database"))]
pub async fn get_workspace_threats(
    State(_state): State<ThreatModelingState>,
    Path(_workspace_id): Path<String>,
) -> Result<Json<ThreatAssessment>, StatusCode> {
    Err(StatusCode::SERVICE_UNAVAILABLE)
}

/// Get service-level threat assessment
///
/// GET /api/v1/threats/service/{service_id}
#[cfg(feature = "database")]
pub async fn get_service_threats(
    State(state): State<ThreatModelingState>,
    Path(service_id): Path<String>,
) -> Result<Json<ThreatAssessment>, StatusCode> {
    let pool = match state.database.as_ref().and_then(|db| db.pool()) {
        Some(pool) => pool,
        None => return Err(StatusCode::SERVICE_UNAVAILABLE),
    };

    let row = sqlx::query(
        "SELECT * FROM contract_threat_assessments 
         WHERE service_id = $1 AND aggregation_level = 'service'
         ORDER BY assessed_at DESC LIMIT 1",
    )
    .bind(&service_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query service threats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match row {
        Some(row) => {
            match map_row_to_threat_assessment(&row) {
                Ok(assessment) => Ok(Json(assessment)),
                Err(e) => {
                    tracing::error!("Failed to map threat assessment: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Get service-level threat assessment (no database)
///
/// GET /api/v1/threats/service/{service_id}
#[cfg(not(feature = "database"))]
pub async fn get_service_threats(
    State(_state): State<ThreatModelingState>,
    Path(_service_id): Path<String>,
) -> Result<Json<ThreatAssessment>, StatusCode> {
    Err(StatusCode::SERVICE_UNAVAILABLE)
}

/// Get endpoint-level threat assessment
///
/// GET /api/v1/threats/endpoint/{endpoint}
#[cfg(feature = "database")]
pub async fn get_endpoint_threats(
    State(state): State<ThreatModelingState>,
    Path(endpoint): Path<String>,
    Query(params): Query<serde_json::Value>,
) -> Result<Json<ThreatAssessment>, StatusCode> {
    let pool = match state.database.as_ref().and_then(|db| db.pool()) {
        Some(pool) => pool,
        None => return Err(StatusCode::SERVICE_UNAVAILABLE),
    };

    let method = params
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("%");

    let row = sqlx::query(
        "SELECT * FROM contract_threat_assessments 
         WHERE endpoint = $1 AND method LIKE $2 AND aggregation_level = 'endpoint'
         ORDER BY assessed_at DESC LIMIT 1",
    )
    .bind(&endpoint)
    .bind(method)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query endpoint threats: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match row {
        Some(row) => {
            match map_row_to_threat_assessment(&row) {
                Ok(assessment) => Ok(Json(assessment)),
                Err(e) => {
                    tracing::error!("Failed to map threat assessment: {}", e);
                    Err(StatusCode::INTERNAL_SERVER_ERROR)
                }
            }
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

/// Get endpoint-level threat assessment (no database)
///
/// GET /api/v1/threats/endpoint/{endpoint}
#[cfg(not(feature = "database"))]
pub async fn get_endpoint_threats(
    State(_state): State<ThreatModelingState>,
    Path(_endpoint): Path<String>,
    Query(_params): Query<serde_json::Value>,
) -> Result<Json<ThreatAssessment>, StatusCode> {
    Err(StatusCode::SERVICE_UNAVAILABLE)
}

/// Request to trigger threat assessment
#[derive(Debug, Deserialize)]
pub struct AssessThreatsRequest {
    /// OpenAPI spec (YAML/JSON)
    pub spec: String,
    /// Workspace ID
    pub workspace_id: Option<String>,
    /// Service ID
    pub service_id: Option<String>,
    /// Service name
    pub service_name: Option<String>,
    /// Endpoint (optional)
    pub endpoint: Option<String>,
    /// Method (optional)
    pub method: Option<String>,
}

/// Trigger threat assessment
///
/// POST /api/v1/threats/assess
pub async fn assess_threats(
    State(state): State<ThreatModelingState>,
    Json(request): Json<AssessThreatsRequest>,
) -> Result<Json<ThreatAssessment>, StatusCode> {
    // Parse OpenAPI spec
    let spec = match OpenApiSpec::from_string(&request.spec, None) {
        Ok(spec) => spec,
        Err(_) => return Err(StatusCode::BAD_REQUEST),
    };

    // Run threat analysis
    match state
        .analyzer
        .analyze_contract(
            &spec,
            request.workspace_id.clone(),
            request.service_id.clone(),
            request.service_name.clone(),
            request.endpoint.clone(),
            request.method.clone(),
        )
        .await
    {
        Ok(assessment) => {
            // Store assessment in database
            #[cfg(feature = "database")]
            if let Some(pool) = state.database.as_ref().and_then(|db| db.pool()) {
                if let Err(e) = store_threat_assessment(pool, &assessment).await {
                    tracing::warn!("Failed to store threat assessment: {}", e);
                }
            }

            // Trigger webhook notifications
            trigger_threat_assessment_webhooks(&state.webhook_configs, &assessment).await;

            Ok(Json(assessment))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Store threat assessment in database
#[cfg(feature = "database")]
async fn store_threat_assessment(
    pool: &sqlx::PgPool,
    assessment: &ThreatAssessment,
) -> Result<(), sqlx::Error> {
    let id = Uuid::new_v4();
    let workspace_uuid = assessment.workspace_id.as_ref().and_then(|id| Uuid::parse_str(id).ok());
    let service_uuid = assessment.service_id.as_ref().and_then(|id| Uuid::parse_str(id).ok());

    // Store main assessment
    sqlx::query(
        r#"
        INSERT INTO contract_threat_assessments (
            id, workspace_id, service_id, service_name, endpoint, method, aggregation_level,
            threat_level, threat_score, threat_categories, findings, remediation_suggestions,
            assessed_at, last_updated, created_at
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15
        )
        ON CONFLICT (workspace_id, service_id, endpoint, method, aggregation_level)
        DO UPDATE SET
            threat_level = EXCLUDED.threat_level,
            threat_score = EXCLUDED.threat_score,
            threat_categories = EXCLUDED.threat_categories,
            findings = EXCLUDED.findings,
            remediation_suggestions = EXCLUDED.remediation_suggestions,
            assessed_at = EXCLUDED.assessed_at,
            last_updated = EXCLUDED.last_updated
        "#,
    )
    .bind(id)
    .bind(workspace_uuid)
    .bind(service_uuid)
    .bind(assessment.service_name.as_deref())
    .bind(assessment.endpoint.as_deref())
    .bind(assessment.method.as_deref())
    .bind(format!("{:?}", assessment.aggregation_level))
    .bind(format!("{:?}", assessment.threat_level))
    .bind(assessment.threat_score)
    .bind(serde_json::to_value(&assessment.threat_categories).unwrap_or_default())
    .bind(serde_json::to_value(&assessment.findings).unwrap_or_default())
    .bind(serde_json::to_value(&assessment.remediation_suggestions).unwrap_or_default())
    .bind(assessment.assessed_at)
    .bind(Utc::now())
    .bind(assessment.assessed_at)
    .execute(pool)
    .await?;

    // Store individual findings
    for finding in &assessment.findings {
        let finding_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO threat_findings (
                id, assessment_id, finding_type, severity, description, field_path,
                context, remediation_suggestion, remediation_code_example, confidence,
                ai_generated_remediation, detected_at, created_at, updated_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14
            )
            "#,
        )
        .bind(finding_id)
        .bind(id)
        .bind(format!("{:?}", finding.finding_type))
        .bind(format!("{:?}", finding.severity))
        .bind(&finding.description)
        .bind(finding.field_path.as_deref())
        .bind(serde_json::to_value(&finding.context).unwrap_or_default())
        .bind(None::<String>) // remediation_suggestion from remediation_suggestions
        .bind(None::<String>) // remediation_code_example
        .bind(finding.confidence)
        .bind(false) // ai_generated_remediation
        .bind(Utc::now())
        .bind(Utc::now())
        .bind(Utc::now())
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// List all threat findings
///
/// GET /api/v1/threats/findings
#[cfg(feature = "database")]
pub async fn list_findings(
    State(state): State<ThreatModelingState>,
    Query(_params): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let pool = match state.database.as_ref().and_then(|db| db.pool()) {
        Some(pool) => pool,
        None => {
            return Ok(Json(serde_json::json!({
                "findings": []
            })));
        }
    };

    let rows = sqlx::query(
        "SELECT tf.*, ta.workspace_id, ta.service_id, ta.endpoint, ta.method
         FROM threat_findings tf
         JOIN contract_threat_assessments ta ON tf.assessment_id = ta.id
         ORDER BY tf.detected_at DESC LIMIT 100",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query threat findings: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Map rows to findings
    let mut findings = Vec::new();
    for row in rows {
        let finding_id: uuid::Uuid = row.try_get("id")?;
        let finding_type_str: String = row.try_get("finding_type")?;
        let severity_str: String = row.try_get("severity")?;
        let description: String = row.try_get("description")?;
        let field_path: Option<String> = row.try_get("field_path")?;
        let context_json: serde_json::Value = row.try_get("context")?;
        let confidence: f64 = row.try_get("confidence")?;

        use mockforge_core::contract_drift::threat_modeling::{ThreatCategory, ThreatLevel};
        use std::collections::HashMap;

        let finding_type = match finding_type_str.as_str() {
            "pii_exposure" => ThreatCategory::PiiExposure,
            "dos_risk" => ThreatCategory::DoSRisk,
            "error_leakage" => ThreatCategory::ErrorLeakage,
            "schema_inconsistency" => ThreatCategory::SchemaInconsistency,
            "unbounded_arrays" => ThreatCategory::UnboundedArrays,
            "missing_rate_limits" => ThreatCategory::MissingRateLimits,
            "stack_trace_leakage" => ThreatCategory::StackTraceLeakage,
            "sensitive_data_exposure" => ThreatCategory::SensitiveDataExposure,
            "insecure_schema_design" => ThreatCategory::InsecureSchemaDesign,
            "missing_validation" => ThreatCategory::MissingValidation,
            "excessive_optional_fields" => ThreatCategory::ExcessiveOptionalFields,
            _ => continue, // Skip invalid finding types
        };

        let severity = match severity_str.as_str() {
            "low" => ThreatLevel::Low,
            "medium" => ThreatLevel::Medium,
            "high" => ThreatLevel::High,
            "critical" => ThreatLevel::Critical,
            _ => continue, // Skip invalid severity
        };

        let context: HashMap<String, serde_json::Value> = serde_json::from_value(context_json)
            .unwrap_or_default();

        findings.push(serde_json::json!({
            "id": finding_id.to_string(),
            "finding_type": finding_type_str,
            "severity": severity_str,
            "description": description,
            "field_path": field_path,
            "context": context,
            "confidence": confidence,
        }));
    }

    Ok(Json(serde_json::json!({
        "findings": findings,
        "total": findings.len()
    })))
}

/// List threat findings (no database)
///
/// GET /api/v1/threats/findings
#[cfg(not(feature = "database"))]
pub async fn list_findings(
    State(_state): State<ThreatModelingState>,
    Query(_params): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "findings": []
    })))
}

/// Get remediation suggestions
///
/// GET /api/v1/threats/remediations
#[cfg(feature = "database")]
pub async fn get_remediations(
    State(state): State<ThreatModelingState>,
    Query(_params): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let pool = match state.database.as_ref().and_then(|db| db.pool()) {
        Some(pool) => pool,
        None => {
            return Ok(Json(serde_json::json!({
                "remediations": []
            })));
        }
    };

    // Query remediations from assessments
    let rows = sqlx::query(
        "SELECT remediation_suggestions FROM contract_threat_assessments
         WHERE remediation_suggestions IS NOT NULL AND jsonb_array_length(remediation_suggestions) > 0
         ORDER BY assessed_at DESC LIMIT 50",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| {
        tracing::error!("Failed to query remediations: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Extract and flatten remediation suggestions
    let mut remediations = Vec::new();
    for row in rows {
        let remediations_json: serde_json::Value = row.try_get("remediation_suggestions")?;
        if let serde_json::Value::Array(remediation_array) = remediations_json {
            for remediation in remediation_array {
                remediations.push(remediation);
            }
        }
    }

    Ok(Json(serde_json::json!({
        "remediations": remediations,
        "total": remediations.len()
    })))
}

/// Get remediation suggestions (no database)
///
/// GET /api/v1/threats/remediations
#[cfg(not(feature = "database"))]
pub async fn get_remediations(
    State(_state): State<ThreatModelingState>,
    Query(_params): Query<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "remediations": []
    })))
}

/// Trigger webhook notifications for threat assessment
async fn trigger_threat_assessment_webhooks(
    webhook_configs: &[mockforge_core::incidents::integrations::WebhookConfig],
    assessment: &ThreatAssessment,
) {
    use mockforge_core::incidents::integrations::send_webhook;
    use serde_json::json;

    for webhook in webhook_configs {
        if !webhook.enabled {
            continue;
        }

        let event_type = "threat.assessment.completed";
        if !webhook.events.is_empty() && !webhook.events.contains(&event_type.to_string()) {
            continue;
        }

        let payload = json!({
            "event": event_type,
            "assessment": {
                "workspace_id": assessment.workspace_id,
                "service_id": assessment.service_id,
                "service_name": assessment.service_name,
                "endpoint": assessment.endpoint,
                "method": assessment.method,
                "threat_level": format!("{:?}", assessment.threat_level),
                "threat_score": assessment.threat_score,
                "findings_count": assessment.findings.len(),
                "assessed_at": assessment.assessed_at,
            }
        });

        let webhook_clone = webhook.clone();
        tokio::spawn(async move {
            if let Err(e) = send_webhook(&webhook_clone, &payload).await {
                tracing::warn!("Failed to send threat assessment webhook: {}", e);
            }
        });
    }
}

/// Create router for threat modeling endpoints
pub fn threat_modeling_router(state: ThreatModelingState) -> axum::Router {
    use axum::routing::{get, post};
    use axum::Router;

    Router::new()
        .route("/api/v1/threats/workspace/:workspace_id", get(get_workspace_threats))
        .route("/api/v1/threats/service/:service_id", get(get_service_threats))
        .route("/api/v1/threats/endpoint/:endpoint", get(get_endpoint_threats))
        .route("/api/v1/threats/assess", post(assess_threats))
        .route("/api/v1/threats/findings", get(list_findings))
        .route("/api/v1/threats/remediations", get(get_remediations))
        .with_state(state)
}
