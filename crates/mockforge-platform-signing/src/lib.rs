//! HSM-backed platform signing-root for `MockForge`.
//!
//! Implements RFC §8.2 (kill-switch signing) and §9 (rotation procedure) of
//! the cloud trust & permissions RFC.
//!
//! # Layout
//!
//! - [`signer`] — [`PlatformSigner`] trait + an in-memory [`MockSigner`]
//!   for tests.
//! - [`aws_kms`] (feature: `aws-kms`) — production [`AwsKmsSigner`] that
//!   round-trips signatures through AWS KMS so private bytes never leave
//!   the service boundary.
//! - [`rotation`] — [`RotationStateMachine`] + [`RotationEvent`]; how the
//!   operator drives a key handover and how the wire-format manifest is
//!   built.
//! - [`verifier`] — pure-Rust verifier for `RotationEvent` manifests, used
//!   by plugin-hosts to decide whether to trust a newly-rotated platform
//!   key. Does not need the AWS SDK.
//!
//! # Quick start (operator-facing)
//!
//! ```no_run
//! # #[cfg(feature = "aws-kms")]
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! use mockforge_platform_signing::aws_kms::AwsKmsSigner;
//! use mockforge_platform_signing::rotation::RotationStateMachine;
//! use chrono::Duration;
//!
//! // Active key — `MOCKFORGE_PLATFORM_SIGNING_KMS_KEY_ID`.
//! let current = AwsKmsSigner::from_env().await?;
//! // New key — generated out-of-band via the runbook.
//! let next = AwsKmsSigner::from_key_id("arn:aws:kms:us-east-1:...:key/new").await?;
//!
//! let mut sm = RotationStateMachine::new(current);
//! let event = sm.begin_handover(&next, Duration::days(30)).await?;
//! // `event` is the wire manifest the registry publishes; every host
//! // verifies it before trusting `next.key_id()`.
//! # Ok(()) }
//! ```
//!
//! See `docs/plugins/security/platform-signing-rotation-runbook.md`
//! for the end-to-end runbook (this crate is the machinery; the runbook
//! is the process).

#![warn(missing_docs)]

pub mod rotation;
pub mod signer;
pub mod verifier;

#[cfg(feature = "aws-kms")]
pub mod aws_kms;

pub use rotation::{
    RotationError, RotationEvent, RotationEventPayload, RotationPhase, RotationStateMachine,
};
pub use signer::{MockSigner, PlatformSigner, SignerError, SigningAlgorithm};
pub use verifier::{verify_rotation_event, VerifyError};
