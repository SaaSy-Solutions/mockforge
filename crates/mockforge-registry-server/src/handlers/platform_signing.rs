//! HTTP control plane for the platform signing-root rotation
//! (Issue #568 — follow-up to #567 / #550).
//!
//! Four endpoints, all under `/api/internal/`:
//!
//!   - `POST /api/internal/platform-signing/begin-handover`
//!   - `POST /api/internal/platform-signing/retire-old`
//!   - `POST /api/internal/platform-signing/emergency-revoke`
//!   - `GET  /api/internal/plugin-rotation-events`
//!
//! ## Auth model
//!
//! All four use the shared internal-API bearer token
//! (`MOCKFORGE_INTERNAL_API_TOKEN`) for consistency with the existing
//! `internal_test_runs` / `internal_contract_diff` family. The issue
//! talks about a scope-bound platform-operator JWT (`platform.signing.rotate`
//! / `platform.signing.read`); that's the planned upgrade — the same
//! pattern the runner endpoints will adopt when mTLS plumbing lands.
//! The shared token is fine for the runbook's "operator from a bastion
//! VPN" call site today.
//!
//! ## Operator identity
//!
//! The audit-log layer wants `(org_id, user_id, ip, user_agent)` on
//! every row. The internal-token path doesn't carry a JWT, so the
//! body of each POST endpoint MUST include `operator_org_id` and
//! `operator_user_id`. Operators run `aws sts get-caller-identity`
//! to discover their internal IDs once (post-onboarding) and bake them
//! into the bastion's environment.

use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    Json,
};
use chrono::Duration;
use mockforge_platform_signing::{RotationError, RotationEvent, RotationPhase};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::platform_signing::{ControllerError, OperatorIdentity};
use crate::AppState;

/// Maximum transition window we accept on the wire. Anything beyond a
/// year is almost certainly a typo (`30000` instead of `30`); refuse
/// it loudly rather than scheduling a year-long key overlap.
const MAX_TRANSITION_WINDOW_DAYS: i64 = 365;

/// Verify the request carries the shared internal-API bearer token.
///
/// Mirrors `internal_test_runs::require_internal_auth`. Returns
/// `InvalidRequest("Not found")` for any auth failure so a leaked /
/// curl'd request can't probe whether the endpoint is mounted.
fn require_internal_auth(headers: &HeaderMap) -> ApiResult<()> {
    let configured = match std::env::var("MOCKFORGE_INTERNAL_API_TOKEN") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            return Err(ApiError::Internal(anyhow::anyhow!(
                "MOCKFORGE_INTERNAL_API_TOKEN not configured"
            )));
        }
    };
    let provided = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or_else(|| ApiError::InvalidRequest("Not found".into()))?;
    if !constant_time_eq(provided.as_bytes(), configured.as_bytes()) {
        return Err(ApiError::InvalidRequest("Not found".into()));
    }
    Ok(())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Pull the inbound IP + user-agent off the request headers so they
/// can be threaded through to the audit-log layer.
fn operator_context(
    headers: &HeaderMap,
    body_org_id: Uuid,
    body_user_id: Uuid,
) -> OperatorIdentity {
    let ip_address = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string());
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    OperatorIdentity {
        org_id: body_org_id,
        user_id: body_user_id,
        ip_address,
        user_agent,
    }
}

/// Translate a [`ControllerError`] into the right HTTP status. Kept
/// out of `error.rs` because the controller is registry-server-local —
/// dragging the variant into `mockforge-registry-core` would force
/// every consumer (including the OSS admin server) to depend on
/// platform-signing concepts they don't need.
fn controller_error_to_response(err: ControllerError) -> axum::response::Response {
    use axum::response::IntoResponse;
    match err {
        ControllerError::Rotation(rot) => {
            let (status, code) = match &rot {
                RotationError::WrongPhase { .. } => (StatusCode::CONFLICT, "WRONG_PHASE"),
                RotationError::SameKey => (StatusCode::BAD_REQUEST, "SAME_KEY"),
                RotationError::InvalidTransitionWindow => {
                    (StatusCode::BAD_REQUEST, "INVALID_TRANSITION_WINDOW")
                }
                RotationError::TransitionStillOpen { .. } => {
                    (StatusCode::CONFLICT, "TRANSITION_STILL_OPEN")
                }
                RotationError::NoRotationInProgress => {
                    (StatusCode::CONFLICT, "NO_ROTATION_IN_PROGRESS")
                }
                RotationError::Encoding(_) | RotationError::Signer(_) => {
                    (StatusCode::INTERNAL_SERVER_ERROR, "ROTATION_ERROR")
                }
            };
            (status, Json(json!({ "error": { "code": code, "message": rot.to_string() } })))
                .into_response()
        }
        ControllerError::Backend(msg) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({
                "error": { "code": "SIGNER_BACKEND_ERROR", "message": msg }
            })),
        )
            .into_response(),
        ControllerError::NotConfigured => {
            unreachable!("callers must check controller presence before reaching the error path")
        }
    }
}

/// Look up the controller on the request state; respond 503 if the
/// deployment didn't configure one (OSS smoke runs, dev). Returns a
/// borrowed `Arc` so the caller can hold it across an `await` without
/// re-cloning.
fn require_controller(
    state: &AppState,
) -> Result<
    std::sync::Arc<dyn crate::platform_signing::PlatformSigningController>,
    axum::response::Response,
> {
    use axum::response::IntoResponse;
    state.platform_signing.clone().ok_or_else(|| {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({
                "error": {
                    "code": "PLATFORM_SIGNING_NOT_CONFIGURED",
                    "message": "MOCKFORGE_PLATFORM_SIGNING_KMS_KEY_ID is not set on this deployment",
                }
            })),
        )
            .into_response()
    })
}

#[derive(Debug, Deserialize)]
pub struct BeginHandoverRequest {
    /// Key id (KMS ARN, alias, or UUID) of the **next** key. Operator
    /// generates this out-of-band per the runbook.
    pub to_key_id: String,
    /// How long both keys remain trusted by the fleet. RFC §9 default
    /// is 30; we cap at 365 to catch obvious typos.
    pub transition_window_days: i64,
    /// Audit-log operator org. Required.
    pub operator_org_id: Uuid,
    /// Audit-log operator user. Required.
    pub operator_user_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct BeginHandoverResponse {
    pub event: RotationEvent,
}

/// `POST /api/internal/platform-signing/begin-handover`
pub async fn begin_handover(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<BeginHandoverRequest>,
) -> Result<Json<BeginHandoverResponse>, axum::response::Response> {
    use axum::response::IntoResponse;
    require_internal_auth(&headers).map_err(|e| e.into_response())?;
    let controller = require_controller(&state)?;
    if body.transition_window_days <= 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": {
                    "code": "INVALID_TRANSITION_WINDOW",
                    "message": "transition_window_days must be positive",
                }
            })),
        )
            .into_response());
    }
    if body.transition_window_days > MAX_TRANSITION_WINDOW_DAYS {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": {
                    "code": "TRANSITION_WINDOW_TOO_LONG",
                    "message": format!(
                        "transition_window_days exceeds {MAX_TRANSITION_WINDOW_DAYS}-day cap",
                    ),
                }
            })),
        )
            .into_response());
    }
    if body.to_key_id.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": { "code": "INVALID_KEY_ID", "message": "to_key_id is empty" }
            })),
        )
            .into_response());
    }
    let operator = operator_context(&headers, body.operator_org_id, body.operator_user_id);
    let event = controller
        .begin_handover(&operator, &body.to_key_id, Duration::days(body.transition_window_days))
        .await
        .map_err(controller_error_to_response)?;
    Ok(Json(BeginHandoverResponse { event }))
}

#[derive(Debug, Deserialize)]
pub struct RetireOldRequest {
    pub operator_org_id: Uuid,
    pub operator_user_id: Uuid,
}

/// `POST /api/internal/platform-signing/retire-old`
pub async fn retire_old(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RetireOldRequest>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    use axum::response::IntoResponse;
    require_internal_auth(&headers).map_err(|e| e.into_response())?;
    let controller = require_controller(&state)?;
    let operator = operator_context(&headers, body.operator_org_id, body.operator_user_id);
    controller.retire_old(&operator).await.map_err(controller_error_to_response)?;
    Ok(Json(json!({ "status": "retired" })))
}

#[derive(Debug, Deserialize)]
pub struct EmergencyRevokeRequest {
    /// Free-form operator-supplied reason. Logged verbatim in the
    /// audit row — keep it concise and SIEM-friendly.
    pub reason: String,
    pub operator_org_id: Uuid,
    pub operator_user_id: Uuid,
}

/// `POST /api/internal/platform-signing/emergency-revoke`
pub async fn emergency_revoke(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<EmergencyRevokeRequest>,
) -> Result<Json<serde_json::Value>, axum::response::Response> {
    use axum::response::IntoResponse;
    require_internal_auth(&headers).map_err(|e| e.into_response())?;
    let controller = require_controller(&state)?;
    if body.reason.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": {
                    "code": "REASON_REQUIRED",
                    "message": "reason is empty — emergency revocation must be justified for audit",
                }
            })),
        )
            .into_response());
    }
    let operator = operator_context(&headers, body.operator_org_id, body.operator_user_id);
    controller
        .emergency_revoke(&operator, &body.reason)
        .await
        .map_err(controller_error_to_response)?;
    Ok(Json(json!({ "status": "revoked" })))
}

#[derive(Debug, Serialize)]
pub struct PluginRotationEventsResponse {
    /// Current state-machine phase. Useful for plugin-host fleet
    /// dashboards.
    pub phase: RotationPhase,
    /// The most recent rotation event, if any. Plugin-hosts verify
    /// this against their currently-trusted root via
    /// [`mockforge_platform_signing::verify_rotation_event`] before
    /// applying it.
    pub latest: Option<RotationEvent>,
    /// Key ids the registry considers trusted right now (`[current]`
    /// during Active phase, `[from, to]` during Transitioning). The
    /// runbook's "verify the host fleet" step compares this against
    /// what each host reports via IPC Health.
    pub trusted_key_ids: Vec<String>,
}

/// `GET /api/internal/plugin-rotation-events`
///
/// Same auth as the POST endpoints — operator bastion or plugin-host
/// fleet, both carry `MOCKFORGE_INTERNAL_API_TOKEN`. mTLS upgrade is
/// the planned production posture (RFC §8.3).
pub async fn list_rotation_events(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PluginRotationEventsResponse>, axum::response::Response> {
    use axum::response::IntoResponse;
    require_internal_auth(&headers).map_err(|e| e.into_response())?;
    let controller = require_controller(&state)?;
    let phase = controller.phase().await;
    let latest = controller.last_event().await;
    let trusted_key_ids = controller.trusted_key_ids().await;
    Ok(Json(PluginRotationEventsResponse {
        phase,
        latest,
        trusted_key_ids,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_eq_distinguishes_lengths() {
        assert!(!constant_time_eq(b"ab", b"abc"));
    }

    #[test]
    fn constant_time_eq_equal_strings() {
        assert!(constant_time_eq(b"hello", b"hello"));
    }

    #[test]
    fn constant_time_eq_different_strings() {
        assert!(!constant_time_eq(b"hello", b"world"));
    }

    #[test]
    fn operator_context_extracts_ip_and_ua() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "203.0.113.42, 10.0.0.1".parse().unwrap());
        headers.insert(header::USER_AGENT, "MockForge-CLI/1.0".parse().unwrap());
        let org = Uuid::new_v4();
        let user = Uuid::new_v4();
        let op = operator_context(&headers, org, user);
        assert_eq!(op.org_id, org);
        assert_eq!(op.user_id, user);
        assert_eq!(op.ip_address.as_deref(), Some("203.0.113.42"));
        assert_eq!(op.user_agent.as_deref(), Some("MockForge-CLI/1.0"));
    }

    #[test]
    fn operator_context_tolerates_missing_headers() {
        let headers = HeaderMap::new();
        let op = operator_context(&headers, Uuid::nil(), Uuid::nil());
        assert!(op.ip_address.is_none());
        assert!(op.user_agent.is_none());
    }
}
