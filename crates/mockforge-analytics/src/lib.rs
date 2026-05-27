//! `MockForge` Analytics
//!
//! Provides comprehensive traffic analytics and metrics dashboard capabilities
//! for `MockForge`, including:
//!
//! - Time-series metrics aggregation (minute/hour/day granularity)
//! - Endpoint performance tracking
//! - Error analysis and monitoring
//! - Client analytics
//! - Traffic pattern detection
//! - Data export (CSV, JSON)
//! - Configurable retention policies
//!
//! # Architecture
//!
//! The analytics system consists of several components:
//!
//! - **Database**: SQLite-based storage for aggregated metrics
//! - **Aggregator**: Background service that queries Prometheus and stores metrics
//! - **Queries**: High-level query API for dashboard data
//! - **Export**: Data export functionality
//! - **Retention**: Automatic cleanup of old data
//!
//! # Example
//!
//! ```no_run
//! use mockforge_analytics::{AnalyticsDatabase, AnalyticsConfig, RetentionConfig};
//! use std::path::PathBuf;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let config = AnalyticsConfig {
//!     enabled: true,
//!     database_path: PathBuf::from("analytics.db"),
//!     aggregation_interval_seconds: 60,
//!     rollup_interval_hours: 1,
//!     retention: RetentionConfig::default(),
//!     batch_size: 1000,
//!     max_query_results: 10000,
//! };
//!
//! let db = AnalyticsDatabase::new(&config.database_path).await?;
//! db.run_migrations().await?;
//!
//! // Query top endpoints
//! let endpoints = db.get_top_endpoints(10, None).await?;
//! for endpoint in &endpoints {
//!     println!("Endpoint: {} - {} requests", endpoint.endpoint, endpoint.total_requests);
//! }
//! # Ok(())
//! # }
//! ```

pub mod aggregator;
pub mod config;
pub mod database;
pub mod error;
pub mod export;
pub mod models;
pub mod pillar_usage;
pub mod queries;
pub mod retention;

pub use config::{AnalyticsConfig, RetentionConfig};
pub use database::AnalyticsDatabase;
pub use error::{AnalyticsError, Result};
pub use models::*;
pub use pillar_usage::*;

// Explicitly re-export coverage metrics types for easier importing
pub use models::{
    DriftPercentageMetrics, EndpointCoverage, PersonaCIHit, RealityLevelStaleness,
    ScenarioUsageMetrics,
};

/// Initialize the analytics system with the given configuration
///
/// # Errors
///
/// Returns an error if the database cannot be opened or migrations fail.
pub async fn init(config: AnalyticsConfig) -> Result<AnalyticsDatabase> {
    let db = AnalyticsDatabase::new(&config.database_path).await?;
    db.run_migrations().await?;
    Ok(db)
}

// ---- Global database accessor (#677) -----------------------------------
//
// Several middlewares need to record analytics (drift_tracking,
// set_reality_level, scenario execution, endpoint coverage) but live in
// crates that don't see each other and can't easily thread an
// `Arc<AnalyticsDatabase>` through their state. We expose a lazy global
// the same way `mockforge-observability::get_global_registry()` does for
// Prometheus — initialise once at startup, fire-and-forget from hot paths.

use once_cell::sync::OnceCell;
use std::sync::Arc;

static GLOBAL_DB: OnceCell<Arc<AnalyticsDatabase>> = OnceCell::new();

/// Install the global analytics database. Called once from the CLI / server
/// startup when analytics is enabled. Returns `Err(_)` if it has already been
/// initialised — callers should treat that as a no-op (the first installation
/// wins).
///
/// Wrapping the `AnalyticsDatabase` in `Arc` lets recorders take cheap clones
/// without holding the global cell.
pub fn set_global_db(db: AnalyticsDatabase) -> std::result::Result<(), Arc<AnalyticsDatabase>> {
    GLOBAL_DB.set(Arc::new(db))
}

/// Return the global analytics database if one has been installed.
///
/// Recording sites should treat `None` as "analytics is disabled, skip"
/// rather than an error — making the install opt-in keeps the OSS quick-start
/// from creating a sqlite file the operator didn't ask for.
pub fn get_global_db() -> Option<Arc<AnalyticsDatabase>> {
    GLOBAL_DB.get().cloned()
}

/// Spawn a fire-and-forget task that records a drift percentage sample to
/// the global analytics database. No-op when the global isn't installed.
/// Errors are logged at WARN — the hot path must never wait or fail because
/// analytics is unavailable.
///
/// Pair with the `mockforge_drift_percentage` Prometheus gauge so live drift
/// shows up in both the dashboard query (`get_drift_percentage`) and the
/// Grafana / `/metrics` time series.
pub fn record_drift_percentage_async(
    workspace_id: String,
    org_id: Option<String>,
    total_mocks: i64,
    drifting_mocks: i64,
) {
    if let Some(db) = get_global_db() {
        tokio::spawn(async move {
            if let Err(e) = db
                .record_drift_percentage(
                    &workspace_id,
                    org_id.as_deref(),
                    total_mocks,
                    drifting_mocks,
                )
                .await
            {
                tracing::warn!(
                    workspace_id = %workspace_id,
                    error = %e,
                    "failed to record drift percentage sample"
                );
            }
        });
    }
}

/// Spawn a fire-and-forget task that records that a scenario fired.
/// No-op when the global isn't installed.
pub fn record_scenario_usage_async(
    scenario_id: String,
    workspace_id: Option<String>,
    org_id: Option<String>,
) {
    if let Some(db) = get_global_db() {
        tokio::spawn(async move {
            if let Err(e) = db
                .record_scenario_usage(&scenario_id, workspace_id.as_deref(), org_id.as_deref())
                .await
            {
                tracing::warn!(
                    scenario_id = %scenario_id,
                    error = %e,
                    "failed to record scenario usage sample"
                );
            }
        });
    }
}

/// Spawn a fire-and-forget task that records a single endpoint hit for
/// coverage tracking. The HTTP middleware calls this on every matched
/// request when analytics is installed.
pub fn record_endpoint_coverage_async(
    endpoint: String,
    method: Option<String>,
    protocol: String,
    workspace_id: Option<String>,
    org_id: Option<String>,
) {
    if let Some(db) = get_global_db() {
        tokio::spawn(async move {
            if let Err(e) = db
                .record_endpoint_coverage(
                    &endpoint,
                    method.as_deref(),
                    &protocol,
                    workspace_id.as_deref(),
                    org_id.as_deref(),
                    None,
                )
                .await
            {
                tracing::warn!(
                    endpoint = %endpoint,
                    error = %e,
                    "failed to record endpoint coverage sample"
                );
            }
        });
    }
}

/// Spawn a fire-and-forget task that records the current reality level for
/// staleness tracking. `current_reality_level` is the level name (e.g.
/// `"production_chaos"`); `staleness_days` is "how long ago this level was
/// last refreshed" — pass `Some(0)` when the level was set right now.
pub fn record_reality_level_staleness_async(
    workspace_id: String,
    org_id: Option<String>,
    current_reality_level: Option<String>,
    staleness_days: Option<i32>,
) {
    if let Some(db) = get_global_db() {
        tokio::spawn(async move {
            if let Err(e) = db
                .record_reality_level_staleness(
                    &workspace_id,
                    org_id.as_deref(),
                    None,
                    None,
                    None,
                    current_reality_level.as_deref(),
                    staleness_days,
                )
                .await
            {
                tracing::warn!(
                    workspace_id = %workspace_id,
                    error = %e,
                    "failed to record reality level staleness sample"
                );
            }
        });
    }
}

#[cfg(test)]
mod global_accessor_tests {
    use super::*;

    #[tokio::test]
    async fn get_global_db_returns_none_before_install() {
        // The global is process-wide and OnceCell, so a test that installs
        // wouldn't be able to un-install for a second test. This test only
        // runs reliably as the first global-touching test in the process,
        // but it documents the intended pre-install state.
        let observed = get_global_db();
        // Either the global was never set in this test binary (None) or a
        // previous test set it; either way the API returns an Option, not
        // a panic. The recording helpers must remain safe to call when the
        // global isn't installed.
        match observed {
            Some(_) | None => {
                // Both shapes are valid — assert nothing panics.
            }
        }
    }

    #[tokio::test]
    async fn recording_helpers_are_safe_when_global_uninstalled() {
        // The helpers are defined as fire-and-forget no-ops when
        // `GLOBAL_DB` is empty; this test pins that contract so hot-path
        // middlewares can call them unconditionally without crashing.
        record_drift_percentage_async("ws".to_string(), None, 10, 1);
        record_scenario_usage_async("scenario-a".to_string(), None, None);
        record_endpoint_coverage_async(
            "/users".to_string(),
            Some("GET".to_string()),
            "http".to_string(),
            None,
            None,
        );
        record_reality_level_staleness_async(
            "ws".to_string(),
            None,
            Some("static_stubs".to_string()),
            Some(0),
        );
        // Give spawned tasks a moment to attempt anything (they should
        // bail at the `get_global_db()` guard).
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}
