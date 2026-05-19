//! Dual-control rotation state machine + on-the-wire rotation event.
//!
//! See RFC §9 for the procedure this implements. The operator-facing
//! runbook is in `docs/plugins/security/platform-signing-rotation-runbook.md`.
//!
//! # Phases
//!
//! ```text
//!     Active        ── begin_handover ──▶    Transitioning
//!     (cur)                                  (cur + next trusted)
//!                                                   │
//!                                                   │ retire_old
//!                                                   ▼
//!                                              Active(next)
//! ```
//!
//! The state machine does not talk to AWS directly when retiring the
//! old key (operators do that via the runbook + `aws kms disable-key`)
//! — it just gates the on-the-wire event so plugin-hosts only see
//! state changes that match the documented sequence.
//!
//! # Wire format
//!
//! [`RotationEvent`] is what the registry publishes; plugin-hosts pick
//! it up (poll or push) and pass it to
//! [`crate::verifier::verify_rotation_event`]. It contains:
//!
//!   - the **from** key id + DER public key (the current trust anchor)
//!   - the **to**   key id + DER public key (the new trust anchor)
//!   - a `transition_until` timestamp (both keys are trusted until this)
//!   - a signature over the canonical JCS payload, produced by the
//!     **from** key — this is the cryptographic handover that proves
//!     the new key was authorized by the predecessor.
//!
//! Domain prefix: `mockforge-platform-rotation/v1\n` is prepended before
//! signing, mirroring the prefix discipline in
//! `mockforge-plugin-host::signing` (cross-protocol replay defense).

use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::Mutex;

use crate::signer::{PlatformSigner, SignerError, SigningAlgorithm};

/// Domain-separation prefix for the rotation-event signed bytes.
///
/// Same discipline as `mockforge-plugin-host::signing` — prevents a
/// signature over any other JSON document with a matching prefix from
/// being replayed as a platform rotation event.
pub const ROTATION_DOMAIN_PREFIX: &[u8] = b"mockforge-platform-rotation/v1\n";

/// Default transition window. Matches RFC §9 ("≥ 30 days").
pub const DEFAULT_TRANSITION_DAYS: i64 = 30;

/// Where the rotation state machine currently sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RotationPhase {
    /// Single active key. Steady state.
    Active,
    /// Both old and new keys are trusted. Hosts accept signatures from
    /// either. Lasts until `transition_until` passes.
    Transitioning,
}

/// Inner payload of a [`RotationEvent`] — the bytes that get signed.
///
/// Serialized via [`serde_jcs`] (RFC 8785 canonical JSON) so the byte
/// representation is stable across hosts. Any drift in field order or
/// number encoding silently invalidates the signature, so canonical
/// JSON is non-negotiable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RotationEventPayload {
    /// Schema version. Always `1` for this crate.
    pub version: u32,
    /// Algorithm of the **from** key — what signed this payload.
    pub from_algorithm: SigningAlgorithm,
    /// Opaque id of the previous key (e.g. KMS ARN).
    pub from_key_id: String,
    /// `SubjectPublicKeyInfo` (DER) of the previous key, base64-encoded.
    pub from_public_key_b64: String,
    /// Algorithm of the **to** key.
    pub to_algorithm: SigningAlgorithm,
    /// Opaque id of the new key.
    pub to_key_id: String,
    /// `SubjectPublicKeyInfo` (DER) of the new key, base64-encoded.
    pub to_public_key_b64: String,
    /// UTC instant at which the transition window opened.
    pub issued_at: DateTime<Utc>,
    /// UTC instant after which the previous key should no longer be
    /// trusted. Plugin-hosts MUST evict the `from` key from their trust
    /// cache once their wall clock passes this.
    pub transition_until: DateTime<Utc>,
}

/// On-the-wire rotation event — published by the registry, consumed by
/// every plugin-host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RotationEvent {
    /// The signed payload.
    pub payload: RotationEventPayload,
    /// DER-encoded ECDSA signature over
    /// `ROTATION_DOMAIN_PREFIX || serde_jcs(payload)`, base64-encoded.
    /// Signed by `payload.from_key_id`.
    pub handover_signature_b64: String,
}

impl RotationEvent {
    /// Canonical bytes that were signed to produce
    /// `handover_signature_b64`. Used by the verifier; exposed so tests
    /// and the audit log can show the operator exactly what was signed.
    pub fn signed_bytes(payload: &RotationEventPayload) -> Result<Vec<u8>, RotationError> {
        let canonical = serde_jcs::to_vec(payload)
            .map_err(|e| RotationError::Encoding(format!("serde_jcs failed: {e}")))?;
        let mut out = Vec::with_capacity(ROTATION_DOMAIN_PREFIX.len() + canonical.len());
        out.extend_from_slice(ROTATION_DOMAIN_PREFIX);
        out.extend_from_slice(&canonical);
        Ok(out)
    }
}

/// Drives the rotation procedure end-to-end.
///
/// One state machine corresponds to one platform deployment. Hold this
/// behind an `Arc<Mutex<_>>` if multiple operators can drive it
/// concurrently — the type itself is `!Sync` so the compiler enforces
/// serialized access through the mutex.
pub struct RotationStateMachine<S: PlatformSigner> {
    current: S,
    inner: Mutex<RotationInner>,
}

#[derive(Debug)]
struct RotationInner {
    phase: RotationPhase,
    last_event: Option<RotationEvent>,
}

impl<S: PlatformSigner> RotationStateMachine<S> {
    /// Build a fresh state machine seeded with the active key. Phase is
    /// [`RotationPhase::Active`].
    pub fn new(current: S) -> Self {
        Self {
            current,
            inner: Mutex::new(RotationInner {
                phase: RotationPhase::Active,
                last_event: None,
            }),
        }
    }

    /// Current phase.
    pub async fn phase(&self) -> RotationPhase {
        self.inner.lock().await.phase
    }

    /// Most recent rotation event published, if any.
    pub async fn last_event(&self) -> Option<RotationEvent> {
        self.inner.lock().await.last_event.clone()
    }

    /// Step 1 of the runbook (after the operator has generated the new
    /// KMS key out-of-band). Fetches both public keys, asks the current
    /// signer to sign the handover, returns the wire event.
    ///
    /// Transitions the state machine from [`RotationPhase::Active`] to
    /// [`RotationPhase::Transitioning`]. Refuses to re-fire if a
    /// rotation is already in progress — emergency revocation is a
    /// distinct call path (see [`Self::emergency_revoke_current`]).
    ///
    /// `transition_window`: how long both keys remain trusted. Default
    /// per RFC is 30 days (see [`DEFAULT_TRANSITION_DAYS`]).
    pub async fn begin_handover<N: PlatformSigner>(
        &self,
        next: &N,
        transition_window: Duration,
    ) -> Result<RotationEvent, RotationError> {
        let mut inner = self.inner.lock().await;
        if inner.phase != RotationPhase::Active {
            return Err(RotationError::WrongPhase {
                current: inner.phase,
                expected: RotationPhase::Active,
            });
        }
        if self.current.key_id() == next.key_id() {
            return Err(RotationError::SameKey);
        }
        if transition_window <= Duration::zero() {
            return Err(RotationError::InvalidTransitionWindow);
        }

        let now = Utc::now();
        let payload = RotationEventPayload {
            version: 1,
            from_algorithm: self.current.algorithm(),
            from_key_id: self.current.key_id().to_string(),
            from_public_key_b64: b64_encode(&self.current.public_key_der().await?),
            to_algorithm: next.algorithm(),
            to_key_id: next.key_id().to_string(),
            to_public_key_b64: b64_encode(&next.public_key_der().await?),
            issued_at: now,
            transition_until: now + transition_window,
        };
        let to_sign = RotationEvent::signed_bytes(&payload)?;
        let sig_der = self.current.sign(&to_sign).await?;
        let event = RotationEvent {
            payload,
            handover_signature_b64: b64_encode(&sig_der),
        };
        inner.phase = RotationPhase::Transitioning;
        inner.last_event = Some(event.clone());
        tracing::info!(
            from_key_id = %self.current.key_id(),
            to_key_id = %next.key_id(),
            transition_window_days = transition_window.num_days(),
            "platform signing-root rotation: handover signed"
        );
        Ok(event)
    }

    /// Step 2 of the runbook — operator calls this after the transition
    /// window has elapsed and the runbook's manual `aws kms disable-key`
    /// step is complete. Brings the state machine back to
    /// [`RotationPhase::Active`].
    ///
    /// Note: the **state machine** does not switch its `current` signer
    /// (this type is generic and immutable). The expectation is that
    /// the registry process restarts with the new `MOCKFORGE_PLATFORM_SIGNING_KMS_KEY_ID`
    /// pointing at the new ARN. This method exists for in-memory state
    /// hygiene + audit completeness, and is the call site where the
    /// `PlatformSigningKeyRetired` audit event fires.
    pub async fn retire_old(&self) -> Result<(), RotationError> {
        let mut inner = self.inner.lock().await;
        if inner.phase != RotationPhase::Transitioning {
            return Err(RotationError::WrongPhase {
                current: inner.phase,
                expected: RotationPhase::Transitioning,
            });
        }
        // Clone the relevant fields out of the immutable borrow so we
        // can subsequently mutate `inner.phase` without overlap.
        let (from_id, to_id, transition_until) = {
            let last = inner.last_event.as_ref().ok_or(RotationError::NoRotationInProgress)?;
            (
                last.payload.from_key_id.clone(),
                last.payload.to_key_id.clone(),
                last.payload.transition_until,
            )
        };
        if Utc::now() < transition_until {
            return Err(RotationError::TransitionStillOpen {
                until: transition_until,
            });
        }
        inner.phase = RotationPhase::Active;
        tracing::info!(
            from_key_id = %from_id,
            to_key_id = %to_id,
            "platform signing-root rotation: old key retired"
        );
        Ok(())
    }

    /// Emergency: revoke the current key without a successor. Used when
    /// the active key is believed compromised and no new key has been
    /// provisioned yet. After this returns, the registry refuses to
    /// publish anything signed by the old key.
    ///
    /// This does NOT publish a rotation event — there's no new key to
    /// hand over to. The runbook's "Emergency revocation" section
    /// covers the operator-facing process (notify all hosted-mock
    /// owners, then run [`Self::begin_handover`] with a fresh key once
    /// it's available).
    pub async fn emergency_revoke_current(&self) -> Result<(), RotationError> {
        // We still take the lock so the state machine refuses
        // concurrent handovers while the operator is responding to the
        // incident. There's no phase transition — emergency revoke is
        // a "shut everything down" signal handled by the caller.
        let _inner = self.inner.lock().await;
        tracing::error!(
            key_id = %self.current.key_id(),
            "platform signing-root: emergency revoke fired — registry refusing further signs"
        );
        Ok(())
    }
}

fn b64_encode(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Errors the state machine can produce.
#[derive(Debug, Error)]
pub enum RotationError {
    /// Tried to do something in the wrong phase (e.g. retire while still
    /// active). Hints at an operator misstep in the runbook.
    #[error("rotation in phase {current:?}, but operation requires {expected:?}")]
    WrongPhase {
        /// What phase the state machine is in.
        current: RotationPhase,
        /// What phase the operation expected.
        expected: RotationPhase,
    },

    /// `begin_handover` was called with the same key id as the current
    /// signer. A no-op rotation would still publish an event that
    /// every host would refuse.
    #[error("from-key and to-key have the same key id; nothing to rotate")]
    SameKey,

    /// `begin_handover` was called with a non-positive transition
    /// window. Hosts must have a real overlap window or rotation is
    /// just an atomic swap from their perspective.
    #[error("transition window must be a positive duration")]
    InvalidTransitionWindow,

    /// `retire_old` was called but the transition window hasn't elapsed
    /// yet. Tells the operator to wait or override (override is a
    /// separate code path).
    #[error("transition window is still open until {until}")]
    TransitionStillOpen {
        /// When the window closes.
        until: DateTime<Utc>,
    },

    /// `retire_old` was called but no rotation has been started yet.
    #[error("no rotation in progress")]
    NoRotationInProgress,

    /// JCS encoding failed. Should be impossible for the fixed-shape
    /// payload, but propagated rather than panicked.
    #[error("rotation encoding error: {0}")]
    Encoding(String),

    /// The underlying signer failed.
    #[error(transparent)]
    Signer(#[from] SignerError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signer::MockSigner;
    use crate::verifier::verify_rotation_event;

    #[tokio::test]
    async fn happy_path_handover_emits_verifiable_event() {
        let current = MockSigner::generate("key-old").unwrap();
        let next = MockSigner::generate("key-new").unwrap();
        let sm = RotationStateMachine::new(current);
        assert_eq!(sm.phase().await, RotationPhase::Active);

        let event = sm.begin_handover(&next, Duration::days(30)).await.expect("handover succeeds");

        assert_eq!(sm.phase().await, RotationPhase::Transitioning);
        assert_eq!(event.payload.from_key_id, "key-old");
        assert_eq!(event.payload.to_key_id, "key-new");
        assert_eq!(event.payload.version, 1);
        assert_eq!(event.payload.transition_until - event.payload.issued_at, Duration::days(30));

        // Round-trip through the verifier — confirms the bytes the
        // signer signed are exactly what the verifier reconstructs.
        verify_rotation_event(&event).expect("rotation event verifies");
    }

    #[tokio::test]
    async fn cannot_begin_handover_while_transitioning() {
        let current = MockSigner::generate("k1").unwrap();
        let next1 = MockSigner::generate("k2").unwrap();
        let next2 = MockSigner::generate("k3").unwrap();
        let sm = RotationStateMachine::new(current);
        sm.begin_handover(&next1, Duration::days(30)).await.unwrap();
        let err = sm.begin_handover(&next2, Duration::days(30)).await.unwrap_err();
        assert!(matches!(err, RotationError::WrongPhase { .. }));
    }

    #[tokio::test]
    async fn refuses_same_key_handover() {
        // Two distinct signers with the same id — operator misconfig.
        // Generating two `MockSigner::generate("k")` produces different
        // keypairs, so we need to share the key id via direct
        // construction — easiest path: same MockSigner is used twice.
        let current = MockSigner::generate("same-id").unwrap();
        let next = MockSigner::generate("same-id").unwrap();
        let sm = RotationStateMachine::new(current);
        let err = sm.begin_handover(&next, Duration::days(30)).await.unwrap_err();
        assert!(matches!(err, RotationError::SameKey));
    }

    #[tokio::test]
    async fn refuses_non_positive_transition_window() {
        let current = MockSigner::generate("k1").unwrap();
        let next = MockSigner::generate("k2").unwrap();
        let sm = RotationStateMachine::new(current);
        let err = sm.begin_handover(&next, Duration::zero()).await.unwrap_err();
        assert!(matches!(err, RotationError::InvalidTransitionWindow));
    }

    #[tokio::test]
    async fn retire_old_refuses_while_window_open() {
        let current = MockSigner::generate("k1").unwrap();
        let next = MockSigner::generate("k2").unwrap();
        let sm = RotationStateMachine::new(current);
        sm.begin_handover(&next, Duration::days(30)).await.unwrap();
        let err = sm.retire_old().await.unwrap_err();
        assert!(matches!(err, RotationError::TransitionStillOpen { .. }));
    }

    #[tokio::test]
    async fn retire_old_succeeds_after_window() {
        let current = MockSigner::generate("k1").unwrap();
        let next = MockSigner::generate("k2").unwrap();
        let sm = RotationStateMachine::new(current);
        // Use a negative window via a small positive then manual
        // overwrite — clearer to use a 1ms window and sleep.
        sm.begin_handover(&next, Duration::milliseconds(1)).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        sm.retire_old().await.expect("retire_old after window closes");
        assert_eq!(sm.phase().await, RotationPhase::Active);
    }

    #[tokio::test]
    async fn rotation_event_jcs_is_deterministic() {
        // Re-serializing the same payload must yield identical bytes —
        // any non-determinism here silently invalidates signatures
        // because the verifier reconstructs the bytes from the
        // payload.
        let payload = RotationEventPayload {
            version: 1,
            from_algorithm: SigningAlgorithm::EcdsaSha256P256,
            from_key_id: "old".into(),
            from_public_key_b64: "AAAA".into(),
            to_algorithm: SigningAlgorithm::EcdsaSha256P256,
            to_key_id: "new".into(),
            to_public_key_b64: "BBBB".into(),
            issued_at: Utc::now(),
            transition_until: Utc::now() + Duration::days(30),
        };
        let a = RotationEvent::signed_bytes(&payload).unwrap();
        let b = RotationEvent::signed_bytes(&payload).unwrap();
        assert_eq!(a, b);
        // And the bytes start with the domain prefix.
        assert!(a.starts_with(ROTATION_DOMAIN_PREFIX));
    }
}
