//! HTTP handlers for change management
//!
//! This module provides REST API endpoints for managing change requests,
//! approvals, implementation tracking, and completion.

use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::Json,
};
use mockforge_core::security::{
    change_management::{
        ChangeManagementEngine, ChangePriority, ChangeStatus, ChangeType, ChangeUrgency,
    },
    emit_security_event, EventActor, EventOutcome, EventTarget, SecurityEvent, SecurityEventType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};
use uuid::Uuid;

use crate::auth::types::AuthClaims;
use crate::handlers::auth_helpers::{extract_user_id_with_fallback, extract_username_from_claims};

/// State for change management handlers
#[derive(Clone)]
pub struct ChangeManagementState {
    /// Change management engine
    pub engine: Arc<RwLock<ChangeManagementEngine>>,
}

/// Request to create a change request
#[derive(Debug, Deserialize)]
pub struct CreateChangeRequest {
    /// Change title
    pub title: String,
    /// Change description
    pub description: String,
    /// Change type
    pub change_type: ChangeType,
    /// Change priority
    pub priority: ChangePriority,
    /// Change urgency
    pub urgency: ChangeUrgency,
    /// Affected systems
    pub affected_systems: Vec<String>,
    /// Impact scope
    pub impact_scope: Option<String>,
    /// Risk level
    pub risk_level: Option<String>,
    /// Rollback plan
    pub rollback_plan: Option<String>,
    /// Testing required
    pub testing_required: bool,
    /// Test plan
    pub test_plan: Option<String>,
    /// Test environment
    pub test_environment: Option<String>,
}

/// Request to approve a change
#[derive(Debug, Deserialize)]
pub struct ApproveChangeRequest {
    /// Whether to approve
    pub approved: bool,
    /// Comments
    pub comments: Option<String>,
    /// Conditions (if approved)
    pub conditions: Option<Vec<String>>,
    /// Rejection reason (if rejected)
    pub reason: Option<String>,
}

/// Request to start implementation
#[derive(Debug, Deserialize)]
pub struct StartImplementationRequest {
    /// Implementation plan
    pub implementation_plan: String,
    /// Scheduled time (optional)
    pub scheduled_time: Option<chrono::DateTime<chrono::Utc>>,
}

/// Request to complete change
#[derive(Debug, Deserialize)]
pub struct CompleteChangeRequest {
    /// Test results
    pub test_results: Option<String>,
    /// Post-implementation review
    pub post_implementation_review: Option<String>,
}

/// Response for change request
#[derive(Debug, Serialize)]
pub struct ChangeRequestResponse {
    /// Change ID
    pub change_id: String,
    /// Status
    pub status: ChangeStatus,
    /// Approvers
    pub approvers: Vec<String>,
    /// Request date
    pub request_date: chrono::DateTime<chrono::Utc>,
}

/// Response for change list
#[derive(Debug, Serialize)]
pub struct ChangeListResponse {
    /// Changes
    pub changes: Vec<ChangeSummary>,
}

/// Summary of a change request
#[derive(Debug, Serialize)]
pub struct ChangeSummary {
    /// Change ID
    pub change_id: String,
    /// Title
    pub title: String,
    /// Status
    pub status: ChangeStatus,
    /// Priority
    pub priority: ChangePriority,
    /// Request date
    pub request_date: chrono::DateTime<chrono::Utc>,
}

/// Create a change request
///
/// POST /api/v1/change-management/change-requests
pub async fn create_change_request(
    State(state): State<ChangeManagementState>,
    Json(request): Json<CreateChangeRequest>,
    claims: Option<Extension<AuthClaims>>,
) -> Result<Json<ChangeRequestResponse>, StatusCode> {
    // Extract requester ID from authentication claims, or use default for mock server
    let requester_id = extract_user_id_with_fallback(claims);

    let engine = state.engine.write().await;
    let change = engine
        .create_change_request(
            request.title,
            request.description,
            requester_id,
            request.change_type,
            request.priority,
            request.urgency,
            request.affected_systems,
            request.testing_required,
            request.test_plan,
            request.test_environment,
            request.rollback_plan,
            request.impact_scope,
            request.risk_level,
        )
        .await
        .map_err(|e| {
            error!("Failed to create change request: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    info!("Change request created: {}", change.change_id);

    // Emit security event
    let event = SecurityEvent::new(SecurityEventType::ConfigChanged, None, None)
        .with_actor(EventActor {
            user_id: Some(requester_id.to_string()),
            username: None,
            ip_address: None,
            user_agent: None,
        })
        .with_target(EventTarget {
            resource_type: Some("change_request".to_string()),
            resource_id: Some(change.change_id.clone()),
            method: None,
        })
        .with_outcome(EventOutcome {
            success: true,
            reason: Some("Change request created".to_string()),
        });
    emit_security_event(event).await;

    Ok(Json(ChangeRequestResponse {
        change_id: change.change_id,
        status: change.status,
        approvers: change.approvers,
        request_date: change.request_date,
    }))
}

/// Approve a change request
///
/// POST /api/v1/change-management/change-requests/{change_id}/approve
pub async fn approve_change(
    State(state): State<ChangeManagementState>,
    Path(change_id): Path<String>,
    Json(request): Json<ApproveChangeRequest>,
    claims: Option<Extension<AuthClaims>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Extract approver ID and name from authentication claims, or use defaults for mock server
    let approver_id = extract_user_id_with_fallback(claims.clone());
    let approver = extract_username_from_claims(claims)
        .unwrap_or_else(|| format!("user-{}", approver_id));

    let engine = state.engine.write().await;

    if request.approved {
        engine
            .approve_change(&change_id, &approver, approver_id, request.comments, request.conditions)
            .await
            .map_err(|e| {
                error!("Failed to approve change: {}", e);
                StatusCode::BAD_REQUEST
            })?;

        info!("Change request approved: {}", change_id);

        // Emit security event
        let event = SecurityEvent::new(SecurityEventType::ConfigChanged, None, None)
            .with_actor(EventActor {
                user_id: Some(approver_id.to_string()),
                username: None,
                ip_address: None,
                user_agent: None,
            })
            .with_target(EventTarget {
                resource_type: Some("change_request".to_string()),
                resource_id: Some(change_id.clone()),
                method: None,
            })
            .with_outcome(EventOutcome {
                success: true,
                reason: Some("Change approved".to_string()),
            });
        emit_security_event(event).await;

        Ok(Json(serde_json::json!({
            "status": "approved",
            "change_id": change_id
        })))
    } else {
        let reason = request.reason.unwrap_or_else(|| "No reason provided".to_string());
        engine
            .reject_change(&change_id, &approver, approver_id, reason.clone())
            .await
            .map_err(|e| {
                error!("Failed to reject change: {}", e);
                StatusCode::BAD_REQUEST
            })?;

        info!("Change request rejected: {}", change_id);

        // Emit security event
        let event = SecurityEvent::new(SecurityEventType::ConfigChanged, None, None)
            .with_actor(EventActor {
                user_id: Some(approver_id.to_string()),
                username: None,
                ip_address: None,
                user_agent: None,
            })
            .with_target(EventTarget {
                resource_type: Some("change_request".to_string()),
                resource_id: Some(change_id.clone()),
                method: None,
            })
            .with_outcome(EventOutcome {
                success: false,
                reason: Some(format!("Change rejected: {}", reason)),
            });
        emit_security_event(event).await;

        Ok(Json(serde_json::json!({
            "status": "rejected",
            "change_id": change_id
        })))
    }
}

/// Start change implementation
///
/// POST /api/v1/change-management/change-requests/{change_id}/implement
pub async fn start_implementation(
    State(state): State<ChangeManagementState>,
    Path(change_id): Path<String>,
    Json(request): Json<StartImplementationRequest>,
    claims: Option<Extension<AuthClaims>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Extract implementer ID from authentication claims, or use default for mock server
    let implementer_id = extract_user_id_with_fallback(claims);

    let engine = state.engine.write().await;
    engine
        .start_implementation(&change_id, implementer_id, request.implementation_plan, request.scheduled_time)
        .await
        .map_err(|e| {
            error!("Failed to start implementation: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    info!("Change implementation started: {}", change_id);

    // Emit security event
    let event = SecurityEvent::new(SecurityEventType::ConfigChanged, None, None)
        .with_actor(EventActor {
            user_id: Some(implementer_id.to_string()),
            username: None,
            ip_address: None,
            user_agent: None,
        })
        .with_target(EventTarget {
            resource_type: Some("change_request".to_string()),
            resource_id: Some(change_id.clone()),
            method: None,
        })
        .with_outcome(EventOutcome {
            success: true,
            reason: Some("Change implementation started".to_string()),
        });
    emit_security_event(event).await;

    Ok(Json(serde_json::json!({
        "status": "implementing",
        "change_id": change_id
    })))
}

/// Complete change implementation
///
/// POST /api/v1/change-management/change-requests/{change_id}/complete
pub async fn complete_change(
    State(state): State<ChangeManagementState>,
    Path(change_id): Path<String>,
    Json(request): Json<CompleteChangeRequest>,
    claims: Option<Extension<AuthClaims>>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Extract implementer ID from authentication claims, or use default for mock server
    let implementer_id = extract_user_id_with_fallback(claims);

    let engine = state.engine.write().await;
    engine
        .complete_change(&change_id, implementer_id, request.test_results, request.post_implementation_review)
        .await
        .map_err(|e| {
            error!("Failed to complete change: {}", e);
            StatusCode::BAD_REQUEST
        })?;

    info!("Change implementation completed: {}", change_id);

    // Emit security event
    let event = SecurityEvent::new(SecurityEventType::ConfigChanged, None, None)
        .with_actor(EventActor {
            user_id: Some(implementer_id.to_string()),
            username: None,
            ip_address: None,
            user_agent: None,
        })
        .with_target(EventTarget {
            resource_type: Some("change_request".to_string()),
            resource_id: Some(change_id.clone()),
            method: None,
        })
        .with_outcome(EventOutcome {
            success: true,
            reason: Some("Change implementation completed".to_string()),
        });
    emit_security_event(event).await;

    Ok(Json(serde_json::json!({
        "status": "completed",
        "change_id": change_id
    })))
}

/// Get a change request
///
/// GET /api/v1/change-management/change-requests/{change_id}
pub async fn get_change(
    State(state): State<ChangeManagementState>,
    Path(change_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let engine = state.engine.read().await;
    let change = engine
        .get_change(&change_id)
        .await
        .map_err(|e| {
            error!("Failed to get change: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            error!("Change request not found: {}", change_id);
            StatusCode::NOT_FOUND
        })?;

    Ok(Json(serde_json::to_value(&change).unwrap()))
}

/// List change requests
///
/// GET /api/v1/change-management/change-requests
pub async fn list_changes(
    State(state): State<ChangeManagementState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ChangeListResponse>, StatusCode> {
    let engine = state.engine.read().await;

    let changes = if let Some(status_str) = params.get("status") {
        // Parse status from query parameter
        let status = match status_str.as_str() {
            "pending_approval" => ChangeStatus::PendingApproval,
            "approved" => ChangeStatus::Approved,
            "rejected" => ChangeStatus::Rejected,
            "implementing" => ChangeStatus::Implementing,
            "completed" => ChangeStatus::Completed,
            "cancelled" => ChangeStatus::Cancelled,
            "rolled_back" => ChangeStatus::RolledBack,
            _ => return Err(StatusCode::BAD_REQUEST),
        };
        engine
            .get_changes_by_status(status)
            .await
            .map_err(|e| {
                error!("Failed to get changes by status: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    } else if let Some(requester_str) = params.get("requester_id") {
        let requester_id = requester_str
            .parse::<Uuid>()
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        engine
            .get_changes_by_requester(requester_id)
            .await
            .map_err(|e| {
                error!("Failed to get changes by requester: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    } else {
        engine
            .get_all_changes()
            .await
            .map_err(|e| {
                error!("Failed to get all changes: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
    };

    let summaries: Vec<ChangeSummary> = changes
        .into_iter()
        .map(|c| ChangeSummary {
            change_id: c.change_id,
            title: c.title,
            status: c.status,
            priority: c.priority,
            request_date: c.request_date,
        })
        .collect();

    Ok(Json(ChangeListResponse {
        changes: summaries,
    }))
}

/// Get change history
///
/// GET /api/v1/change-management/change-requests/{change_id}/history
pub async fn get_change_history(
    State(state): State<ChangeManagementState>,
    Path(change_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let engine = state.engine.read().await;
    let change = engine
        .get_change(&change_id)
        .await
        .map_err(|e| {
            error!("Failed to get change: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            error!("Change request not found: {}", change_id);
            StatusCode::NOT_FOUND
        })?;

    Ok(Json(serde_json::json!({
        "change_id": change.change_id,
        "history": change.history
    })))
}

/// Create change management router
pub fn change_management_router(state: ChangeManagementState) -> axum::Router {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/change-requests", get(list_changes))
        .route("/change-requests", post(create_change_request))
        .route("/change-requests/{change_id}", get(get_change))
        .route("/change-requests/{change_id}/approve", post(approve_change))
        .route("/change-requests/{change_id}/implement", post(start_implementation))
        .route("/change-requests/{change_id}/complete", post(complete_change))
        .route("/change-requests/{change_id}/history", get(get_change_history))
        .with_state(state)
}
