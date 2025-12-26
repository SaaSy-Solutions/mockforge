//! HTTP handlers for risk assessment
//!
//! This module provides REST API endpoints for managing the risk register,
//! creating risks, updating assessments, and tracking treatment plans.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use mockforge_core::security::{
    emit_security_event,
    risk_assessment::{
        Impact, Likelihood, Risk, RiskAssessmentEngine, RiskCategory, RiskLevel, TreatmentOption,
        TreatmentStatus,
    },
    EventActor, EventOutcome, EventTarget, SecurityEvent, SecurityEventType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

use crate::handlers::auth_helpers::{extract_user_id_with_fallback, OptionalAuthClaims};

/// State for risk assessment handlers
#[derive(Clone)]
pub struct RiskAssessmentState {
    /// Risk assessment engine
    pub engine: Arc<RwLock<RiskAssessmentEngine>>,
}

/// Request to create a risk
#[derive(Debug, Deserialize)]
pub struct CreateRiskRequest {
    /// Risk title
    pub title: String,
    /// Risk description
    pub description: String,
    /// Risk category
    pub category: RiskCategory,
    /// Risk subcategory (optional)
    pub subcategory: Option<String>,
    /// Likelihood
    pub likelihood: Likelihood,
    /// Impact
    pub impact: Impact,
    /// Threat description (optional)
    pub threat: Option<String>,
    /// Vulnerability description (optional)
    pub vulnerability: Option<String>,
    /// Affected asset (optional)
    pub asset: Option<String>,
    /// Existing controls (optional)
    pub existing_controls: Option<Vec<String>>,
    /// Compliance requirements (optional)
    pub compliance_requirements: Option<Vec<String>>,
}

/// Request to update risk assessment
#[derive(Debug, Deserialize)]
pub struct UpdateRiskAssessmentRequest {
    /// New likelihood (optional)
    pub likelihood: Option<Likelihood>,
    /// New impact (optional)
    pub impact: Option<Impact>,
}

/// Request to update treatment plan
#[derive(Debug, Deserialize)]
pub struct UpdateTreatmentPlanRequest {
    /// Treatment option
    pub treatment_option: TreatmentOption,
    /// Treatment plan
    pub treatment_plan: Vec<String>,
    /// Treatment owner (optional)
    pub treatment_owner: Option<String>,
    /// Treatment deadline (optional)
    pub treatment_deadline: Option<chrono::DateTime<chrono::Utc>>,
}

/// Request to set residual risk
#[derive(Debug, Deserialize)]
pub struct SetResidualRiskRequest {
    /// Residual likelihood
    pub residual_likelihood: Likelihood,
    /// Residual impact
    pub residual_impact: Impact,
}

/// Response for risk list
#[derive(Debug, Serialize)]
pub struct RiskListResponse {
    /// Risks
    pub risks: Vec<Risk>,
    /// Summary
    pub summary: mockforge_core::security::risk_assessment::RiskSummary,
}

/// Create a new risk
///
/// POST /api/v1/security/risks
pub async fn create_risk(
    State(state): State<RiskAssessmentState>,
    claims: OptionalAuthClaims,
    Json(request): Json<CreateRiskRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Extract user ID from authentication claims, or use default for mock server
    let created_by = extract_user_id_with_fallback(&claims);

    let engine = state.engine.write().await;
    let risk = engine
        .create_risk(
            request.title,
            request.description,
            request.category,
            request.likelihood,
            request.impact,
            created_by,
        )
        .await
        .map_err(|e| {
            error!("Failed to create risk: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Set optional fields
    if let Some(subcategory) = request.subcategory {
        // Note: Risk struct doesn't have a setter for subcategory in the engine
        // This would need to be added to the engine or handled differently
    }
    if let Some(threat) = request.threat {
        // Similar note - would need engine support
    }
    if let Some(vulnerability) = request.vulnerability {
        // Similar note
    }
    if let Some(asset) = request.asset {
        // Similar note
    }
    if let Some(controls) = request.existing_controls {
        // Similar note
    }
    if let Some(requirements) = request.compliance_requirements {
        // Similar note
    }

    info!("Risk created: {}", risk.risk_id);

    // Emit security event
    let event = SecurityEvent::new(SecurityEventType::ConfigChanged, None, None)
        .with_actor(EventActor {
            user_id: Some(created_by.to_string()),
            username: None,
            ip_address: None,
            user_agent: None,
        })
        .with_target(EventTarget {
            resource_type: Some("risk".to_string()),
            resource_id: Some(risk.risk_id.clone()),
            method: None,
        })
        .with_outcome(EventOutcome {
            success: true,
            reason: Some("Risk created".to_string()),
        });
    emit_security_event(event).await;

    Ok(Json(serde_json::to_value(&risk).map_err(|e| {
        error!("Failed to serialize risk: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?))
}

/// Get a risk by ID
///
/// GET /api/v1/security/risks/{risk_id}
pub async fn get_risk(
    State(state): State<RiskAssessmentState>,
    Path(risk_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let engine = state.engine.read().await;
    let risk = engine
        .get_risk(&risk_id)
        .await
        .map_err(|e| {
            error!("Failed to get risk: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            error!("Risk not found: {}", risk_id);
            StatusCode::NOT_FOUND
        })?;

    Ok(Json(serde_json::to_value(&risk).map_err(|e| {
        error!("Failed to serialize risk: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?))
}

/// List all risks
///
/// GET /api/v1/security/risks
pub async fn list_risks(
    State(state): State<RiskAssessmentState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<RiskListResponse>, StatusCode> {
    let engine = state.engine.read().await;

    let risks = if let Some(level_str) = params.get("level") {
        let level = match level_str.as_str() {
            "critical" => RiskLevel::Critical,
            "high" => RiskLevel::High,
            "medium" => RiskLevel::Medium,
            "low" => RiskLevel::Low,
            _ => return Err(StatusCode::BAD_REQUEST),
        };
        engine.get_risks_by_level(level).await.map_err(|e| {
            error!("Failed to get risks by level: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    } else if let Some(category_str) = params.get("category") {
        let category = match category_str.as_str() {
            "technical" => RiskCategory::Technical,
            "operational" => RiskCategory::Operational,
            "compliance" => RiskCategory::Compliance,
            "business" => RiskCategory::Business,
            _ => return Err(StatusCode::BAD_REQUEST),
        };
        engine.get_risks_by_category(category).await.map_err(|e| {
            error!("Failed to get risks by category: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    } else if let Some(status_str) = params.get("treatment_status") {
        let status = match status_str.as_str() {
            "not_started" => TreatmentStatus::NotStarted,
            "in_progress" => TreatmentStatus::InProgress,
            "completed" => TreatmentStatus::Completed,
            "on_hold" => TreatmentStatus::OnHold,
            _ => return Err(StatusCode::BAD_REQUEST),
        };
        engine.get_risks_by_treatment_status(status).await.map_err(|e| {
            error!("Failed to get risks by treatment status: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    } else {
        engine.get_all_risks().await.map_err(|e| {
            error!("Failed to get all risks: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    };

    let summary = engine.get_risk_summary().await.map_err(|e| {
        error!("Failed to get risk summary: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(RiskListResponse { risks, summary }))
}

/// Update risk assessment (likelihood/impact)
///
/// PUT /api/v1/security/risks/{risk_id}/assessment
pub async fn update_risk_assessment(
    State(state): State<RiskAssessmentState>,
    Path(risk_id): Path<String>,
    claims: OptionalAuthClaims,
    Json(request): Json<UpdateRiskAssessmentRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Extract user ID from authentication claims, or use default for mock server
    let updated_by = extract_user_id_with_fallback(&claims);

    let engine = state.engine.write().await;
    engine
        .update_risk_assessment(&risk_id, request.likelihood, request.impact)
        .await
        .map_err(|e| {
            error!("Failed to update risk assessment: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    info!("Risk assessment updated: {}", risk_id);

    // Emit security event
    let event = SecurityEvent::new(SecurityEventType::ConfigChanged, None, None)
        .with_actor(EventActor {
            user_id: Some(updated_by.to_string()),
            username: None,
            ip_address: None,
            user_agent: None,
        })
        .with_target(EventTarget {
            resource_type: Some("risk".to_string()),
            resource_id: Some(risk_id.clone()),
            method: None,
        })
        .with_outcome(EventOutcome {
            success: true,
            reason: Some("Risk assessment updated".to_string()),
        });
    emit_security_event(event).await;

    Ok(Json(serde_json::json!({
        "risk_id": risk_id,
        "status": "updated"
    })))
}

/// Update treatment plan
///
/// PUT /api/v1/security/risks/{risk_id}/treatment
pub async fn update_treatment_plan(
    State(state): State<RiskAssessmentState>,
    Path(risk_id): Path<String>,
    claims: OptionalAuthClaims,
    Json(request): Json<UpdateTreatmentPlanRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Extract user ID from authentication claims, or use default for mock server
    let updated_by = extract_user_id_with_fallback(&claims);

    let engine = state.engine.write().await;
    engine
        .update_treatment_plan(
            &risk_id,
            request.treatment_option,
            request.treatment_plan,
            request.treatment_owner,
            request.treatment_deadline,
        )
        .await
        .map_err(|e| {
            error!("Failed to update treatment plan: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    info!("Treatment plan updated: {}", risk_id);

    // Emit security event
    let event = SecurityEvent::new(SecurityEventType::ConfigChanged, None, None)
        .with_actor(EventActor {
            user_id: Some(updated_by.to_string()),
            username: None,
            ip_address: None,
            user_agent: None,
        })
        .with_target(EventTarget {
            resource_type: Some("risk".to_string()),
            resource_id: Some(risk_id.clone()),
            method: None,
        })
        .with_outcome(EventOutcome {
            success: true,
            reason: Some("Treatment plan updated".to_string()),
        });
    emit_security_event(event).await;

    Ok(Json(serde_json::json!({
        "risk_id": risk_id,
        "status": "updated"
    })))
}

/// Update treatment status
///
/// PATCH /api/v1/security/risks/{risk_id}/treatment/status
pub async fn update_treatment_status(
    State(state): State<RiskAssessmentState>,
    Path(risk_id): Path<String>,
    claims: OptionalAuthClaims,
    Json(request): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Extract user ID from authentication claims, or use default for mock server
    let _updated_by = extract_user_id_with_fallback(&claims);

    let status_str =
        request.get("status").and_then(|v| v.as_str()).ok_or(StatusCode::BAD_REQUEST)?;

    let status = match status_str {
        "not_started" => TreatmentStatus::NotStarted,
        "in_progress" => TreatmentStatus::InProgress,
        "completed" => TreatmentStatus::Completed,
        "on_hold" => TreatmentStatus::OnHold,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    let engine = state.engine.write().await;
    engine.update_treatment_status(&risk_id, status).await.map_err(|e| {
        error!("Failed to update treatment status: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    info!("Treatment status updated: {}", risk_id);

    Ok(Json(serde_json::json!({
        "risk_id": risk_id,
        "status": "updated"
    })))
}

/// Set residual risk
///
/// PUT /api/v1/security/risks/{risk_id}/residual
pub async fn set_residual_risk(
    State(state): State<RiskAssessmentState>,
    Path(risk_id): Path<String>,
    claims: OptionalAuthClaims,
    Json(request): Json<SetResidualRiskRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Extract user ID from authentication claims, or use default for mock server
    let _updated_by = extract_user_id_with_fallback(&claims);

    let engine = state.engine.write().await;
    engine
        .set_residual_risk(&risk_id, request.residual_likelihood, request.residual_impact)
        .await
        .map_err(|e| {
            error!("Failed to set residual risk: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    info!("Residual risk set: {}", risk_id);

    Ok(Json(serde_json::json!({
        "risk_id": risk_id,
        "status": "updated"
    })))
}

/// Review a risk
///
/// POST /api/v1/security/risks/{risk_id}/review
pub async fn review_risk(
    State(state): State<RiskAssessmentState>,
    Path(risk_id): Path<String>,
    claims: OptionalAuthClaims,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Extract user ID from authentication claims, or use default for mock server
    let reviewed_by = extract_user_id_with_fallback(&claims);

    let engine = state.engine.write().await;
    engine.review_risk(&risk_id, reviewed_by).await.map_err(|e| {
        error!("Failed to review risk: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    info!("Risk reviewed: {}", risk_id);

    Ok(Json(serde_json::json!({
        "risk_id": risk_id,
        "status": "reviewed"
    })))
}

/// Get risks due for review
///
/// GET /api/v1/security/risks/due-for-review
pub async fn get_risks_due_for_review(
    State(state): State<RiskAssessmentState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let engine = state.engine.read().await;
    let risks = engine.get_risks_due_for_review().await.map_err(|e| {
        error!("Failed to get risks due for review: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(serde_json::to_value(&risks).map_err(|e| {
        error!("Failed to serialize risks: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?))
}

/// Get risk summary
///
/// GET /api/v1/security/risks/summary
pub async fn get_risk_summary(
    State(state): State<RiskAssessmentState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let engine = state.engine.read().await;
    let summary = engine.get_risk_summary().await.map_err(|e| {
        error!("Failed to get risk summary: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(serde_json::to_value(&summary).map_err(|e| {
        error!("Failed to serialize summary: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?))
}

/// Create risk assessment router
pub fn risk_assessment_router(state: RiskAssessmentState) -> axum::Router {
    use axum::routing::{get, patch, post, put};

    axum::Router::new()
        .route("/risks", get(list_risks))
        .route("/risks", post(create_risk))
        .route("/risks/{risk_id}", get(get_risk))
        .route("/risks/{risk_id}/assessment", put(update_risk_assessment))
        .route("/risks/{risk_id}/treatment", put(update_treatment_plan))
        .route("/risks/{risk_id}/treatment/status", patch(update_treatment_status))
        .route("/risks/{risk_id}/residual", put(set_residual_risk))
        .route("/risks/{risk_id}/review", post(review_risk))
        .route("/risks/due-for-review", get(get_risks_due_for_review))
        .route("/risks/summary", get(get_risk_summary))
        .with_state(state)
}
