// Some models and internal modules are not yet wired into routes.
// Suppress dead_code for the library crate during development.
#![allow(dead_code)]

//! Pillars: [Cloud]
//!
//! MockForge Plugin Registry Server — library crate.
//!
//! This is the multi-tenant **SaaS binary** crate. The reusable, OSS-friendly
//! pieces — domain models, the `RegistryStore` trait + SQLite/Postgres
//! backends, auth, and TOTP/2FA helpers — live in `mockforge-registry-core`
//! and are consumed directly by single-tenant builds such as `mockforge-ui`
//! (`mockforge-registry-core` with `default-features = false, features =
//! ["sqlite"]`). This crate re-exports a few of them for path stability but
//! is *not* itself intended to be consumed as a slimmed-down library; that
//! role belongs to `mockforge-registry-core`.
//!
//! Consequently this crate requires its full integration set. The SaaS-only
//! integrations (`stripe`, `email`, `storage-s3`, `cache-redis`) are referenced
//! unconditionally throughout the crate, so the only supported build is the
//! `saas` rollup (the default). A slimmed-down build such as
//! `--no-default-features --features postgres` is not supported — see #644.
//! The guard below turns that into one actionable message instead of 30+
//! "unresolved crate" errors. If OSS consumers ever need a smaller surface of
//! *this* crate, the right move is to extend `mockforge-registry-core`, not to
//! feature-gate the SaaS server.

// This crate requires its full integration set (see the module docs above and
// #644). Keyed on the actual load-bearing integrations rather than the `saas`
// umbrella, so an explicit `--features postgres,stripe,email,storage-s3,
// cache-redis` build is allowed, and the guard naturally shrinks if any of
// these are ever properly feature-gated.
#[cfg(not(all(
    feature = "stripe",
    feature = "email",
    feature = "storage-s3",
    feature = "cache-redis"
)))]
compile_error!(
    "mockforge-registry-server is the SaaS binary and must be built with its \
     full integration set. Use the default `saas` feature (or at minimum \
     `--features postgres,stripe,email,storage-s3,cache-redis`). The `stripe`, \
     `email`, `storage-s3`, and `cache-redis` integrations are referenced \
     unconditionally, so a slimmer build does not compile. For an OSS-friendly \
     registry library, depend on `mockforge-registry-core` instead. See #644."
);

/// JWT/password auth helpers moved to `mockforge-registry-core`.
pub use mockforge_registry_core::auth;
pub mod ai;
pub mod cache;
pub mod circuit_breaker;
pub mod config;
pub mod database;
pub mod deployment;
pub mod email;
pub mod error;
pub mod fly_logs;
pub mod fly_metrics;
pub mod fly_nats;
pub mod handlers;
pub mod metrics;
pub mod middleware;
pub mod otlp_grpc;
/// Domain models now live in `mockforge-registry-core`. Re-exported here
/// so existing `crate::models::X` paths continue to resolve during the
/// cloud-core extraction.
pub use mockforge_registry_core::models;
pub mod pillar_tracking_init;
/// HSM-backed platform signing-root rotation (Issue #550, RFC §8.2 / §9).
/// Audit-aware wrapper around [`mockforge_platform_signing`].
pub mod platform_signing;
pub mod redis;
pub mod routes;
pub mod run_queue;
pub mod sso_domain;
pub mod storage;
/// Storage trait + backends now live in `mockforge-registry-core`.
/// Re-exported so existing `crate::store::*` paths keep working.
pub use mockforge_registry_core::store;
/// TOTP/2FA helpers moved to `mockforge-registry-core`.
pub use mockforge_registry_core::two_factor;
/// Validation helpers moved to `mockforge-registry-core`.
pub use mockforge_registry_core::validation;
pub mod workers;

use std::sync::Arc;

use crate::circuit_breaker::CircuitBreakerRegistry;
use crate::config::Config;
use crate::database::Database;
use crate::redis::RedisPool;
use crate::storage::PluginStorage;
use crate::store::RegistryStore;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub storage: PluginStorage,
    pub config: Config,
    pub metrics: Arc<mockforge_observability::prometheus::MetricsRegistry>,
    pub analytics_db: Option<mockforge_analytics::AnalyticsDatabase>,
    pub redis: Option<RedisPool>,
    pub circuit_breakers: CircuitBreakerRegistry,
    /// Backend-agnostic domain store. Handlers should migrate to this over
    /// time, in parallel with `db`, so that both Postgres (SaaS) and SQLite
    /// (OSS admin) backends can satisfy the same handler code.
    pub store: Arc<dyn RegistryStore>,
    /// HSM-backed platform-signing rotation control (Issue #568). `None`
    /// when the deployment didn't configure `MOCKFORGE_PLATFORM_SIGNING_KMS_KEY_ID`
    /// (OSS smoke runs, dev) — the corresponding HTTP endpoints answer
    /// 503 in that case.
    pub platform_signing: Option<Arc<dyn platform_signing::PlatformSigningController>>,
}
