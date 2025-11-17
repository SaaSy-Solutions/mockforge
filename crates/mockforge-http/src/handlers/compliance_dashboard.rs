//! HTTP handlers for compliance dashboard
//!
//! This module provides REST API endpoints for accessing compliance
//! dashboard data, gaps, alerts, and control effectiveness metrics.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use mockforge_core::security::compliance_dashboard::{
    AlertType, ComplianceAlert, ComplianceDashboardEngine, ComplianceGap, ComplianceStandard,
    GapSeverity,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};

/// State for compliance dashboard handlers
#[derive(Clone)]
pub struct ComplianceDashboardState {
    /// Compliance dashboard engine
    pub engine: Arc<RwLock<ComplianceDashboardEngine>>,
}

/// Request to add a compliance gap
#[derive(Debug, Deserialize)]
pub struct AddGapRequest {
    /// Gap description
    pub description: String,
    /// Severity
    pub severity: GapSeverity,
    /// Standard
    pub standard: ComplianceStandard,
    /// Control ID (optional)
    pub control_id: Option<String>,
    /// Target remediation date (optional)
    pub target_remediation_date: Option<chrono::DateTime<chrono::Utc>>,
}

/// Request to add a compliance alert
#[derive(Debug, Deserialize)]
pub struct AddAlertRequest {
    /// Alert type
    pub alert_type: AlertType,
    /// Severity
    pub severity: GapSeverity,
    /// Message
    pub message: String,
    /// Standard (optional)
    pub standard: Option<ComplianceStandard>,
    /// Control ID (optional)
    pub control_id: Option<String>,
}

/// Request to update gap status
#[derive(Debug, Deserialize)]
pub struct UpdateGapStatusRequest {
    /// New status
    pub status: mockforge_core::security::compliance_dashboard::GapStatus,
}

/// Get dashboard data
///
/// GET /api/v1/compliance/dashboard
pub async fn get_dashboard(
    State(state): State<ComplianceDashboardState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let engine = state.engine.read().await;
    let dashboard = engine.get_dashboard_data().await.map_err(|e| {
        error!("Failed to get dashboard data: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(serde_json::to_value(&dashboard).unwrap()))
}

/// Get all compliance gaps
///
/// GET /api/v1/compliance/gaps
pub async fn get_gaps(
    State(state): State<ComplianceDashboardState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let engine = state.engine.read().await;

    let gaps = if let Some(severity_str) = params.get("severity") {
        let severity = match severity_str.as_str() {
            "critical" => GapSeverity::Critical,
            "high" => GapSeverity::High,
            "medium" => GapSeverity::Medium,
            "low" => GapSeverity::Low,
            _ => return Err(StatusCode::BAD_REQUEST),
        };
        engine.get_gaps_by_severity(severity).await.map_err(|e| {
            error!("Failed to get gaps by severity: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    } else {
        engine.get_all_gaps().await.map_err(|e| {
            error!("Failed to get all gaps: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    };

    Ok(Json(serde_json::to_value(&gaps).unwrap()))
}

/// Add a compliance gap
///
/// POST /api/v1/compliance/gaps
pub async fn add_gap(
    State(state): State<ComplianceDashboardState>,
    Json(request): Json<AddGapRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let gap_id = format!("GAP-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());

    let engine = state.engine.write().await;
    engine
        .add_gap(
            gap_id.clone(),
            request.description,
            request.severity,
            request.standard,
            request.control_id,
            request.target_remediation_date,
        )
        .await
        .map_err(|e| {
            error!("Failed to add gap: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("Compliance gap added: {}", gap_id);

    Ok(Json(serde_json::json!({
        "gap_id": gap_id,
        "status": "created"
    })))
}

/// Update gap status
///
/// PATCH /api/v1/compliance/gaps/{gap_id}/status
pub async fn update_gap_status(
    State(state): State<ComplianceDashboardState>,
    Path(gap_id): Path<String>,
    Json(request): Json<UpdateGapStatusRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let engine = state.engine.write().await;
    engine.update_gap_status(&gap_id, request.status).await.map_err(|e| {
        error!("Failed to update gap status: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    info!("Gap status updated: {}", gap_id);

    Ok(Json(serde_json::json!({
        "gap_id": gap_id,
        "status": "updated"
    })))
}

/// Get all compliance alerts
///
/// GET /api/v1/compliance/alerts
pub async fn get_alerts(
    State(state): State<ComplianceDashboardState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let engine = state.engine.read().await;

    let alerts = if let Some(severity_str) = params.get("severity") {
        let severity = match severity_str.as_str() {
            "critical" => GapSeverity::Critical,
            "high" => GapSeverity::High,
            "medium" => GapSeverity::Medium,
            "low" => GapSeverity::Low,
            _ => return Err(StatusCode::BAD_REQUEST),
        };
        engine.get_alerts_by_severity(severity).await.map_err(|e| {
            error!("Failed to get alerts by severity: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    } else {
        engine.get_all_alerts().await.map_err(|e| {
            error!("Failed to get all alerts: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
    };

    Ok(Json(serde_json::to_value(&alerts).unwrap()))
}

/// Add a compliance alert
///
/// POST /api/v1/compliance/alerts
pub async fn add_alert(
    State(state): State<ComplianceDashboardState>,
    Json(request): Json<AddAlertRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let alert_id = format!("ALERT-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap());

    let engine = state.engine.write().await;
    engine
        .add_alert(
            alert_id.clone(),
            request.alert_type,
            request.severity,
            request.message,
            request.standard,
            request.control_id,
        )
        .await
        .map_err(|e| {
            error!("Failed to add alert: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("Compliance alert added: {}", alert_id);

    Ok(Json(serde_json::json!({
        "alert_id": alert_id,
        "status": "created"
    })))
}

/// Create compliance dashboard router
pub fn compliance_dashboard_router(state: ComplianceDashboardState) -> axum::Router {
    use axum::routing::{get, patch, post};

    axum::Router::new()
        .route("/dashboard", get(get_dashboard))
        .route("/gaps", get(get_gaps))
        .route("/gaps", post(add_gap))
        .route("/gaps/{gap_id}/status", patch(update_gap_status))
        .route("/alerts", get(get_alerts))
        .route("/alerts", post(add_alert))
        .with_state(state)
}
