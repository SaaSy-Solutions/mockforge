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
//! // Query recent metrics
//! let metrics = db.get_recent_metrics(3600).await?;
//! println!("Total requests in last hour: {}", metrics.total_requests);
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
pub async fn init(config: AnalyticsConfig) -> Result<AnalyticsDatabase> {
    let db = AnalyticsDatabase::new(&config.database_path).await?;
    db.run_migrations().await?;
    Ok(db)
}
