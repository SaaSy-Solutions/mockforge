//! Pure-Rust verifier for [`crate::rotation::RotationEvent`] manifests.
//!
//! Lives outside `rotation.rs` so plugin-host code that only needs to
//! **verify** rotation events doesn't have to pull in the
//! [`crate::signer::PlatformSigner`] trait or anything that depends on
//! the AWS SDK feature.

use base64::Engine;
use chrono::Utc;
use thiserror::Error;

use crate::rotation::RotationEvent;
use crate::signer::SigningAlgorithm;

/// Verify a rotation event end-to-end:
///
/// 1. Reconstruct the canonical signed bytes (domain prefix + JCS payload).
/// 2. Decode `from_public_key_b64` as a `SubjectPublicKeyInfo` and pull
///    out the algorithm OID + raw point.
/// 3. Verify the DER signature against the from-key's public bytes.
/// 4. Optional clock sanity: refuse if `issued_at > now + 5 min` (clock
///    skew) or `transition_until < issued_at` (malformed).
///
/// Returns `Ok(())` on success; any failure path returns a descriptive
/// `VerifyError`. The caller is responsible for the trust decision
/// **after** verification (i.e. "do I currently trust `from_key_id` as
/// my platform root?" — that's the cache lookup, separate from
/// crypto verification).
pub fn verify_rotation_event(event: &RotationEvent) -> Result<(), VerifyError> {
    let payload = &event.payload;

    if payload.version != 1 {
        return Err(VerifyError::UnsupportedVersion(payload.version));
    }
    if payload.transition_until <= payload.issued_at {
        return Err(VerifyError::MalformedTimestamps);
    }
    // 5-minute skew tolerance — enough for clock drift between regions
    // without letting a backdated event sneak in.
    let now = Utc::now();
    if payload.issued_at > now + chrono::Duration::minutes(5) {
        return Err(VerifyError::FutureIssuedAt);
    }

    let signed_bytes = RotationEvent::signed_bytes(payload)
        .map_err(|e| VerifyError::Reencoding(format!("{e}")))?;

    let sig_der = base64::engine::general_purpose::STANDARD
        .decode(event.handover_signature_b64.as_bytes())
        .map_err(|e| VerifyError::InvalidSignatureBase64(e.to_string()))?;

    let from_spki = base64::engine::general_purpose::STANDARD
        .decode(payload.from_public_key_b64.as_bytes())
        .map_err(|e| VerifyError::InvalidPublicKeyBase64(e.to_string()))?;

    let raw_point = extract_p256_or_p384_point(&from_spki)?;
    let alg: &dyn ring::signature::VerificationAlgorithm = match payload.from_algorithm {
        SigningAlgorithm::EcdsaSha256P256 => &ring::signature::ECDSA_P256_SHA256_ASN1,
        SigningAlgorithm::EcdsaSha384P384 => &ring::signature::ECDSA_P384_SHA384_ASN1,
    };
    let pubkey = ring::signature::UnparsedPublicKey::new(alg, raw_point);
    pubkey
        .verify(&signed_bytes, &sig_der)
        .map_err(|e| VerifyError::SignatureMismatch(format!("{e}")))?;

    Ok(())
}

/// Strip a `SubjectPublicKeyInfo` to its raw uncompressed point.
///
/// Tolerates both the P-256 (65-byte point) and P-384 (97-byte point)
/// shapes that AWS KMS and [`crate::signer::MockSigner`] both produce.
/// Does NOT validate the OID — the algorithm choice comes from the
/// signed `from_algorithm` field. (An attacker can't lie about the
/// algorithm without invalidating the signature.)
fn extract_p256_or_p384_point(spki: &[u8]) -> Result<Vec<u8>, VerifyError> {
    // Pure structural scan — find the BIT STRING and return everything
    // after its "unused bits" byte. We don't trust the absolute length
    // because AWS KMS may emit either P-256 (91-byte SPKI) or P-384
    // (120-byte SPKI), and there's no harm in accepting both.
    //
    // SubjectPublicKeyInfo ::= SEQUENCE {
    //   algorithm AlgorithmIdentifier,
    //   subjectPublicKey BIT STRING
    // }
    if spki.len() < 4 || spki[0] != 0x30 {
        return Err(VerifyError::MalformedPublicKey("not a SEQUENCE"));
    }
    let (_seq_len, after_seq_hdr) = read_der_length(&spki[1..])?;
    // First inner element: AlgorithmIdentifier (a SEQUENCE) — skip it.
    if after_seq_hdr.is_empty() || after_seq_hdr[0] != 0x30 {
        return Err(VerifyError::MalformedPublicKey("missing AlgorithmIdentifier"));
    }
    let (alg_len, after_alg_hdr) = read_der_length(&after_seq_hdr[1..])?;
    if after_alg_hdr.len() < alg_len {
        return Err(VerifyError::MalformedPublicKey("truncated AlgorithmIdentifier"));
    }
    let after_alg = &after_alg_hdr[alg_len..];
    // Next: BIT STRING.
    if after_alg.is_empty() || after_alg[0] != 0x03 {
        return Err(VerifyError::MalformedPublicKey("missing BIT STRING"));
    }
    let (bs_len, after_bs_hdr) = read_der_length(&after_alg[1..])?;
    if after_bs_hdr.len() < bs_len || bs_len == 0 {
        return Err(VerifyError::MalformedPublicKey("truncated BIT STRING"));
    }
    // First byte of a BIT STRING is the number of unused bits — must
    // be zero for a key.
    if after_bs_hdr[0] != 0 {
        return Err(VerifyError::MalformedPublicKey("BIT STRING has unused bits"));
    }
    Ok(after_bs_hdr[1..bs_len].to_vec())
}

/// Read a DER length octet stream. Returns `(length, rest_after_length_bytes)`.
fn read_der_length(input: &[u8]) -> Result<(usize, &[u8]), VerifyError> {
    if input.is_empty() {
        return Err(VerifyError::MalformedPublicKey("missing length octet"));
    }
    let first = input[0];
    if first & 0x80 == 0 {
        return Ok((first as usize, &input[1..]));
    }
    let n_octets = (first & 0x7F) as usize;
    if n_octets == 0 || n_octets > 4 || input.len() < 1 + n_octets {
        return Err(VerifyError::MalformedPublicKey("bad long-form length"));
    }
    let mut len = 0usize;
    for &byte in &input[1..=n_octets] {
        len = (len << 8) | byte as usize;
    }
    Ok((len, &input[1 + n_octets..]))
}

/// Reasons rotation-event verification can fail.
#[derive(Debug, Error)]
pub enum VerifyError {
    /// Event uses a `version` this build doesn't understand.
    #[error("unsupported rotation-event version: {0}")]
    UnsupportedVersion(u32),

    /// `transition_until <= issued_at`.
    #[error("malformed timestamps: transition_until must be after issued_at")]
    MalformedTimestamps,

    /// `issued_at` is more than 5 minutes in the future.
    #[error("issued_at is in the future beyond clock-skew tolerance")]
    FutureIssuedAt,

    /// Could not re-serialize the payload to canonical bytes.
    #[error("re-encoding failed: {0}")]
    Reencoding(String),

    /// `handover_signature_b64` was not valid base64.
    #[error("handover signature is not valid base64: {0}")]
    InvalidSignatureBase64(String),

    /// `from_public_key_b64` was not valid base64.
    #[error("from public key is not valid base64: {0}")]
    InvalidPublicKeyBase64(String),

    /// SPKI parsing failed.
    #[error("from public key is not a valid SubjectPublicKeyInfo: {0}")]
    MalformedPublicKey(&'static str),

    /// Ring rejected the signature.
    #[error("handover signature did not verify against from public key: {0}")]
    SignatureMismatch(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rotation::{RotationEvent, RotationEventPayload, RotationStateMachine};
    use crate::signer::MockSigner;
    use chrono::Duration;

    #[tokio::test]
    async fn happy_path_verifies() {
        let cur = MockSigner::generate("from").unwrap();
        let next = MockSigner::generate("to").unwrap();
        let sm = RotationStateMachine::new(cur);
        let event = sm.begin_handover(&next, Duration::days(30)).await.unwrap();
        verify_rotation_event(&event).expect("verifies");
    }

    #[tokio::test]
    async fn tampered_to_key_id_fails() {
        let cur = MockSigner::generate("from").unwrap();
        let next = MockSigner::generate("to").unwrap();
        let sm = RotationStateMachine::new(cur);
        let mut event = sm.begin_handover(&next, Duration::days(30)).await.unwrap();
        event.payload.to_key_id = "attacker-key".into();
        let err = verify_rotation_event(&event).unwrap_err();
        assert!(matches!(err, VerifyError::SignatureMismatch(_)));
    }

    #[tokio::test]
    async fn tampered_transition_until_fails() {
        let cur = MockSigner::generate("from").unwrap();
        let next = MockSigner::generate("to").unwrap();
        let sm = RotationStateMachine::new(cur);
        let mut event = sm.begin_handover(&next, Duration::days(30)).await.unwrap();
        event.payload.transition_until += Duration::days(365);
        let err = verify_rotation_event(&event).unwrap_err();
        assert!(matches!(err, VerifyError::SignatureMismatch(_)));
    }

    #[tokio::test]
    async fn replayed_signature_against_different_payload_fails() {
        // Take a valid signature, paste it onto a fresh payload — must
        // not verify. Confirms the domain prefix + JCS payload binding.
        let cur = MockSigner::generate("from").unwrap();
        let next1 = MockSigner::generate("to1").unwrap();
        let sm = RotationStateMachine::new(cur);
        let event1 = sm.begin_handover(&next1, Duration::days(30)).await.unwrap();
        // Build a parallel event using `next2` but reuse event1's
        // signature. We can't call `begin_handover` again (state
        // machine refuses) so we hand-craft a payload.
        let mut event2 = event1.clone();
        event2.payload.to_key_id = "to2".into();
        // Crucially, leave the signature from event1 in place.
        let err = verify_rotation_event(&event2).unwrap_err();
        assert!(matches!(err, VerifyError::SignatureMismatch(_)));
    }

    #[test]
    fn rejects_version_mismatch() {
        let payload = RotationEventPayload {
            version: 99,
            from_algorithm: SigningAlgorithm::EcdsaSha256P256,
            from_key_id: "a".into(),
            from_public_key_b64: "AAAA".into(),
            to_algorithm: SigningAlgorithm::EcdsaSha256P256,
            to_key_id: "b".into(),
            to_public_key_b64: "BBBB".into(),
            issued_at: Utc::now(),
            transition_until: Utc::now() + Duration::days(30),
        };
        let event = RotationEvent {
            payload,
            handover_signature_b64: "AAAA".into(),
        };
        let err = verify_rotation_event(&event).unwrap_err();
        assert!(matches!(err, VerifyError::UnsupportedVersion(99)));
    }

    #[test]
    fn rejects_inverted_timestamps() {
        let now = Utc::now();
        let payload = RotationEventPayload {
            version: 1,
            from_algorithm: SigningAlgorithm::EcdsaSha256P256,
            from_key_id: "a".into(),
            from_public_key_b64: "AAAA".into(),
            to_algorithm: SigningAlgorithm::EcdsaSha256P256,
            to_key_id: "b".into(),
            to_public_key_b64: "BBBB".into(),
            issued_at: now,
            transition_until: now - Duration::days(1),
        };
        let event = RotationEvent {
            payload,
            handover_signature_b64: "AAAA".into(),
        };
        let err = verify_rotation_event(&event).unwrap_err();
        assert!(matches!(err, VerifyError::MalformedTimestamps));
    }

    #[test]
    fn rejects_future_issued_at() {
        let now = Utc::now();
        let payload = RotationEventPayload {
            version: 1,
            from_algorithm: SigningAlgorithm::EcdsaSha256P256,
            from_key_id: "a".into(),
            from_public_key_b64: "AAAA".into(),
            to_algorithm: SigningAlgorithm::EcdsaSha256P256,
            to_key_id: "b".into(),
            to_public_key_b64: "BBBB".into(),
            issued_at: now + Duration::hours(1),
            transition_until: now + Duration::days(30),
        };
        let event = RotationEvent {
            payload,
            handover_signature_b64: "AAAA".into(),
        };
        let err = verify_rotation_event(&event).unwrap_err();
        assert!(matches!(err, VerifyError::FutureIssuedAt));
    }
}
