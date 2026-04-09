// Some models and internal modules are not yet wired into routes.
// Suppress dead_code for the library crate during development.
#![allow(dead_code)]

//! Pillars: [Cloud]
//!
//! MockForge Plugin Registry Server — library crate.
//!
//! This crate is being extracted into a reusable library so that both the
//! multi-tenant SaaS binary (`mockforge-registry-server`) and the single-tenant
//! OSS admin server (`mockforge-ui`) can share the same domain models,
//! storage layer, handlers, and authentication middleware.
//!
//! Phase 0 of the extraction: expose the existing modules via `lib.rs`
//! without behavior changes. Later phases will introduce a `RegistryStore`
//! trait, a SQLite backend, and feature gates for SaaS-only integrations.

pub mod auth;
pub mod cache;
pub mod circuit_breaker;
pub mod config;
pub mod database;
pub mod deployment;
pub mod email;
pub mod error;
pub mod handlers;
pub mod metrics;
pub mod middleware;
/// Domain models now live in `mockforge-registry-core`. Re-exported here
/// so existing `crate::models::X` paths continue to resolve during the
/// cloud-core extraction.
pub use mockforge_registry_core::models;
pub mod pillar_tracking_init;
pub mod redis;
pub mod routes;
pub mod storage;
/// Storage trait + backends now live in `mockforge-registry-core`.
/// Re-exported so existing `crate::store::*` paths keep working.
pub use mockforge_registry_core::store;
pub mod two_factor;
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
}
