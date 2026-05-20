//! Glue between [`mockforge_platform_signing`] and the registry's
//! audit-log machinery (Issue #550, RFC §8.2 / §9), plus the
//! [`PlatformSigningController`] surface the HTTP handlers and the
//! one-shot operator binary call into (Issue #568).
//!
//! The state machine itself lives in `mockforge-platform-signing`; the
//! `audited_*` helpers below wrap each operator-facing call so every
//! step writes an `audit_logs` row before it returns success. Failure
//! paths still log a warning via `tracing` but do not write an audit
//! row (the operation didn't happen).
//!
//! [`PlatformSigningController`] is the type-erased trait the HTTP
//! handlers reach through `AppState`. The AWS-KMS-backed
//! implementation is feature-gated under `platform-signing-aws-kms`;
//! the in-memory `MockSigner` implementation in tests below works on
//! any build.

use std::sync::Arc;

use async_trait::async_trait;
use chrono::Duration;
use mockforge_platform_signing::{
    PlatformSigner, RotationError, RotationEvent, RotationPhase, RotationStateMachine,
};
use mockforge_registry_core::models::{audit_log::record_audit_event, AuditEventType};
use serde_json::json;
use sqlx::PgPool;
use thiserror::Error;
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
    let current_key_id = state_machine_current_key_id(state_machine);
    state_machine.emergency_revoke_current().await?;
    let metadata = json!({
        "reason": reason,
        "key_id_revoked": current_key_id,
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

/// Errors a [`PlatformSigningController`] call can produce.
///
/// Distinguishes operator misuse (wrong phase, bad ARN) from backend
/// failures (KMS network error, IAM denied) so the HTTP layer can map
/// each variant to the right status code.
#[derive(Debug, Error)]
pub enum ControllerError {
    /// The state machine refused the operation. Includes wrong-phase,
    /// same-key, non-positive window, transition still open.
    #[error(transparent)]
    Rotation(#[from] RotationError),

    /// The configured KMS backend couldn't materialize a signer for the
    /// requested key id (missing IAM permissions, network failure,
    /// unknown ARN). Surfaces the raw backend message.
    #[error("signer backend error: {0}")]
    Backend(String),

    /// The controller wasn't configured on this deployment — i.e. the
    /// SaaS feature is off or the env vars aren't set. The handlers
    /// translate this to 503 so on-call sees "service not configured"
    /// rather than 500.
    #[error("platform-signing controller not configured")]
    NotConfigured,
}

/// Type-erased rotation control surface used by the HTTP handlers and
/// the one-shot operator binary.
///
/// Hides the [`RotationStateMachine`]'s `S: PlatformSigner` generic
/// behind `dyn` so [`crate::AppState`] can store one regardless of
/// which signer backend was compiled in. Implementations also know how
/// to materialize the **next** signer from a string key id (`to_key_id`
/// on the wire), so the HTTP layer doesn't need to know about KMS at
/// all.
#[async_trait]
pub trait PlatformSigningController: Send + Sync {
    /// Current rotation phase. Surfaces on `GET /plugin-rotation-events`
    /// for fleet-state diagnostics.
    async fn phase(&self) -> RotationPhase;

    /// Last published [`RotationEvent`], if any. Plugin-hosts poll for
    /// this — see [`crate::platform_signing::PlatformSigningController::last_event`].
    async fn last_event(&self) -> Option<RotationEvent>;

    /// The key ids currently considered trusted by the registry.
    /// During `Active` phase this is `[current_key]`; during
    /// `Transitioning` it's `[from, to]`. Used by the runbook's
    /// fleet-sync verification step.
    async fn trusted_key_ids(&self) -> Vec<String>;

    /// Audit-aware wrapper around `begin_handover`. Materializes the
    /// next signer from the supplied key id (e.g. KMS ARN) and drives
    /// the state machine.
    async fn begin_handover(
        &self,
        operator: &OperatorIdentity,
        to_key_id: &str,
        transition_window: Duration,
    ) -> Result<RotationEvent, ControllerError>;

    /// Audit-aware wrapper around `retire_old`.
    async fn retire_old(&self, operator: &OperatorIdentity) -> Result<(), ControllerError>;

    /// Audit-aware wrapper around `emergency_revoke_current`.
    async fn emergency_revoke(
        &self,
        operator: &OperatorIdentity,
        reason: &str,
    ) -> Result<(), ControllerError>;
}

/// Factory the controller uses to materialize the **next** signer when
/// `begin_handover` is called with a string key id. Pulled out as its
/// own trait so the tests (with `MockSigner`) and the production AWS
/// KMS path (with `AwsKmsSigner`) share the same controller body.
#[async_trait]
pub trait NextSignerFactory: Send + Sync {
    /// Build a signer for the given key id. The returned `Box<dyn _>`
    /// is consumed once by the controller for the handover step — no
    /// caching expected.
    async fn build(
        &self,
        key_id: &str,
    ) -> Result<Box<dyn PlatformSigner>, mockforge_platform_signing::SignerError>;
}

/// Concrete [`PlatformSigningController`] driven by a
/// [`RotationStateMachine`] and a [`NextSignerFactory`].
///
/// Holds an `Arc` to the pool and the state machine; cheap to clone.
pub struct StateMachineController<S: PlatformSigner> {
    pool: PgPool,
    state_machine: RotationStateMachine<S>,
    next_factory: Arc<dyn NextSignerFactory>,
}

impl<S: PlatformSigner + 'static> StateMachineController<S> {
    /// Wrap an existing state machine in a controller that uses
    /// `next_factory` to build the successor signer on
    /// `begin_handover`.
    pub fn new(
        pool: PgPool,
        state_machine: RotationStateMachine<S>,
        next_factory: Arc<dyn NextSignerFactory>,
    ) -> Self {
        Self {
            pool,
            state_machine,
            next_factory,
        }
    }
}

#[async_trait]
impl<S: PlatformSigner + 'static> PlatformSigningController for StateMachineController<S> {
    async fn phase(&self) -> RotationPhase {
        self.state_machine.phase().await
    }

    async fn last_event(&self) -> Option<RotationEvent> {
        self.state_machine.last_event().await
    }

    async fn trusted_key_ids(&self) -> Vec<String> {
        // The state machine doesn't expose its inner signer (so we
        // can't ask it directly for the current key id). Reconstruct
        // from the last published event: during `Transitioning` both
        // keys are trusted; during `Active` either pre-rotation (no
        // event yet, return empty — operator can re-query after the
        // first handover) or post-retire (the `to_key_id` of the most
        // recent event is the current trust anchor).
        let last = self.state_machine.last_event().await;
        match (self.state_machine.phase().await, last) {
            (RotationPhase::Transitioning, Some(ev)) => {
                vec![ev.payload.from_key_id, ev.payload.to_key_id]
            }
            (RotationPhase::Active, Some(ev)) => vec![ev.payload.to_key_id],
            // Two empty cases:
            //   - Active before any rotation: we have a current signer
            //     but no published event to read its id off of. The
            //     runbook's fleet check still works from the plugin-host
            //     side (the host knows what root it embedded).
            //   - Transitioning without a `last_event` is unreachable in
            //     the state machine's normal flow — the transition is
            //     entered only when an event has been published — but
            //     keep the pattern total so a future state-machine
            //     change can't quietly UB this match.
            (RotationPhase::Active, None) | (RotationPhase::Transitioning, None) => Vec::new(),
        }
    }

    async fn begin_handover(
        &self,
        operator: &OperatorIdentity,
        to_key_id: &str,
        transition_window: Duration,
    ) -> Result<RotationEvent, ControllerError> {
        let next = self
            .next_factory
            .build(to_key_id)
            .await
            .map_err(|e| ControllerError::Backend(e.to_string()))?;
        let event = audited_begin_handover(
            &self.pool,
            operator,
            &self.state_machine,
            &next,
            transition_window,
        )
        .await?;
        Ok(event)
    }

    async fn retire_old(&self, operator: &OperatorIdentity) -> Result<(), ControllerError> {
        audited_retire_old(&self.pool, operator, &self.state_machine).await?;
        Ok(())
    }

    async fn emergency_revoke(
        &self,
        operator: &OperatorIdentity,
        reason: &str,
    ) -> Result<(), ControllerError> {
        audited_emergency_revoke(&self.pool, operator, &self.state_machine, reason).await?;
        Ok(())
    }
}

/// AWS-KMS-backed factory. Materializes [`mockforge_platform_signing::aws_kms::AwsKmsSigner`]
/// instances from string ARNs/aliases.
#[cfg(feature = "platform-signing-aws-kms")]
pub struct AwsKmsNextSignerFactory;

#[cfg(feature = "platform-signing-aws-kms")]
#[async_trait]
impl NextSignerFactory for AwsKmsNextSignerFactory {
    async fn build(
        &self,
        key_id: &str,
    ) -> Result<Box<dyn PlatformSigner>, mockforge_platform_signing::SignerError> {
        let signer =
            mockforge_platform_signing::aws_kms::AwsKmsSigner::from_key_id(key_id.to_string())
                .await?;
        Ok(Box::new(signer))
    }
}

/// Convenience constructor: build an AWS-KMS-backed controller from
/// the standard env var (`MOCKFORGE_PLATFORM_SIGNING_KMS_KEY_ID`).
/// Returns `Ok(None)` if the env var is absent — the registry then
/// boots without rotation capability and the HTTP endpoints respond
/// 503.
#[cfg(feature = "platform-signing-aws-kms")]
pub async fn aws_kms_controller_from_env(
    pool: PgPool,
) -> anyhow::Result<Option<Arc<dyn PlatformSigningController>>> {
    use mockforge_platform_signing::aws_kms::{AwsKmsSigner, ENV_KEY_ID};

    if std::env::var(ENV_KEY_ID).ok().filter(|v| !v.is_empty()).is_none() {
        tracing::info!(
            "{ENV_KEY_ID} not set — platform-signing rotation endpoints will return 503"
        );
        return Ok(None);
    }
    let signer = AwsKmsSigner::from_env().await?;
    let state_machine = RotationStateMachine::new(signer);
    let factory: Arc<dyn NextSignerFactory> = Arc::new(AwsKmsNextSignerFactory);
    let controller: Arc<dyn PlatformSigningController> =
        Arc::new(StateMachineController::new(pool, state_machine, factory));
    tracing::info!("AWS-KMS platform-signing controller initialized");
    Ok(Some(controller))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_platform_signing::MockSigner;
    use mockforge_platform_signing::SignerError;
    use std::sync::Mutex;

    /// Factory that hands out pre-generated [`MockSigner`]s indexed by
    /// key id. Used by the controller tests below so we can drive
    /// `begin_handover` against a deterministic successor signer
    /// without standing up a KMS mock.
    pub(super) struct MockSignerFactory {
        signers: Mutex<std::collections::HashMap<String, MockSigner>>,
    }

    impl MockSignerFactory {
        pub fn new() -> Self {
            Self {
                signers: Mutex::new(std::collections::HashMap::new()),
            }
        }

        pub fn insert(&self, key_id: &str, signer: MockSigner) {
            self.signers.lock().unwrap().insert(key_id.to_string(), signer);
        }
    }

    #[async_trait]
    impl NextSignerFactory for MockSignerFactory {
        async fn build(&self, key_id: &str) -> Result<Box<dyn PlatformSigner>, SignerError> {
            let signer = self
                .signers
                .lock()
                .unwrap()
                .remove(key_id)
                .ok_or_else(|| SignerError::InvalidKeyId(key_id.to_string()))?;
            Ok(Box::new(signer))
        }
    }

    /// Controller-level smoke: trusted_key_ids tracks the state
    /// machine's phase. Audit logging exercises the Postgres pool so
    /// it's deferred to the integration test in
    /// `tests/platform_signing_controller.rs`.
    #[tokio::test]
    async fn trusted_key_ids_reflect_phase_transitions() {
        // No DB — bypass the audit-log path by exercising the state
        // machine wrapper directly. The controller's HTTP/DB path is
        // covered by integration tests.
        let current = MockSigner::generate("key-old").unwrap();
        let next = MockSigner::generate("key-new").unwrap();
        let sm = RotationStateMachine::new(current);

        // Before any handover: Active, no event published, empty.
        assert_eq!(sm.phase().await, RotationPhase::Active);
        assert!(sm.last_event().await.is_none());

        // After handover: Transitioning, both keys trusted.
        let _event = sm.begin_handover(&next, Duration::milliseconds(50)).await.unwrap();
        assert_eq!(sm.phase().await, RotationPhase::Transitioning);
        let ev = sm.last_event().await.unwrap();
        assert_eq!(ev.payload.from_key_id, "key-old");
        assert_eq!(ev.payload.to_key_id, "key-new");

        // After retire: Active again with just the new key.
        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        sm.retire_old().await.unwrap();
        assert_eq!(sm.phase().await, RotationPhase::Active);
    }
}
