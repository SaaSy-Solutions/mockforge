//! Server-side Sentry initialization (feature: `sentry`).
//!
//! The UI has had opt-in Sentry via `@sentry/react` for a while
//! (`crates/mockforge-ui/ui/src/services/errorReporting.ts`), but until this
//! module the Rust server flew blind in production — panics, 5xxs, and
//! `tracing::error!` calls went to stderr and nowhere else. First production
//! incident would have been investigated by tailing Fly logs.
//!
//! Configuration is env-driven so the same binary works locally (no DSN
//! ⇒ no-op) and in production (DSN set ⇒ events captured):
//!
//! - `SENTRY_DSN` — required. Empty/unset disables capture entirely.
//! - `SENTRY_ENVIRONMENT` — defaults to "production".
//! - `SENTRY_RELEASE` — defaults to the crate version (matches the published
//!   binary). Override to tag deploys with a git SHA or Fly release ID.
//! - `SENTRY_TRACES_SAMPLE_RATE` — performance trace sampling rate (0.0–1.0).
//!   Defaults to 0.0 (errors only). Bump to ~0.05 in production when you
//!   want trace data without blowing past the Sentry quota.
//!
//! The returned guard MUST be kept alive for the lifetime of the program;
//! dropping it flushes queued events and tears down the transport.

use sentry::ClientInitGuard;

/// Initialize Sentry from env vars. Returns `None` when `SENTRY_DSN` is unset
/// or empty so binaries can safely call this unconditionally.
pub fn init() -> Option<ClientInitGuard> {
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
            // The `panic` feature wires this automatically, but being explicit
            // here documents the contract: we want panic events.
            ..Default::default()
        },
    ));

    tracing::info!(traces_sample_rate, "Sentry initialized (server-side error capture enabled)");

    Some(guard)
}
