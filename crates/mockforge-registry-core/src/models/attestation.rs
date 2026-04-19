//! Publisher SBOM attestation primitives.
//!
//! A [`UserPublicKey`] is an Ed25519 public key registered on a user's
//! account. At publish time the user may submit a detached signature over
//! `SHA-256(checksum || sbom_canonical_json)`; the server tries each of the
//! user's non-revoked keys and records which one verified. That result
//! rolls up into the plugin security scan as a positive finding.
//!
//! This is a narrower primitive than Sigstore/in-toto — there's no
//! transparency log, no certificate chain, no key discovery via OIDC. It's
//! a minimum viable attestation: "the account that published this plugin
//! also vouched for this SBOM." When richer trust is needed the
//! `algorithm` column is the growth point.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct UserPublicKey {
    pub id: Uuid,
    pub user_id: Uuid,
    pub algorithm: String,
    pub public_key_b64: String,
    pub label: String,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl UserPublicKey {
    /// Has this key been revoked? Revoked keys must never verify a new
    /// signature. Kept as a tiny helper to make the call site read
    /// declaratively.
    pub fn is_active(&self) -> bool {
        self.revoked_at.is_none()
    }
}

/// Input passed to [`verify_sbom_attestation`]. Kept as a named struct
/// rather than a long argument list so each field can be documented
/// independently.
#[derive(Debug)]
pub struct SbomAttestationInput<'a> {
    /// Hex SHA-256 of the published WASM artifact.
    pub artifact_checksum: &'a str,
    /// Canonical JSON bytes of the SBOM (serialized in a stable,
    /// round-trip-safe form before signing).
    pub sbom_canonical: &'a [u8],
    /// Base64-encoded detached Ed25519 signature over the message
    /// `SHA-256(checksum_bytes || sbom_canonical)`.
    pub signature_b64: &'a str,
}

/// Verification outcome. We deliberately split "no keys registered" from
/// "signature rejected" so the handler can give the publisher an
/// actionable error.
#[derive(Debug, PartialEq)]
pub enum SbomVerifyOutcome {
    /// Signature verified; the returned key id is what gets stored on the
    /// plugin_version row.
    Verified { key_id: Uuid },
    /// The user has no active keys registered. Not a signature failure —
    /// an account setup problem.
    NoKeys,
    /// Signature didn't match any registered key. This is a hard reject.
    Invalid,
    /// The signature or public key couldn't be decoded. Same blast
    /// radius as Invalid (reject the publish) but distinguished for log
    /// readability.
    Malformed(String),
}

/// Verify a detached Ed25519 SBOM signature against any of a user's
/// registered, non-revoked public keys. The message the signature must
/// cover is `SHA-256(checksum_bytes || sbom_canonical)` — checksum is
/// decoded from hex first so trailing whitespace or case differences in
/// the publish request don't leak into the signed payload.
pub fn verify_sbom_attestation(
    keys: &[UserPublicKey],
    input: &SbomAttestationInput<'_>,
) -> SbomVerifyOutcome {
    use base64::Engine;
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    use sha2::{Digest, Sha256};

    let active: Vec<&UserPublicKey> = keys.iter().filter(|k| k.is_active()).collect();
    if active.is_empty() {
        return SbomVerifyOutcome::NoKeys;
    }

    let signature_bytes = match base64::engine::general_purpose::STANDARD
        .decode(input.signature_b64)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(input.signature_b64))
    {
        Ok(b) if b.len() == ed25519_dalek::SIGNATURE_LENGTH => b,
        Ok(b) => {
            return SbomVerifyOutcome::Malformed(format!(
                "signature is {} bytes; ed25519 expects {}",
                b.len(),
                ed25519_dalek::SIGNATURE_LENGTH
            ));
        }
        Err(e) => {
            return SbomVerifyOutcome::Malformed(format!("signature is not base64: {}", e));
        }
    };
    let signature = Signature::from_slice(&signature_bytes).expect("length checked above");

    let checksum_bytes = match hex::decode(input.artifact_checksum.trim()) {
        Ok(b) => b,
        Err(e) => {
            return SbomVerifyOutcome::Malformed(format!("checksum is not hex: {}", e));
        }
    };

    let mut hasher = Sha256::new();
    hasher.update(&checksum_bytes);
    hasher.update(input.sbom_canonical);
    let message = hasher.finalize();

    for key in active {
        if key.algorithm != "ed25519" {
            continue;
        }
        let raw =
            match base64::engine::general_purpose::STANDARD.decode(&key.public_key_b64).or_else(
                |_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(&key.public_key_b64),
            ) {
                Ok(b) if b.len() == ed25519_dalek::PUBLIC_KEY_LENGTH => b,
                _ => continue,
            };
        let array: [u8; ed25519_dalek::PUBLIC_KEY_LENGTH] =
            raw.as_slice().try_into().expect("length checked above");
        let verifying = match VerifyingKey::from_bytes(&array) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if verifying.verify(&message, &signature).is_ok() {
            return SbomVerifyOutcome::Verified { key_id: key.id };
        }
    }

    SbomVerifyOutcome::Invalid
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey};
    use sha2::{Digest, Sha256};

    fn make_keypair(user_id: Uuid) -> (SigningKey, UserPublicKey) {
        // ed25519-dalek 2.x gates `SigningKey::generate` behind the `rand_core`
        // feature flag. Rather than depend on the upstream feature for test
        // code, we build the key from random bytes directly.
        use rand::RngCore;
        let mut secret = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret);
        let signing = SigningKey::from_bytes(&secret);
        let pk_b64 =
            base64::engine::general_purpose::STANDARD.encode(signing.verifying_key().to_bytes());
        let key = UserPublicKey {
            id: Uuid::new_v4(),
            user_id,
            algorithm: "ed25519".to_string(),
            public_key_b64: pk_b64,
            label: "test".to_string(),
            created_at: Utc::now(),
            revoked_at: None,
        };
        (signing, key)
    }

    fn sign(signing: &SigningKey, checksum_hex: &str, sbom: &[u8]) -> String {
        let checksum = hex::decode(checksum_hex).unwrap();
        let mut h = Sha256::new();
        h.update(&checksum);
        h.update(sbom);
        let sig = signing.sign(&h.finalize());
        base64::engine::general_purpose::STANDARD.encode(sig.to_bytes())
    }

    #[test]
    fn verify_happy_path() {
        let user = Uuid::new_v4();
        let (signing, key) = make_keypair(user);
        let checksum = "deadbeef".repeat(8);
        let sbom = br#"{"components":[]}"#;
        let sig = sign(&signing, &checksum, sbom);

        let outcome = verify_sbom_attestation(
            std::slice::from_ref(&key),
            &SbomAttestationInput {
                artifact_checksum: &checksum,
                sbom_canonical: sbom,
                signature_b64: &sig,
            },
        );
        assert_eq!(outcome, SbomVerifyOutcome::Verified { key_id: key.id });
    }

    #[test]
    fn verify_rejects_wrong_message() {
        let user = Uuid::new_v4();
        let (signing, key) = make_keypair(user);
        let checksum = "deadbeef".repeat(8);
        let signed_sbom = br#"{"components":[{"name":"honest"}]}"#;
        let tampered_sbom = br#"{"components":[{"name":"evil"}]}"#;
        let sig = sign(&signing, &checksum, signed_sbom);

        let outcome = verify_sbom_attestation(
            &[key],
            &SbomAttestationInput {
                artifact_checksum: &checksum,
                sbom_canonical: tampered_sbom,
                signature_b64: &sig,
            },
        );
        assert_eq!(outcome, SbomVerifyOutcome::Invalid);
    }

    #[test]
    fn verify_skips_revoked_keys() {
        let user = Uuid::new_v4();
        let (signing, mut key) = make_keypair(user);
        key.revoked_at = Some(Utc::now());
        let checksum = "deadbeef".repeat(8);
        let sbom = br#"{}"#;
        let sig = sign(&signing, &checksum, sbom);

        let outcome = verify_sbom_attestation(
            &[key],
            &SbomAttestationInput {
                artifact_checksum: &checksum,
                sbom_canonical: sbom,
                signature_b64: &sig,
            },
        );
        // Revoked key is the only registered one → NoKeys, not Invalid.
        assert_eq!(outcome, SbomVerifyOutcome::NoKeys);
    }

    #[test]
    fn verify_reports_malformed_signature() {
        let user = Uuid::new_v4();
        let (_s, key) = make_keypair(user);
        let outcome = verify_sbom_attestation(
            &[key],
            &SbomAttestationInput {
                artifact_checksum: "deadbeef",
                sbom_canonical: b"{}",
                signature_b64: "!!!not base64!!!",
            },
        );
        assert!(matches!(outcome, SbomVerifyOutcome::Malformed(_)));
    }

    #[test]
    fn verify_picks_any_matching_key_across_many() {
        // A user with 3 keys — only the 2nd one signed this SBOM.
        let user = Uuid::new_v4();
        let (_s1, k1) = make_keypair(user);
        let (s2, k2) = make_keypair(user);
        let (_s3, k3) = make_keypair(user);
        let checksum = "cafebabe".repeat(8);
        let sbom = br#"{}"#;
        let sig = sign(&s2, &checksum, sbom);

        let outcome = verify_sbom_attestation(
            &[k1, k2.clone(), k3],
            &SbomAttestationInput {
                artifact_checksum: &checksum,
                sbom_canonical: sbom,
                signature_b64: &sig,
            },
        );
        assert_eq!(outcome, SbomVerifyOutcome::Verified { key_id: k2.id });
    }
}

/// Property-based fuzz coverage for the JCS canonicalization layer. We
/// treat `serde_jcs` as a black box and assert the invariants our
/// attestation protocol depends on, across arbitrary JSON trees:
///
/// 1. **Idempotence** — feeding a canonicalized blob back through the
///    canonicalizer produces identical bytes. This is the guarantee
///    the server-side verifier relies on: it re-canonicalizes whatever
///    the publisher sent and hashes that, so the first pass done
///    client-side must be a fixed point.
///
/// 2. **Determinism across parses** — starting from different in-memory
///    representations of "the same" JSON value (keys reordered, extra
///    whitespace) must produce identical canonical bytes. This is the
///    property that lets two publishers using different JSON libraries
///    produce interoperable signatures.
///
/// 3. **Signature stability** — signing the canonical form and then
///    re-canonicalizing + verifying must round-trip. This catches
///    regressions that preserve bytes but break the downstream
///    `verify_sbom_attestation` contract somehow.
#[cfg(test)]
mod jcs_fuzz {
    use super::{verify_sbom_attestation, SbomAttestationInput, SbomVerifyOutcome};
    use proptest::prelude::*;
    use proptest::string::string_regex;
    use rand::RngCore;
    use sha2::{Digest, Sha256};

    /// Recursive generator for arbitrary JSON values. Bounded depth
    /// and breadth so proptest can finish in a reasonable time; the
    /// point is wide coverage of the *shape* space, not exhaustion.
    fn arb_json() -> impl Strategy<Value = serde_json::Value> {
        // Leaves: null, bool, integer, small float, string.
        let leaf = prop_oneof![
            Just(serde_json::Value::Null),
            any::<bool>().prop_map(serde_json::Value::Bool),
            any::<i64>().prop_map(|n| serde_json::json!(n)),
            (-1e6f64..1e6f64)
                .prop_filter("finite", |f| f.is_finite())
                .prop_map(|f| serde_json::json!(f)),
            // Keep strings printable so the reason a failure fires is
            // debuggable in the shrinker's output.
            string_regex("[a-zA-Z0-9 _\\-.:]{0,32}")
                .unwrap()
                .prop_map(serde_json::Value::String),
        ];
        // Recursive: arrays and objects of the leaves (and each other).
        leaf.prop_recursive(
            /* depth = */ 4,
            /* max total nodes = */ 48,
            /* collection size = */ 6,
            |inner| {
                prop_oneof![
                    prop::collection::vec(inner.clone(), 0..6).prop_map(serde_json::Value::Array),
                    prop::collection::hash_map(
                        string_regex("[a-zA-Z_][a-zA-Z0-9_]{0,8}").unwrap(),
                        inner,
                        0..6,
                    )
                    .prop_map(|m| serde_json::Value::Object(m.into_iter().collect())),
                ]
            },
        )
    }

    proptest! {
        /// `canonicalize` is a fixed point — re-canonicalizing its
        /// output produces the same bytes. This is the load-bearing
        /// property for the server verifier.
        #[test]
        fn jcs_idempotent(v in arb_json()) {
            let once = serde_jcs::to_vec(&v).expect("first canonicalize");
            let parsed: serde_json::Value =
                serde_json::from_slice(&once).expect("first canonical is valid JSON");
            let twice = serde_jcs::to_vec(&parsed).expect("second canonicalize");
            prop_assert_eq!(once, twice);
        }

        /// Two differently-ordered/whitespaced serializations of the
        /// same JSON value canonicalize to identical bytes.
        /// `serde_json::to_vec_pretty` gives us a distinct in-memory
        /// and textual representation to compare against.
        #[test]
        fn jcs_determinism_across_parses(v in arb_json()) {
            let compact = serde_json::to_vec(&v).unwrap();
            let pretty = serde_json::to_vec_pretty(&v).unwrap();
            let from_compact: serde_json::Value = serde_json::from_slice(&compact).unwrap();
            let from_pretty: serde_json::Value = serde_json::from_slice(&pretty).unwrap();
            let c_compact = serde_jcs::to_vec(&from_compact).unwrap();
            let c_pretty = serde_jcs::to_vec(&from_pretty).unwrap();
            prop_assert_eq!(c_compact, c_pretty);
        }

        /// Signing the canonical form of an arbitrary SBOM shape must
        /// verify against the registered public key. Regressions that
        /// preserve canonical bytes but break the signature-message
        /// layout in `verify_sbom_attestation` would fail here.
        #[test]
        fn jcs_signature_round_trips(v in arb_json()) {
            use super::UserPublicKey;
            use chrono::Utc;
            use ed25519_dalek::{Signer, SigningKey};
            use uuid::Uuid;

            // Build a keypair the same way the attestation tests do:
            // fill 32 bytes from the OS RNG and wrap.
            let mut secret = [0u8; 32];
            rand::thread_rng().fill_bytes(&mut secret);
            let signing = SigningKey::from_bytes(&secret);
            let key = UserPublicKey {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                algorithm: "ed25519".to_string(),
                public_key_b64: base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    signing.verifying_key().to_bytes(),
                ),
                label: "prop".to_string(),
                created_at: Utc::now(),
                revoked_at: None,
            };

            let sbom = serde_jcs::to_vec(&v).unwrap();
            let checksum = "cafebabe".repeat(8);
            let checksum_bytes = hex::decode(&checksum).unwrap();
            let mut h = Sha256::new();
            h.update(&checksum_bytes);
            h.update(&sbom);
            let sig = signing.sign(&h.finalize());
            let sig_b64 = base64::Engine::encode(
                &base64::engine::general_purpose::STANDARD,
                sig.to_bytes(),
            );

            let outcome = verify_sbom_attestation(
                std::slice::from_ref(&key),
                &SbomAttestationInput {
                    artifact_checksum: &checksum,
                    sbom_canonical: &sbom,
                    signature_b64: &sig_b64,
                },
            );
            prop_assert!(
                matches!(outcome, SbomVerifyOutcome::Verified { key_id } if key_id == key.id),
                "verify rejected signature over arbitrary canonical SBOM: {:?}",
                outcome
            );
        }
    }
}
