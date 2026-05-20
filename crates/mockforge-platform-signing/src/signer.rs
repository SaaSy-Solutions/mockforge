//! [`PlatformSigner`] — backend-agnostic trait for the platform signing
//! root.
//!
//! Backends keep the private key behind an HSM boundary (AWS KMS today,
//! GCP KMS or a `YubiHSM` tomorrow) and only expose a `sign(...)`
//! round-trip. All test fixtures use [`MockSigner`], which holds a
//! software keypair in memory and is **not safe for production**.

use async_trait::async_trait;
use thiserror::Error;

/// What signature algorithm a [`PlatformSigner`] produces.
///
/// AWS KMS does not support Ed25519, so the platform root uses ECDSA over
/// NIST P-256 or P-384. P-256 is the default (smaller signatures, faster
/// verify on the host fleet) — P-384 is available for higher-assurance
/// deployments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SigningAlgorithm {
    /// ECDSA over NIST P-256 with SHA-256. Default.
    EcdsaSha256P256,
    /// ECDSA over NIST P-384 with SHA-384. Higher-assurance opt-in.
    EcdsaSha384P384,
}

impl SigningAlgorithm {
    /// Stable wire-form for use inside [`crate::rotation::RotationEventPayload`].
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EcdsaSha256P256 => "ecdsa-sha256-p256",
            Self::EcdsaSha384P384 => "ecdsa-sha384-p384",
        }
    }
}

/// The platform signer abstraction. One instance corresponds to one HSM-
/// hosted key.
///
/// Implementations MUST guarantee that the private key bytes never leave
/// the HSM. Only [`PlatformSigner::sign`] crosses the boundary, and only
/// the resulting signature comes back.
#[async_trait]
pub trait PlatformSigner: Send + Sync {
    /// Opaque identifier the operator uses to refer to this key (e.g. a
    /// KMS key ARN). Stable across signer instances.
    fn key_id(&self) -> &str;

    /// Signature algorithm this key produces.
    fn algorithm(&self) -> SigningAlgorithm;

    /// `SubjectPublicKeyInfo` (DER) for the key. Plugin-hosts use this
    /// to verify signatures the signer produces.
    async fn public_key_der(&self) -> Result<Vec<u8>, SignerError>;

    /// Sign the given message. The returned bytes are the DER-encoded
    /// ECDSA signature (matches what AWS KMS `Sign` returns when called
    /// with `MessageType=RAW`).
    async fn sign(&self, message: &[u8]) -> Result<Vec<u8>, SignerError>;
}

/// Errors a signer backend can produce.
#[derive(Debug, Error)]
pub enum SignerError {
    /// The backend rejected the call (e.g. `AccessDenied`, `KeyDisabled`).
    /// The string is the backend's own error message, surfaced as-is.
    #[error("signer backend error: {0}")]
    Backend(String),

    /// The configured key id was empty or malformed.
    #[error("invalid key id: {0}")]
    InvalidKeyId(String),

    /// Required environment variable missing.
    #[error("missing environment variable: {0}")]
    MissingEnv(&'static str),

    /// The backend returned a public key in an unexpected encoding.
    #[error("unexpected public-key encoding from backend: {0}")]
    UnexpectedPublicKey(String),
}

/// Forward [`PlatformSigner`] through a boxed trait object so a
/// `RotationStateMachine<Box<dyn PlatformSigner>>` is a valid concrete
/// instantiation. This is the type that `mockforge-registry-server`
/// stores in `AppState`: the binary's startup code may build either an
/// `AwsKmsSigner` (production) or a `MockSigner` (tests / OSS smoke
/// runs), and erasing to `Box<dyn _>` keeps the rest of the call sites
/// free of cargo features.
#[async_trait]
impl PlatformSigner for Box<dyn PlatformSigner> {
    fn key_id(&self) -> &str {
        (**self).key_id()
    }

    fn algorithm(&self) -> SigningAlgorithm {
        (**self).algorithm()
    }

    async fn public_key_der(&self) -> Result<Vec<u8>, SignerError> {
        (**self).public_key_der().await
    }

    async fn sign(&self, message: &[u8]) -> Result<Vec<u8>, SignerError> {
        (**self).sign(message).await
    }
}

/// Software-keypair signer for tests. **Never use in production** — the
/// private bytes live in process memory, which defeats the entire point
/// of this crate.
///
/// Uses ring's ECDSA implementation over NIST P-256 with SHA-256.
pub struct MockSigner {
    key_id: String,
    keypair: ring::signature::EcdsaKeyPair,
    public_key_der: Vec<u8>,
    algorithm: SigningAlgorithm,
}

impl std::fmt::Debug for MockSigner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockSigner")
            .field("key_id", &self.key_id)
            .field("algorithm", &self.algorithm)
            .finish_non_exhaustive()
    }
}

impl MockSigner {
    /// Generate a fresh P-256 keypair labelled with `key_id`.
    pub fn generate(key_id: impl Into<String>) -> Result<Self, SignerError> {
        let rng = ring::rand::SystemRandom::new();
        let pkcs8 = ring::signature::EcdsaKeyPair::generate_pkcs8(
            &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
            &rng,
        )
        .map_err(|e| SignerError::Backend(format!("ring pkcs8 generate failed: {e}")))?;
        let keypair = ring::signature::EcdsaKeyPair::from_pkcs8(
            &ring::signature::ECDSA_P256_SHA256_ASN1_SIGNING,
            pkcs8.as_ref(),
            &rng,
        )
        .map_err(|e| SignerError::Backend(format!("ring keypair load failed: {e}")))?;
        // ring returns the raw subject-public-key (uncompressed point);
        // wrap it in a SubjectPublicKeyInfo so the verifier can use the
        // same DER shape as the AWS KMS path.
        let raw_pub = ring::signature::KeyPair::public_key(&keypair).as_ref().to_vec();
        let public_key_der = wrap_p256_spki(&raw_pub);
        Ok(Self {
            key_id: key_id.into(),
            keypair,
            public_key_der,
            algorithm: SigningAlgorithm::EcdsaSha256P256,
        })
    }
}

#[async_trait]
impl PlatformSigner for MockSigner {
    fn key_id(&self) -> &str {
        &self.key_id
    }

    fn algorithm(&self) -> SigningAlgorithm {
        self.algorithm
    }

    async fn public_key_der(&self) -> Result<Vec<u8>, SignerError> {
        Ok(self.public_key_der.clone())
    }

    async fn sign(&self, message: &[u8]) -> Result<Vec<u8>, SignerError> {
        let rng = ring::rand::SystemRandom::new();
        let sig = self
            .keypair
            .sign(&rng, message)
            .map_err(|e| SignerError::Backend(format!("ring sign failed: {e}")))?;
        Ok(sig.as_ref().to_vec())
    }
}

/// Wrap a raw P-256 uncompressed public-key point (65 bytes, leading 0x04)
/// in a minimal `SubjectPublicKeyInfo` so the verifier sees the same DER
/// shape the AWS KMS backend returns.
fn wrap_p256_spki(raw_uncompressed_point: &[u8]) -> Vec<u8> {
    // Hand-rolled DER builder — this is fixed-shape ASN.1 (RFC 5480
    // §2.1.1) so a bespoke encoding is simpler than pulling in a full
    // DER library just for one record.
    //
    // SEQUENCE {
    //   SEQUENCE {            // AlgorithmIdentifier
    //     OID 1.2.840.10045.2.1     // id-ecPublicKey
    //     OID 1.2.840.10045.3.1.7   // secp256r1
    //   }
    //   BIT STRING { <unused = 0> || raw_uncompressed_point }
    // }
    const ALG_PREFIX: &[u8] = &[
        0x30, 0x13, // SEQUENCE (AlgorithmIdentifier), len 19
        0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, // OID id-ecPublicKey
        0x06, 0x08, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, // OID secp256r1
    ];
    let bitstring_len = 1 + raw_uncompressed_point.len(); // 1 = "unused bits" byte
    let mut bitstring = Vec::with_capacity(2 + bitstring_len);
    bitstring.push(0x03); // BIT STRING tag
    bitstring.push(bitstring_len as u8);
    bitstring.push(0x00); // 0 unused bits
    bitstring.extend_from_slice(raw_uncompressed_point);
    let body_len = ALG_PREFIX.len() + bitstring.len();
    let mut out = Vec::with_capacity(2 + body_len);
    out.push(0x30); // outer SEQUENCE
    out.push(body_len as u8);
    out.extend_from_slice(ALG_PREFIX);
    out.extend_from_slice(&bitstring);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_signer_round_trips() {
        let signer = MockSigner::generate("test-key-1").unwrap();
        assert_eq!(signer.key_id(), "test-key-1");
        assert_eq!(signer.algorithm(), SigningAlgorithm::EcdsaSha256P256);

        let msg = b"hello mockforge";
        let sig = signer.sign(msg).await.unwrap();
        let pub_der = signer.public_key_der().await.unwrap();

        // Verify with ring directly against the wrapped SPKI — the
        // verifier module does the same thing for the rotation event.
        // Strip the SPKI wrapping to get back the raw point ring expects.
        let raw_point = extract_p256_point_from_spki(&pub_der).expect("valid spki");
        let pubkey = ring::signature::UnparsedPublicKey::new(
            &ring::signature::ECDSA_P256_SHA256_ASN1,
            &raw_point,
        );
        pubkey.verify(msg, &sig).expect("signature should verify");
    }

    #[tokio::test]
    async fn mock_signer_rejects_tampered_message() {
        let signer = MockSigner::generate("test-key-2").unwrap();
        let sig = signer.sign(b"original").await.unwrap();
        let pub_der = signer.public_key_der().await.unwrap();
        let raw_point = extract_p256_point_from_spki(&pub_der).unwrap();
        let pubkey = ring::signature::UnparsedPublicKey::new(
            &ring::signature::ECDSA_P256_SHA256_ASN1,
            &raw_point,
        );
        assert!(pubkey.verify(b"tampered", &sig).is_err());
    }

    #[test]
    fn signing_algorithm_wire_form_is_stable() {
        // These strings ship over the wire inside RotationEventPayload;
        // any change is a backwards-incompatible break of every
        // plugin-host that's persisted a rotation event.
        assert_eq!(SigningAlgorithm::EcdsaSha256P256.as_str(), "ecdsa-sha256-p256");
        assert_eq!(SigningAlgorithm::EcdsaSha384P384.as_str(), "ecdsa-sha384-p384");
    }

    /// Pull the 65-byte uncompressed point back out of a minimal P-256
    /// `SubjectPublicKeyInfo` so ring can verify against it.
    fn extract_p256_point_from_spki(spki: &[u8]) -> Option<Vec<u8>> {
        // The wrap_p256_spki layout is fixed; the raw point starts at
        // a known offset. This is test-only — production verifier in
        // `verifier.rs` does the same thing more carefully.
        const HEADER_LEN: usize = 26; // outer SEQ(2) + AlgorithmIdentifier(21) + BIT STRING tag+len(2) + unused-bits(1)
        if spki.len() <= HEADER_LEN {
            return None;
        }
        Some(spki[HEADER_LEN..].to_vec())
    }
}
