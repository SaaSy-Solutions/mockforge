//! HTTP handlers for privileged access management
//!
//! This module provides REST API endpoints for managing privileged access requests,
//! monitoring privileged actions, and managing privileged sessions.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use mockforge_core::security::{
    emit_security_event, EventActor, EventOutcome, EventTarget, PrivilegedAccessManager,
    PrivilegedActionType, PrivilegedRole, RequestStatus, SecurityEvent, SecurityEventType,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};
use uuid::Uuid;

use crate::handlers::auth_helpers::{extract_user_id_with_fallback, OptionalAuthClaims};

/// State for privileged access handlers
#[derive(Clone)]
pub struct PrivilegedAccessState {
    /// Privileged access manager
    pub manager: Arc<RwLock<PrivilegedAccessManager>>,
}

/// Request to create a privileged access request
#[derive(Debug, Deserialize)]
pub struct CreatePrivilegedAccessRequest {
    /// Requested role
    pub requested_role: PrivilegedRole,
    /// Justification
    pub justification: String,
    /// Business need
    pub business_need: Option<String>,
    /// Manager approval (optional)
    pub manager_approval: Option<Uuid>,
}

/// Request to approve a privileged access request
#[derive(Debug, Deserialize)]
pub struct ApproveRequest {
    /// Whether to approve
    pub approved: bool,
    /// Justification for approval/denial
    pub justification: Option<String>,
    /// Expiration days (if approved)
    pub expiration_days: Option<u64>,
}

/// Response for privileged access request
#[derive(Debug, Serialize)]
pub struct PrivilegedAccessRequestResponse {
    /// Request ID
    pub request_id: Uuid,
    /// Status
    pub status: RequestStatus,
    /// Created at
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Expires at
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Response for privileged actions list
#[derive(Debug, Serialize)]
pub struct PrivilegedActionsResponse {
    /// Actions
    pub actions: Vec<PrivilegedActionSummary>,
}

/// Summary of a privileged action
#[derive(Debug, Serialize)]
pub struct PrivilegedActionSummary {
    /// Action ID
    pub action_id: Uuid,
    /// Action type
    pub action_type: PrivilegedActionType,
    /// Resource
    pub resource: Option<String>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Request privileged access
///
/// POST /api/v1/security/privileged-access/request
pub async fn request_privileged_access(
    State(state): State<PrivilegedAccessState>,
    claims: OptionalAuthClaims,
    Json(request): Json<CreatePrivilegedAccessRequest>,
) -> Result<Json<PrivilegedAccessRequestResponse>, StatusCode> {
    // Extract user ID from authentication claims, or use default for mock server
    let user_id = extract_user_id_with_fallback(&claims);

    let manager = state.manager.read().await;
    let access_request = manager
        .request_privileged_access(
            user_id,
            request.requested_role,
            request.justification,
            request.business_need,
            request.manager_approval,
        )
        .await
        .map_err(|e| {
            error!("Failed to create privileged access request: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("Privileged access request created: {}", access_request.request_id);

    // Emit security event
    let event = SecurityEvent::new(SecurityEventType::AuthzPrivilegeEscalation, None, None)
        .with_actor(EventActor {
            user_id: Some(user_id.to_string()),
            username: None,
            ip_address: None,
            user_agent: None,
        })
        .with_target(EventTarget {
            resource_type: Some("privileged_access_request".to_string()),
            resource_id: Some(access_request.request_id.to_string()),
            method: None,
        })
        .with_outcome(EventOutcome {
            success: true,
            reason: Some("Privileged access request created".to_string()),
        });
    emit_security_event(event).await;

    Ok(Json(PrivilegedAccessRequestResponse {
        request_id: access_request.request_id,
        status: access_request.status,
        created_at: access_request.created_at,
        expires_at: access_request.expires_at,
    }))
}

/// Approve privileged access request (manager)
///
/// POST /api/v1/security/privileged-access/{request_id}/approve-manager
pub async fn approve_manager(
    State(state): State<PrivilegedAccessState>,
    Path(request_id): Path<Uuid>,
    claims: OptionalAuthClaims,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Extract approver ID from authentication claims, or use default for mock server
    let approver_id = extract_user_id_with_fallback(&claims);

    let manager = state.manager.write().await;
    manager
        .approve_manager(request_id, approver_id)
        .await
        .map_err(|e| {
            error!("Failed to approve privileged access request: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    info!("Privileged access request approved by manager: {}", request_id);

    // Emit security event
    let event = SecurityEvent::new(SecurityEventType::AuthzPrivilegeEscalation, None, None)
        .with_actor(EventActor {
            user_id: Some(approver_id.to_string()),
            username: None,
            ip_address: None,
            user_agent: None,
        })
        .with_target(EventTarget {
            resource_type: Some("privileged_access_request".to_string()),
            resource_id: Some(request_id.to_string()),
            method: None,
        })
        .with_outcome(EventOutcome {
            success: true,
            reason: Some("Manager approval granted".to_string()),
        });
    emit_security_event(event).await;

    Ok(Json(serde_json::json!({
        "status": "approved",
        "request_id": request_id
    })))
}

/// Approve privileged access request (security)
///
/// POST /api/v1/security/privileged-access/{request_id}/approve-security
pub async fn approve_security(
    State(state): State<PrivilegedAccessState>,
    Path(request_id): Path<Uuid>,
    claims: OptionalAuthClaims,
    Json(request): Json<ApproveRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    if !request.approved {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Extract approver ID from authentication claims, or use default for mock server
    let approver_id = extract_user_id_with_fallback(&claims);

    let expiration_days = request.expiration_days.unwrap_or(365);

    let manager = state.manager.write().await;
    manager
        .approve_security(request_id, approver_id, expiration_days)
        .await
        .map_err(|e| {
            error!("Failed to approve privileged access request: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    info!("Privileged access request approved by security: {}", request_id);

    // Emit security event
    let event = SecurityEvent::new(SecurityEventType::AuthzPrivilegeEscalation, None, None)
        .with_actor(EventActor {
            user_id: Some(approver_id.to_string()),
            username: None,
            ip_address: None,
            user_agent: None,
        })
        .with_target(EventTarget {
            resource_type: Some("privileged_access_request".to_string()),
            resource_id: Some(request_id.to_string()),
            method: None,
        })
        .with_outcome(EventOutcome {
            success: true,
            reason: Some("Security approval granted".to_string()),
        });
    emit_security_event(event).await;

    Ok(Json(serde_json::json!({
        "status": "approved",
        "request_id": request_id
    })))
}

/// Get privileged actions for a user
///
/// GET /api/v1/security/privileged-access/actions/{user_id}
pub async fn get_user_actions(
    State(state): State<PrivilegedAccessState>,
    Path(user_id): Path<Uuid>,
) -> Result<Json<PrivilegedActionsResponse>, StatusCode> {
    let manager = state.manager.read().await;
    let actions = manager
        .get_user_actions(user_id)
        .await
        .map_err(|e| {
            error!("Failed to get user actions: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let summaries: Vec<PrivilegedActionSummary> = actions
        .into_iter()
        .map(|a| PrivilegedActionSummary {
            action_id: a.action_id,
            action_type: a.action_type,
            resource: a.resource,
            timestamp: a.timestamp,
        })
        .collect();

    Ok(Json(PrivilegedActionsResponse {
        actions: summaries,
    }))
}

/// Get active privileged sessions
///
/// GET /api/v1/security/privileged-access/sessions
pub async fn get_active_sessions(
    State(state): State<PrivilegedAccessState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let manager = state.manager.read().await;
    let sessions = manager
        .get_active_sessions()
        .await
        .map_err(|e| {
            error!("Failed to get active sessions: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({
        "sessions": sessions
    })))
}

/// Create privileged access router
pub fn privileged_access_router(state: PrivilegedAccessState) -> axum::Router {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/request", post(request_privileged_access))
        .route("/{request_id}/approve-manager", post(approve_manager))
        .route("/{request_id}/approve-security", post(approve_security))
        .route("/actions/{user_id}", get(get_user_actions))
        .route("/sessions", get(get_active_sessions))
        .with_state(state)
}
