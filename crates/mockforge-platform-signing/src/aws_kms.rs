//! AWS KMS-backed [`crate::signer::PlatformSigner`].
//!
//! Round-trips `Sign` and `GetPublicKey` through KMS. Private key bytes
//! never leave the service boundary — only the resulting signature and
//! the (already-public) `SubjectPublicKeyInfo` come back.
//!
//! # Key requirements (runbook also documents these)
//!
//! The KMS CMK MUST be:
//!   - `KeyUsage = SIGN_VERIFY`
//!   - `KeySpec = ECC_NIST_P256` (default) or `ECC_NIST_P384`
//!   - `Origin = AWS_KMS` for first-cut deployments, or
//!     `EXTERNAL`/`AWS_CLOUDHSM` for the FIPS 140-2 L3 upgrade path
//!
//! Ed25519 (`ECC_SECG_P256K1` / `ECC_NIST_P521` are *not* supported)
//! because AWS KMS doesn't offer it — that's the trade-off documented
//! in the crate-level README.
//!
//! # IAM permissions required
//!
//! The registry server's role needs:
//!   - `kms:Sign`              (mandatory — every rotation handover)
//!   - `kms:GetPublicKey`      (mandatory — every rotation handover)
//!   - `kms:DescribeKey`       (recommended — diagnostics + audit-log
//!     enrichment with the operator-facing alias)
//!
//! `kms:DisableKey` is NOT required by this crate — the runbook does
//! that step manually via the AWS CLI, deliberately. Granting
//! `DisableKey` to the registry role would make a registry compromise
//! immediately destructive; keeping it out-of-band means the operator
//! has to consciously perform the irreversible step.

use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_kms::{
    primitives::Blob,
    types::{KeySpec, MessageType, SigningAlgorithmSpec},
    Client as KmsClient,
};

use crate::signer::{PlatformSigner, SignerError, SigningAlgorithm};

/// Environment variable that names the active platform signing-root
/// KMS key. Required by [`AwsKmsSigner::from_env`].
pub const ENV_KEY_ID: &str = "MOCKFORGE_PLATFORM_SIGNING_KMS_KEY_ID";

/// AWS KMS-backed signer.
#[derive(Debug, Clone)]
pub struct AwsKmsSigner {
    client: KmsClient,
    key_id: String,
    algorithm: SigningAlgorithm,
}

impl AwsKmsSigner {
    /// Build a signer using the standard AWS credential chain (env vars,
    /// `~/.aws/credentials`, instance metadata) and the key id from
    /// [`ENV_KEY_ID`].
    pub async fn from_env() -> Result<Self, SignerError> {
        let key_id = std::env::var(ENV_KEY_ID).map_err(|_| SignerError::MissingEnv(ENV_KEY_ID))?;
        if key_id.trim().is_empty() {
            return Err(SignerError::InvalidKeyId(format!("{ENV_KEY_ID} is empty")));
        }
        Self::from_key_id(key_id).await
    }

    /// Build a signer for an explicit key id (ARN, alias, or key UUID).
    /// Probes the key spec via `GetPublicKey` so the configured algorithm
    /// matches what KMS will actually use at `Sign` time.
    pub async fn from_key_id(key_id: impl Into<String>) -> Result<Self, SignerError> {
        let key_id = key_id.into();
        let aws_config = aws_config::defaults(BehaviorVersion::latest()).load().await;
        let client = KmsClient::new(&aws_config);
        let algorithm = probe_algorithm(&client, &key_id).await?;
        tracing::info!(
            key_id = %key_id,
            algorithm = ?algorithm,
            "AwsKmsSigner ready"
        );
        Ok(Self {
            client,
            key_id,
            algorithm,
        })
    }

    /// Inject a pre-built client (used by integration tests with
    /// `LocalStack` or a stubbed transport).
    pub fn with_client(
        client: KmsClient,
        key_id: impl Into<String>,
        algorithm: SigningAlgorithm,
    ) -> Self {
        Self {
            client,
            key_id: key_id.into(),
            algorithm,
        }
    }
}

#[async_trait]
impl PlatformSigner for AwsKmsSigner {
    fn key_id(&self) -> &str {
        &self.key_id
    }

    fn algorithm(&self) -> SigningAlgorithm {
        self.algorithm
    }

    async fn public_key_der(&self) -> Result<Vec<u8>, SignerError> {
        let out = self
            .client
            .get_public_key()
            .key_id(&self.key_id)
            .send()
            .await
            .map_err(|e| SignerError::Backend(format!("kms GetPublicKey: {e}")))?;
        let bytes = out
            .public_key()
            .ok_or_else(|| SignerError::UnexpectedPublicKey("response had no public key".into()))?;
        Ok(bytes.as_ref().to_vec())
    }

    async fn sign(&self, message: &[u8]) -> Result<Vec<u8>, SignerError> {
        let signing_alg = match self.algorithm {
            SigningAlgorithm::EcdsaSha256P256 => SigningAlgorithmSpec::EcdsaSha256,
            SigningAlgorithm::EcdsaSha384P384 => SigningAlgorithmSpec::EcdsaSha384,
        };
        let out = self
            .client
            .sign()
            .key_id(&self.key_id)
            .message(Blob::new(message.to_vec()))
            .message_type(MessageType::Raw)
            .signing_algorithm(signing_alg)
            .send()
            .await
            .map_err(|e| SignerError::Backend(format!("kms Sign: {e}")))?;
        let sig = out
            .signature()
            .ok_or_else(|| SignerError::Backend("kms Sign returned no signature".into()))?;
        Ok(sig.as_ref().to_vec())
    }
}

/// Look up the KMS key spec and translate it into our [`SigningAlgorithm`].
/// Refuses any spec we don't have a signing algorithm pair for — better
/// to fail at boot than to discover at first `sign()` that the key is
/// unusable.
async fn probe_algorithm(
    client: &KmsClient,
    key_id: &str,
) -> Result<SigningAlgorithm, SignerError> {
    let out = client
        .get_public_key()
        .key_id(key_id)
        .send()
        .await
        .map_err(|e| SignerError::Backend(format!("kms GetPublicKey (probe): {e}")))?;
    let spec = out
        .key_spec()
        .ok_or_else(|| SignerError::Backend("kms GetPublicKey returned no KeySpec".into()))?;
    match spec {
        KeySpec::EccNistP256 => Ok(SigningAlgorithm::EcdsaSha256P256),
        KeySpec::EccNistP384 => Ok(SigningAlgorithm::EcdsaSha384P384),
        other => Err(SignerError::Backend(format!(
            "unsupported KMS KeySpec {other:?}; expected ECC_NIST_P256 or ECC_NIST_P384"
        ))),
    }
}
