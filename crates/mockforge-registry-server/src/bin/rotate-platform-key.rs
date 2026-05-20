//! One-shot operator binary for driving a platform signing-root
//! rotation (Issue #568).
//!
//! Wraps the same [`PlatformSigningController::begin_handover`] call
//! the HTTP handler uses, but skips the registry process entirely —
//! handy for air-gapped deployments or for the very first rotation in
//! a brand-new cluster (before the operator has set up a JWT for the
//! HTTP path).
//!
//! Usage (from a bastion with AWS credentials + `DATABASE_URL`
//! pointing at the registry DB):
//!
//! ```text
//! cargo run -p mockforge-registry-server --bin rotate-platform-key -- \
//!   --to-key-id arn:aws:kms:us-east-1:...:key/<new> \
//!   --transition-window-days 30 \
//!   --operator-org-id <uuid> \
//!   --operator-user-id <uuid>
//! ```
//!
//! Env vars required (matching the registry deploy):
//!   - `DATABASE_URL`
//!   - `MOCKFORGE_PLATFORM_SIGNING_KMS_KEY_ID` (current/from key)
//!   - Standard AWS credential chain (env / metadata / `~/.aws/credentials`)
//!
//! Output: the published `RotationEvent` as JSON on stdout. Plugin-hosts
//! poll the registry; operators capture the JSON for incident review.

use std::process::ExitCode;

use anyhow::{Context, Result};
use chrono::Duration;
use clap::Parser;
use mockforge_registry_server::database::Database;
#[cfg(feature = "platform-signing-aws-kms")]
use mockforge_registry_server::platform_signing::{aws_kms_controller_from_env, OperatorIdentity};
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(
    name = "rotate-platform-key",
    about = "One-shot platform signing-root rotation (Issue #568)",
    long_about = "Drives the same audit-aware begin_handover the HTTP \
                  endpoint uses, against the registry's database. Skips \
                  the registry process — operators run this directly \
                  against the DB for air-gapped deployments or for the \
                  very first rotation in a new cluster."
)]
struct Args {
    /// Key id (KMS ARN, alias, or UUID) of the **next** key.
    #[arg(long)]
    to_key_id: String,

    /// How long both keys remain trusted by the fleet. Default 30
    /// (matches the runbook).
    #[arg(long, default_value_t = 30)]
    transition_window_days: i64,

    /// Audit-log: operator org id. Required for the audit row.
    #[arg(long)]
    operator_org_id: Uuid,

    /// Audit-log: operator user id. Required for the audit row.
    #[arg(long)]
    operator_user_id: Uuid,
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "rotate_platform_key=info,mockforge_registry_server=info".into()
            }),
        )
        .init();

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            tracing::error!(error = ?err, "rotation failed");
            // anyhow::Error renders the full cause chain on stderr —
            // operators reading the output through the runbook want
            // the full context.
            eprintln!("Error: {err:#}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(feature = "platform-signing-aws-kms")]
async fn run() -> Result<()> {
    let args = Args::parse();
    if args.transition_window_days <= 0 {
        anyhow::bail!("--transition-window-days must be positive");
    }

    // Use the registry's own config loader so the binary picks up the
    // same DATABASE_URL the registry process sees.
    let config = mockforge_registry_server::config::Config::load()
        .context("loading registry config (DATABASE_URL etc.)")?;
    let db = Database::connect(&config.database_url)
        .await
        .context("connecting to database")?;

    let controller = aws_kms_controller_from_env(db.pool().clone())
        .await
        .context("initializing AWS KMS platform-signing controller")?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "MOCKFORGE_PLATFORM_SIGNING_KMS_KEY_ID is not set — \
                 this binary can only rotate from an already-configured \
                 active key. Run the one-time setup in the runbook first.",
            )
        })?;

    // No request context to mine for ip/user-agent; the audit row will
    // still carry the operator's org+user id, which is the load-bearing
    // bit for SIEM dashboards.
    let operator = OperatorIdentity {
        org_id: args.operator_org_id,
        user_id: args.operator_user_id,
        ip_address: None,
        user_agent: Some(format!("rotate-platform-key/{}", env!("CARGO_PKG_VERSION"))),
    };

    let event = controller
        .begin_handover(&operator, &args.to_key_id, Duration::days(args.transition_window_days))
        .await
        .map_err(|e| anyhow::anyhow!("begin_handover failed: {e}"))?;

    // Stable, machine-readable output so operators can pipe through `jq`.
    println!("{}", serde_json::to_string_pretty(&event)?);
    tracing::info!(
        from_key_id = %event.payload.from_key_id,
        to_key_id = %event.payload.to_key_id,
        transition_until = %event.payload.transition_until,
        "rotation handover signed and recorded; plugin-hosts will pick it up on their next poll"
    );
    Ok(())
}

/// Stub run() that compiles without the AWS-KMS feature so the binary
/// at least surfaces a clear "wrong build" message instead of failing
/// to link. Operators almost always run the `saas` default; this is
/// belt-and-braces for the OSS dev who tries the binary out of
/// curiosity.
#[cfg(not(feature = "platform-signing-aws-kms"))]
async fn run() -> Result<()> {
    let _ = Args::parse();
    anyhow::bail!(
        "this build of rotate-platform-key was compiled without the \
         `platform-signing-aws-kms` feature — rebuild with \
         `--features saas` or `--features platform-signing-aws-kms`"
    )
}
