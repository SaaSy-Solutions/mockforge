//! Server-side Sentry initialization (feature: `sentry`).
//!
//! Used by `mockforge-registry-server` and by `mockforge-cli` when running as
//! a hosted-mock on Fly. Shared here so the two binaries agree on env-var
//! names and defaults — drift between them would mean some deployments
//! capture errors and others don't.
//!
//! Configuration is env-driven so the same binary works locally (no DSN
//! ⇒ no-op) and in production (DSN set ⇒ events captured):
//!
//! - `SENTRY_DSN` — required. Empty/unset disables capture entirely.
//! - `SENTRY_ENVIRONMENT` — defaults to "production".
//! - `SENTRY_RELEASE` — defaults to the workspace crate version.
//! - `SENTRY_TRACES_SAMPLE_RATE` — performance trace sampling rate
//!   (0.0–1.0). Defaults to 0.0 (errors only).
//!
//! The returned guard MUST be kept alive for the lifetime of the program;
//! dropping it flushes queued events and tears down the transport.

use sentry::ClientInitGuard;

/// Initialize Sentry from env vars. Returns `None` when `SENTRY_DSN` is unset
/// or empty so binaries can safely call this unconditionally.
pub fn init_sentry() -> Option<ClientInitGuard> {
    let dsn = std::env::var("SENTRY_DSN").ok().filter(|s| !s.is_empty())?;

    let environment =
        std::env::var("SENTRY_ENVIRONMENT").unwrap_or_else(|_| "production".to_string());

    let release = std::env::var("SENTRY_RELEASE")
        .ok()
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    let traces_sample_rate = std::env::var("SENTRY_TRACES_SAMPLE_RATE")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.0);

    let guard = sentry::init((
        dsn,
        sentry::ClientOptions {
            release: Some(release.into()),
            environment: Some(environment.into()),
            traces_sample_rate,
            attach_stacktrace: true,
            ..Default::default()
        },
    ));

    tracing::info!(traces_sample_rate, "Sentry initialized (server-side error capture enabled)");

    Some(guard)
}
