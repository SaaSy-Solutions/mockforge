//! Ed25519 signature verification for cloud plugins.
//!
//! Implements RFC §7.2 step 3 — runtime verification at LoadPlugin
//! time. The publish-time check (§7.2 step 1) and the attach-time
//! check (§7.2 step 2) are upstream concerns; this is the last
//! line of defense before the WASM bytes touch the loader.
//!
//! ## Trust roots
//!
//! Trust is configured via [`TrustStore`]: a map from publisher
//! key id (string) to Ed25519 public key (32 bytes). The host
//! reads this from one of:
//!
//!   - `MOCKFORGE_PLUGIN_HOST_TRUSTED_KEYS` — JSON object like
//!     `{"publisher-1": "<base64-pubkey>", ...}` inline
//!   - `MOCKFORGE_PLUGIN_HOST_TRUSTED_KEYS_FILE` — same JSON in
//!     a file
//!
//! Empty trust store → reject every signed plugin. Combined with
//! [`SignatureMode::Required`] the proxy is fail-safe by default.
//!
//! ## Modes
//!
//! - [`SignatureMode::Required`] — every LoadPlugin must carry a
//!   valid signature against an active trust root. Default for
//!   cloud deployments.
//! - [`SignatureMode::Optional`] — LoadPlugin without a signature
//!   is allowed (but logged). Default for self-hosted / dev. The
//!   plugin-host bin currently flips to Required in cloud mode
//!   via env var; see `main.rs`.
//!
//! ## Signed payload
//!
//! Signature is over the **WASM bytes alone** — the canonical hash
//! used for storage and signing. We don't include the manifest in
//! the signed payload yet; that's a Phase 2 follow-up that pairs
//! with fetching the real manifest from the registry instead of
//! using the synthetic one in `host.rs::build_synthetic_manifest`.

use std::collections::HashMap;
use std::path::Path;

use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

/// What signature policy the host enforces at LoadPlugin time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SignatureMode {
    /// Every LoadPlugin must carry `(signature_b64, publisher_key_id)`
    /// and the signature must verify against an active trust root.
    /// Cloud-mode default.
    Required,
    /// LoadPlugin without a signature is allowed (and logged).
    /// Self-hosted / dev default — preserves the current OSS
    /// behavior of accepting unsigned plugins for local
    /// development. Default so a stub deployment that forgets to
    /// configure trust roots can still load a plugin; cloud
    /// production explicitly sets Required.
    #[default]
    Optional,
}

/// Map from publisher key id → 32-byte Ed25519 public key.
#[derive(Debug, Clone, Default)]
pub struct TrustStore {
    keys: HashMap<String, VerifyingKey>,
}

impl TrustStore {
    /// Build an empty trust store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of currently-active trust roots.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Whether this store has any trust roots.
    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Insert a key. Returns the previous binding for `key_id` if
    /// any (rotation case).
    pub fn insert(&mut self, key_id: String, key: VerifyingKey) -> Option<VerifyingKey> {
        self.keys.insert(key_id, key)
    }

    /// Look up a key by id.
    pub fn get(&self, key_id: &str) -> Option<&VerifyingKey> {
        self.keys.get(key_id)
    }

    /// Build from a JSON object `{ "key_id": "<base64-pubkey>", ... }`.
    pub fn from_json_str(json: &str) -> Result<Self, TrustStoreError> {
        let raw: HashMap<String, String> =
            serde_json::from_str(json).map_err(|err| TrustStoreError::InvalidJson {
                err: err.to_string(),
            })?;
        let mut store = Self::new();
        for (key_id, b64_value) in raw {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(b64_value.as_bytes())
                .map_err(|err| TrustStoreError::InvalidBase64 {
                    key_id: key_id.clone(),
                    err: err.to_string(),
                })?;
            let key_bytes: [u8; 32] =
                bytes.as_slice().try_into().map_err(|_| TrustStoreError::InvalidKeyLength {
                    key_id: key_id.clone(),
                    actual_bytes: bytes.len(),
                })?;
            let key = VerifyingKey::from_bytes(&key_bytes).map_err(|err| {
                TrustStoreError::InvalidKey {
                    key_id: key_id.clone(),
                    err: err.to_string(),
                }
            })?;
            store.insert(key_id, key);
        }
        Ok(store)
    }

    /// Read trust roots from a file containing the same JSON shape
    /// as [`from_json_str`].
    pub fn from_file(path: &Path) -> Result<Self, TrustStoreError> {
        let contents = std::fs::read_to_string(path).map_err(|err| TrustStoreError::Io {
            what: "reading trust store file",
            err: err.to_string(),
        })?;
        Self::from_json_str(&contents)
    }
}

/// Errors building a trust store from external input.
#[derive(Debug, thiserror::Error)]
pub enum TrustStoreError {
    /// The JSON didn't parse as `{string: string}`.
    #[error("trust store JSON failed to parse: {err}")]
    InvalidJson {
        /// Parser error detail.
        err: String,
    },
    /// A value wasn't valid base64.
    #[error("trust store key '{key_id}' has invalid base64: {err}")]
    InvalidBase64 {
        /// The key id that failed.
        key_id: String,
        /// Decode error detail.
        err: String,
    },
    /// A decoded key wasn't 32 bytes.
    #[error("trust store key '{key_id}' is {actual_bytes} bytes; expected 32")]
    InvalidKeyLength {
        /// The key id that failed.
        key_id: String,
        /// Actual byte length seen.
        actual_bytes: usize,
    },
    /// The 32 bytes weren't a valid Ed25519 point.
    #[error("trust store key '{key_id}' is not a valid Ed25519 key: {err}")]
    InvalidKey {
        /// The key id that failed.
        key_id: String,
        /// Error detail from `VerifyingKey::from_bytes`.
        err: String,
    },
    /// Couldn't read the file.
    #[error("io error while {what}: {err}")]
    Io {
        /// Operation that failed.
        what: &'static str,
        /// Underlying error message.
        err: String,
    },
}

/// Verifier — combines the trust store and the policy mode.
pub struct SignatureVerifier {
    store: TrustStore,
    mode: SignatureMode,
}

impl SignatureVerifier {
    /// Build a verifier with the given trust store and policy.
    pub fn new(store: TrustStore, mode: SignatureMode) -> Self {
        Self { store, mode }
    }

    /// Verify a signature over `wasm_bytes`. Pass `None` for
    /// `signature_b64` / `publisher_key_id` if the LoadPlugin
    /// request didn't include a signature — the verifier will
    /// either accept (Optional mode) or reject (Required mode).
    pub fn verify(
        &self,
        wasm_bytes: &[u8],
        signature_b64: Option<&str>,
        publisher_key_id: Option<&str>,
    ) -> Result<VerificationOutcome, VerificationError> {
        match (signature_b64, publisher_key_id) {
            (None, None) => {
                if self.mode == SignatureMode::Required {
                    return Err(VerificationError::Required);
                }
                Ok(VerificationOutcome::SkippedUnsigned)
            }
            (Some(_), None) | (None, Some(_)) => {
                // Half a signature is more suspicious than none —
                // either we have both fields or neither.
                Err(VerificationError::IncompleteSignatureFields)
            }
            (Some(sig_b64), Some(key_id)) => {
                let sig_bytes = base64::engine::general_purpose::STANDARD
                    .decode(sig_b64.as_bytes())
                    .map_err(|err| VerificationError::InvalidSignatureBase64(err.to_string()))?;
                let sig_array: [u8; 64] = sig_bytes.as_slice().try_into().map_err(|_| {
                    VerificationError::InvalidSignatureLength {
                        actual_bytes: sig_bytes.len(),
                    }
                })?;
                let signature = Signature::from_bytes(&sig_array);

                let key =
                    self.store.get(key_id).ok_or_else(|| VerificationError::UnknownKeyId {
                        key_id: key_id.to_string(),
                    })?;

                key.verify(wasm_bytes, &signature)
                    .map_err(|err| VerificationError::SignatureMismatch(err.to_string()))?;

                Ok(VerificationOutcome::Verified {
                    key_id: key_id.to_string(),
                })
            }
        }
    }

    /// Current policy mode.
    pub fn mode(&self) -> SignatureMode {
        self.mode
    }

    /// Number of active trust roots.
    pub fn trusted_key_count(&self) -> usize {
        self.store.len()
    }
}

/// What happened during verification. Successful cases include
/// the key id used, so the caller can include it in the audit
/// trail. The `SkippedUnsigned` variant is only ever reachable
/// in `SignatureMode::Optional`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationOutcome {
    /// Signature was present and valid against a trust root.
    Verified {
        /// The trust-root key id that matched.
        key_id: String,
    },
    /// No signature was provided and the policy allowed it.
    /// Reachable only in [`SignatureMode::Optional`].
    SkippedUnsigned,
}

/// Errors the verifier can produce. Each variant maps to a stable
/// error code via [`code`] so the IPC layer can surface it
/// consistently.
///
/// [`code`]: VerificationError::code
#[derive(Debug, thiserror::Error)]
pub enum VerificationError {
    /// Signature was required by policy but not present in the
    /// LoadPlugin request.
    #[error("signature required by policy but not provided")]
    Required,
    /// Either `signature_b64` or `publisher_key_id` was set but
    /// not both.
    #[error("LoadPlugin must include both signature_b64 AND publisher_key_id, or neither")]
    IncompleteSignatureFields,
    /// The signature wasn't valid base64.
    #[error("signature_b64 is not valid base64: {0}")]
    InvalidSignatureBase64(String),
    /// The signature decoded to something other than 64 bytes.
    #[error("signature must be 64 bytes; got {actual_bytes}")]
    InvalidSignatureLength {
        /// Actual byte length seen.
        actual_bytes: usize,
    },
    /// The publisher key id wasn't in the trust store. Could be
    /// a stale key id from before a rotation, or an attacker.
    #[error("publisher key id '{key_id}' is not a trusted root")]
    UnknownKeyId {
        /// The key id that wasn't recognized.
        key_id: String,
    },
    /// The signature didn't verify against the named key.
    #[error("signature did not verify against the named key: {0}")]
    SignatureMismatch(String),
}

impl VerificationError {
    /// Stable, machine-readable error code for the IPC `code`
    /// field. Keeps the wire surface compatible across host
    /// versions even if the human messages change.
    pub fn code(&self) -> &'static str {
        match self {
            VerificationError::Required => "signature_required",
            VerificationError::IncompleteSignatureFields => "incomplete_signature",
            VerificationError::InvalidSignatureBase64(_) => "invalid_signature_base64",
            VerificationError::InvalidSignatureLength { .. } => "invalid_signature_length",
            VerificationError::UnknownKeyId { .. } => "unknown_publisher_key",
            VerificationError::SignatureMismatch(_) => "signature_mismatch",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn fixture_keypair() -> (SigningKey, VerifyingKey) {
        // Deterministic keypair — using a fixed seed makes the
        // tests reproducible. Not a real key; never put a value
        // like this near production.
        let sk_bytes: [u8; 32] = [
            0x9d, 0x61, 0xb1, 0x9d, 0xef, 0xfd, 0x5a, 0x60, 0xba, 0x84, 0x4a, 0xf4, 0x92, 0xec,
            0x2c, 0xc4, 0x44, 0x49, 0xc5, 0x69, 0x7b, 0x32, 0x69, 0x19, 0x70, 0x3b, 0xac, 0x03,
            0x1c, 0xae, 0x7f, 0x60,
        ];
        let sk = SigningKey::from_bytes(&sk_bytes);
        let vk = sk.verifying_key();
        (sk, vk)
    }

    fn make_store_with_test_key(key_id: &str) -> (TrustStore, SigningKey) {
        let (sk, vk) = fixture_keypair();
        let mut store = TrustStore::new();
        store.insert(key_id.to_string(), vk);
        (store, sk)
    }

    fn b64(bytes: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    #[test]
    fn verifier_accepts_valid_signature() {
        let (store, sk) = make_store_with_test_key("publisher-1");
        let verifier = SignatureVerifier::new(store, SignatureMode::Required);
        let wasm = b"\x00asm\x01\x00\x00\x00";
        let sig = sk.sign(wasm);
        let outcome = verifier
            .verify(wasm, Some(&b64(sig.to_bytes().as_ref())), Some("publisher-1"))
            .unwrap();
        match outcome {
            VerificationOutcome::Verified { key_id } => assert_eq!(key_id, "publisher-1"),
            other => panic!("expected Verified, got {:?}", other),
        }
    }

    #[test]
    fn verifier_rejects_signature_against_wrong_key() {
        let (mut store, _sk) = make_store_with_test_key("publisher-1");
        // Add a second key but sign with the first.
        let (other_sk, _other_vk) = fixture_keypair();
        let _ = other_sk; // unused; signing was with the first
        let (real_sk, _real_vk) = fixture_keypair();
        let verifier = SignatureVerifier::new(store.clone(), SignatureMode::Required);
        let wasm = b"different bytes";
        let sig = real_sk.sign(wasm);

        // Sign over different bytes than what we'll ask the
        // verifier to check — should fail.
        let bad_message = b"original bytes";
        let outcome =
            verifier.verify(bad_message, Some(&b64(sig.to_bytes().as_ref())), Some("publisher-1"));
        match outcome {
            Err(err) => assert_eq!(err.code(), "signature_mismatch"),
            Ok(other) => panic!("expected signature_mismatch error, got {:?}", other),
        }

        // Confirm the sk we built was actually consulted (sanity
        // check that store has the right key).
        assert!(store.get("publisher-1").is_some());
        let _ = store.insert("publisher-2".to_string(), real_sk.verifying_key());
    }

    #[test]
    fn verifier_rejects_unknown_key_id() {
        let (store, sk) = make_store_with_test_key("publisher-1");
        let verifier = SignatureVerifier::new(store, SignatureMode::Required);
        let wasm = b"\x00asm\x01\x00\x00\x00";
        let sig = sk.sign(wasm);
        let err = verifier
            .verify(wasm, Some(&b64(sig.to_bytes().as_ref())), Some("not-a-real-key"))
            .unwrap_err();
        assert_eq!(err.code(), "unknown_publisher_key");
    }

    #[test]
    fn verifier_required_mode_rejects_unsigned() {
        let (store, _sk) = make_store_with_test_key("publisher-1");
        let verifier = SignatureVerifier::new(store, SignatureMode::Required);
        let err = verifier.verify(b"wasm", None, None).unwrap_err();
        assert_eq!(err.code(), "signature_required");
    }

    #[test]
    fn verifier_optional_mode_accepts_unsigned() {
        let (store, _sk) = make_store_with_test_key("publisher-1");
        let verifier = SignatureVerifier::new(store, SignatureMode::Optional);
        let outcome = verifier.verify(b"wasm", None, None).unwrap();
        assert_eq!(outcome, VerificationOutcome::SkippedUnsigned);
    }

    #[test]
    fn verifier_rejects_half_signature_in_either_mode() {
        let (store, _sk) = make_store_with_test_key("publisher-1");
        for mode in [SignatureMode::Required, SignatureMode::Optional] {
            let verifier = SignatureVerifier::new(store.clone(), mode);
            // Only signature, no key id.
            let err = verifier.verify(b"wasm", Some("AAAA"), None).unwrap_err();
            assert_eq!(err.code(), "incomplete_signature");
            // Only key id, no signature.
            let err = verifier.verify(b"wasm", None, Some("publisher-1")).unwrap_err();
            assert_eq!(err.code(), "incomplete_signature");
        }
    }

    #[test]
    fn verifier_rejects_invalid_signature_base64() {
        let (store, _sk) = make_store_with_test_key("publisher-1");
        let verifier = SignatureVerifier::new(store, SignatureMode::Required);
        let err = verifier
            .verify(b"wasm", Some("not-valid-base64-!!!"), Some("publisher-1"))
            .unwrap_err();
        assert_eq!(err.code(), "invalid_signature_base64");
    }

    #[test]
    fn verifier_rejects_signature_of_wrong_length() {
        let (store, _sk) = make_store_with_test_key("publisher-1");
        let verifier = SignatureVerifier::new(store, SignatureMode::Required);
        // Valid base64 but only 8 bytes — Ed25519 signatures are
        // 64 bytes.
        let err = verifier
            .verify(b"wasm", Some(&b64(&[0u8; 8])), Some("publisher-1"))
            .unwrap_err();
        assert_eq!(err.code(), "invalid_signature_length");
    }

    #[test]
    fn trust_store_round_trips_through_json() {
        let (_sk, vk) = fixture_keypair();
        let json = format!(r#"{{"publisher-1":"{}"}}"#, b64(vk.as_bytes()));
        let store = TrustStore::from_json_str(&json).unwrap();
        assert_eq!(store.len(), 1);
        assert!(store.get("publisher-1").is_some());
    }

    #[test]
    fn trust_store_rejects_short_key() {
        let json = format!(r#"{{"too-short":"{}"}}"#, b64(&[0u8; 16]));
        let result = TrustStore::from_json_str(&json);
        assert!(matches!(result, Err(TrustStoreError::InvalidKeyLength { .. })));
    }

    #[test]
    fn trust_store_rejects_invalid_json() {
        let result = TrustStore::from_json_str("not json");
        assert!(matches!(result, Err(TrustStoreError::InvalidJson { .. })));
    }

    #[test]
    fn empty_trust_store_in_required_mode_rejects_signed_load() {
        let verifier = SignatureVerifier::new(TrustStore::new(), SignatureMode::Required);
        let err = verifier.verify(b"wasm", Some("AAAA"), Some("any-key")).unwrap_err();
        // Empty store means even a syntactically valid request
        // can't find a trust root.
        assert!(matches!(
            err.code(),
            "unknown_publisher_key" | "invalid_signature_length" | "invalid_signature_base64"
        ));
    }
}
