//! Configuration types for the analytics system

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Analytics system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsConfig {
    /// Whether analytics is enabled
    pub enabled: bool,

    /// Path to the analytics database file
    pub database_path: PathBuf,

    /// How often to aggregate metrics from Prometheus (in seconds)
    #[serde(default = "default_aggregation_interval")]
    pub aggregation_interval_seconds: u64,

    /// How often to roll up minute data to hour data (in hours)
    #[serde(default = "default_rollup_interval")]
    pub rollup_interval_hours: u64,

    /// Data retention policies
    #[serde(default)]
    pub retention: RetentionConfig,

    /// Batch size for database operations
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Maximum number of results to return from queries
    #[serde(default = "default_max_query_results")]
    pub max_query_results: usize,
}

/// Data retention configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionConfig {
    /// How long to keep minute-level aggregates (in days)
    #[serde(default = "default_minute_retention")]
    pub minute_aggregates_days: u32,

    /// How long to keep hour-level aggregates (in days)
    #[serde(default = "default_hour_retention")]
    pub hour_aggregates_days: u32,

    /// How long to keep day-level aggregates (in days)
    #[serde(default = "default_day_retention")]
    pub day_aggregates_days: u32,

    /// How long to keep error events (in days)
    #[serde(default = "default_error_retention")]
    pub error_events_days: u32,

    /// How long to keep client analytics (in days)
    #[serde(default = "default_client_retention")]
    pub client_analytics_days: u32,

    /// How long to keep traffic patterns (in days)
    #[serde(default = "default_traffic_retention")]
    pub traffic_patterns_days: u32,

    /// How long to keep analytics snapshots (in days)
    #[serde(default = "default_snapshot_retention")]
    pub snapshots_days: u32,

    /// How often to run cleanup (in hours)
    #[serde(default = "default_cleanup_interval")]
    pub cleanup_interval_hours: u32,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            database_path: PathBuf::from("mockforge-analytics.db"),
            aggregation_interval_seconds: default_aggregation_interval(),
            rollup_interval_hours: default_rollup_interval(),
            retention: RetentionConfig::default(),
            batch_size: default_batch_size(),
            max_query_results: default_max_query_results(),
        }
    }
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            minute_aggregates_days: default_minute_retention(),
            hour_aggregates_days: default_hour_retention(),
            day_aggregates_days: default_day_retention(),
            error_events_days: default_error_retention(),
            client_analytics_days: default_client_retention(),
            traffic_patterns_days: default_traffic_retention(),
            snapshots_days: default_snapshot_retention(),
            cleanup_interval_hours: default_cleanup_interval(),
        }
    }
}

// Default value functions

fn default_aggregation_interval() -> u64 {
    60 // 1 minute
}

fn default_rollup_interval() -> u64 {
    1 // 1 hour
}

fn default_batch_size() -> usize {
    1000
}

fn default_max_query_results() -> usize {
    10000
}

fn default_minute_retention() -> u32 {
    7 // 7 days
}

fn default_hour_retention() -> u32 {
    30 // 30 days
}

fn default_day_retention() -> u32 {
    365 // 1 year
}

fn default_error_retention() -> u32 {
    7 // 7 days
}

fn default_client_retention() -> u32 {
    30 // 30 days
}

fn default_traffic_retention() -> u32 {
    90 // 90 days
}

fn default_snapshot_retention() -> u32 {
    90 // 90 days
}

fn default_cleanup_interval() -> u32 {
    24 // Daily
}
