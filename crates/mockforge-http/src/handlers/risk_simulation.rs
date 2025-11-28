//! Risk simulation API handlers
//!
//! This module provides API endpoints for simulating risk scenarios
//! and managing risk engine configuration.

use axum::{extract::State, http::StatusCode, response::Json};
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

use crate::auth::risk_engine::RiskEngine;

/// Risk simulation state
#[derive(Clone)]
pub struct RiskSimulationState {
    /// Risk engine
    pub risk_engine: Arc<RiskEngine>,
}

/// Set risk score request
#[derive(Debug, Deserialize)]
pub struct SetRiskScoreRequest {
    /// User ID
    pub user_id: String,
    /// Risk score (0.0 - 1.0)
    pub risk_score: f64,
}

/// Set risk factors request
#[derive(Debug, Deserialize)]
pub struct SetRiskFactorsRequest {
    /// User ID
    pub user_id: String,
    /// Risk factors
    pub risk_factors: HashMap<String, f64>,
}

/// Trigger MFA request
#[derive(Debug, Deserialize)]
pub struct TriggerMfaRequest {
    /// User ID
    pub user_id: String,
    /// MFA type
    pub mfa_type: String,
}

/// Block user request
#[derive(Debug, Deserialize)]
pub struct BlockUserRequest {
    /// User ID
    pub user_id: String,
    /// Reason
    pub reason: String,
}

/// Set simulated risk score
pub async fn set_risk_score(
    State(state): State<RiskSimulationState>,
    Json(request): Json<SetRiskScoreRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state
        .risk_engine
        .set_simulated_risk(request.user_id.clone(), Some(request.risk_score))
        .await;

    Ok(Json(serde_json::json!({
        "success": true,
        "user_id": request.user_id,
        "risk_score": request.risk_score
    })))
}

/// Set simulated risk factors
pub async fn set_risk_factors(
    State(state): State<RiskSimulationState>,
    Json(request): Json<SetRiskFactorsRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state
        .risk_engine
        .set_simulated_factors(request.user_id.clone(), request.risk_factors.clone())
        .await;

    Ok(Json(serde_json::json!({
        "success": true,
        "user_id": request.user_id,
        "risk_factors": request.risk_factors
    })))
}

/// Clear simulated risk
pub async fn clear_risk(
    State(state): State<RiskSimulationState>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state.risk_engine.clear_simulated_risk(&user_id).await;

    Ok(Json(serde_json::json!({
        "success": true,
        "user_id": user_id,
        "message": "Simulated risk cleared"
    })))
}

/// Trigger MFA prompt
pub async fn trigger_mfa(
    State(state): State<RiskSimulationState>,
    Json(request): Json<TriggerMfaRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Set risk score high enough to trigger MFA
    state.risk_engine.set_simulated_risk(request.user_id.clone(), Some(0.8)).await;

    Ok(Json(serde_json::json!({
        "success": true,
        "user_id": request.user_id,
        "mfa_type": request.mfa_type,
        "message": "MFA prompt triggered"
    })))
}

/// Block user login
pub async fn block_user(
    State(state): State<RiskSimulationState>,
    Json(request): Json<BlockUserRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Set risk score high enough to block
    state.risk_engine.set_simulated_risk(request.user_id.clone(), Some(0.95)).await;

    Ok(Json(serde_json::json!({
        "success": true,
        "user_id": request.user_id,
        "reason": request.reason,
        "message": "User login blocked"
    })))
}

/// Get risk assessment for user
pub async fn get_risk_assessment(
    State(state): State<RiskSimulationState>,
    axum::extract::Path(user_id): axum::extract::Path<String>,
) -> Json<serde_json::Value> {
    let risk_factors = HashMap::new();
    let assessment = state.risk_engine.assess_risk(&user_id, &risk_factors).await;

    Json(serde_json::json!({
        "user_id": user_id,
        "risk_score": assessment.risk_score,
        "risk_factors": assessment.risk_factors,
        "recommended_action": assessment.recommended_action
    }))
}

/// Create risk simulation router
pub fn risk_simulation_router(state: RiskSimulationState) -> axum::Router {
    use axum::routing::{delete, get, post};

    axum::Router::new()
        .route("/risk/simulate", post(set_risk_score))
        .route("/risk/factors", post(set_risk_factors))
        .route("/risk/clear/{user_id}", delete(clear_risk))
        .route("/risk/trigger-mfa", post(trigger_mfa))
        .route("/risk/block", post(block_user))
        .route("/risk/assessment/{user_id}", get(get_risk_assessment))
        .with_state(state)
}
