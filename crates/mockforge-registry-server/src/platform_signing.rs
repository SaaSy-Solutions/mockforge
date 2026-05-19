//! Glue between [`mockforge_platform_signing`] and the registry's
//! audit-log machinery (Issue #550, RFC §8.2 / §9).
//!
//! The state machine itself lives in `mockforge-platform-signing`; this
//! module wraps each operator-facing call so every step writes an
//! `audit_logs` row before it returns success. Failure paths still log
//! a warning via `tracing` but do not write an audit row (the operation
//! didn't happen).
//!
//! The actual HTTP handlers (POST /api/internal/platform-signing/...)
//! are deliberately not landed in this PR — they'll cross-conflict with
//! the in-flight #549 trust-cache PR. See the follow-up issue for the
//! plumbing.

use chrono::Duration;
use mockforge_platform_signing::{
    PlatformSigner, RotationError, RotationEvent, RotationStateMachine,
};
use mockforge_registry_core::models::{audit_log::record_audit_event, AuditEventType};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

/// Operator identity for an audited rotation step. Captures everything
/// downstream audit consumers (SIEM, compliance dashboards) need to
/// reconstruct "who did this, when, from where".
#[derive(Debug, Clone)]
pub struct OperatorIdentity {
    /// The platform operator's organization id. RFC §1 reserves a
    /// distinct "Operator" persona — the registry maps this to a
    /// dedicated org row provisioned at install time.
    pub org_id: Uuid,
    /// The individual user who triggered the rotation. Required —
    /// rotation is a high-stakes operation, anonymous initiation is
    /// not allowed.
    pub user_id: Uuid,
    /// Inbound request IP, if available.
    pub ip_address: Option<String>,
    /// Inbound `User-Agent`, if available.
    pub user_agent: Option<String>,
}

/// Begin a key handover and audit-log the action.
///
/// Wraps [`RotationStateMachine::begin_handover`]. On success, writes a
/// `platform_signing_rotation_started` row to `audit_logs` containing
/// (in `metadata`):
///   - `from_key_id`, `to_key_id`
///   - `from_algorithm`, `to_algorithm`
///   - `issued_at`, `transition_until`
///   - `from_public_key_b64`, `to_public_key_b64`
///
/// — i.e. exactly what plugin-hosts will see on the rotation-event
/// endpoint. Operators auditing a past rotation can replay the
/// verification step from this row alone.
pub async fn audited_begin_handover<C, N>(
    pool: &PgPool,
    operator: &OperatorIdentity,
    state_machine: &RotationStateMachine<C>,
    next: &N,
    transition_window: Duration,
) -> Result<RotationEvent, RotationError>
where
    C: PlatformSigner,
    N: PlatformSigner,
{
    let event = state_machine.begin_handover(next, transition_window).await?;
    let metadata = json!({
        "from_key_id": event.payload.from_key_id,
        "to_key_id": event.payload.to_key_id,
        "from_algorithm": event.payload.from_algorithm,
        "to_algorithm": event.payload.to_algorithm,
        "issued_at": event.payload.issued_at,
        "transition_until": event.payload.transition_until,
        "from_public_key_b64": event.payload.from_public_key_b64,
        "to_public_key_b64": event.payload.to_public_key_b64,
    });
    record_audit_event(
        pool,
        operator.org_id,
        Some(operator.user_id),
        AuditEventType::PlatformSigningRotationStarted,
        format!(
            "Platform signing-root rotation started: {} → {} (transition until {})",
            event.payload.from_key_id, event.payload.to_key_id, event.payload.transition_until
        ),
        Some(metadata),
        operator.ip_address.as_deref(),
        operator.user_agent.as_deref(),
    )
    .await;
    Ok(event)
}

/// Retire the previous key after the transition window closed.
///
/// Wraps [`RotationStateMachine::retire_old`]. The operator must have
/// already run `aws kms disable-key` per the runbook; this just records
/// the registry observed the retirement and updates in-memory state.
pub async fn audited_retire_old<C: PlatformSigner>(
    pool: &PgPool,
    operator: &OperatorIdentity,
    state_machine: &RotationStateMachine<C>,
) -> Result<(), RotationError> {
    let last_event = state_machine.last_event().await;
    state_machine.retire_old().await?;
    let metadata = last_event.map(|ev| {
        json!({
            "from_key_id": ev.payload.from_key_id,
            "to_key_id": ev.payload.to_key_id,
            "transition_closed_at": ev.payload.transition_until,
        })
    });
    record_audit_event(
        pool,
        operator.org_id,
        Some(operator.user_id),
        AuditEventType::PlatformSigningKeyRetired,
        "Platform signing-root: previous key retired after transition window".to_string(),
        metadata,
        operator.ip_address.as_deref(),
        operator.user_agent.as_deref(),
    )
    .await;
    Ok(())
}

/// Emergency revocation — the active key is believed compromised and
/// must stop being trusted immediately, with no successor in place yet.
///
/// Wraps [`RotationStateMachine::emergency_revoke_current`]. The
/// audit row records the operator + `reason`; the runbook covers the
/// follow-up (notify hosted-mock owners, provision a fresh key, run
/// [`audited_begin_handover`] once it's available).
pub async fn audited_emergency_revoke<C: PlatformSigner>(
    pool: &PgPool,
    operator: &OperatorIdentity,
    state_machine: &RotationStateMachine<C>,
    reason: &str,
) -> Result<(), RotationError> {
    state_machine.emergency_revoke_current().await?;
    let metadata = json!({
        "reason": reason,
        "key_id_revoked": state_machine_current_key_id(state_machine),
    });
    record_audit_event(
        pool,
        operator.org_id,
        Some(operator.user_id),
        AuditEventType::PlatformSigningKeyRevoked,
        format!("Platform signing-root: emergency revocation — {reason}"),
        Some(metadata),
        operator.ip_address.as_deref(),
        operator.user_agent.as_deref(),
    )
    .await;
    Ok(())
}

/// Tiny accessor — `RotationStateMachine` does not expose its inner
/// signer (intentional — the state machine is the only surface that
/// should drive `sign(...)` calls), but for audit-log enrichment we
/// need the key id. Pulled out so a future change to the state machine
/// API has exactly one call site to update.
fn state_machine_current_key_id<C: PlatformSigner>(
    _sm: &RotationStateMachine<C>,
) -> Option<String> {
    // Intentionally returns None today: the state machine purposely
    // does not expose the signer. The audit row is still useful (it
    // records "an emergency revoke happened, by this user, at this
    // time, for this reason"); the key id can be reconstructed from
    // the immediately-preceding `platform_signing_rotation_started`
    // row, or — in steady-state — from the registry's startup config.
    //
    // If a future change adds `current_key_id()` to the state
    // machine, swap this body for the real call.
    None
}
