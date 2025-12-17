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

const fn default_aggregation_interval() -> u64 {
    60 // 1 minute
}

const fn default_rollup_interval() -> u64 {
    1 // 1 hour
}

const fn default_batch_size() -> usize {
    1000
}

const fn default_max_query_results() -> usize {
    10000
}

const fn default_minute_retention() -> u32 {
    7 // 7 days
}

const fn default_hour_retention() -> u32 {
    30 // 30 days
}

const fn default_day_retention() -> u32 {
    365 // 1 year
}

const fn default_error_retention() -> u32 {
    7 // 7 days
}

const fn default_client_retention() -> u32 {
    30 // 30 days
}

const fn default_traffic_retention() -> u32 {
    90 // 90 days
}

const fn default_snapshot_retention() -> u32 {
    90 // 90 days
}

const fn default_cleanup_interval() -> u32 {
    24 // Daily
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analytics_config_default() {
        let config = AnalyticsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.database_path, PathBuf::from("mockforge-analytics.db"));
        assert_eq!(config.aggregation_interval_seconds, 60);
        assert_eq!(config.rollup_interval_hours, 1);
        assert_eq!(config.batch_size, 1000);
        assert_eq!(config.max_query_results, 10000);
    }

    #[test]
    fn test_retention_config_default() {
        let config = RetentionConfig::default();
        assert_eq!(config.minute_aggregates_days, 7);
        assert_eq!(config.hour_aggregates_days, 30);
        assert_eq!(config.day_aggregates_days, 365);
        assert_eq!(config.error_events_days, 7);
        assert_eq!(config.client_analytics_days, 30);
        assert_eq!(config.traffic_patterns_days, 90);
        assert_eq!(config.snapshots_days, 90);
        assert_eq!(config.cleanup_interval_hours, 24);
    }

    #[test]
    fn test_analytics_config_serialize() {
        let config = AnalyticsConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"enabled\":true"));
        assert!(json.contains("\"aggregation_interval_seconds\":60"));
    }

    #[test]
    fn test_analytics_config_deserialize() {
        let json = r#"{
            "enabled": false,
            "database_path": "/tmp/test.db",
            "aggregation_interval_seconds": 120,
            "rollup_interval_hours": 2,
            "batch_size": 500,
            "max_query_results": 5000
        }"#;
        let config: AnalyticsConfig = serde_json::from_str(json).unwrap();
        assert!(!config.enabled);
        assert_eq!(config.database_path, PathBuf::from("/tmp/test.db"));
        assert_eq!(config.aggregation_interval_seconds, 120);
        assert_eq!(config.rollup_interval_hours, 2);
        assert_eq!(config.batch_size, 500);
        assert_eq!(config.max_query_results, 5000);
    }

    #[test]
    fn test_retention_config_serialize() {
        let config = RetentionConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("\"minute_aggregates_days\":7"));
        assert!(json.contains("\"hour_aggregates_days\":30"));
    }

    #[test]
    fn test_retention_config_deserialize() {
        let json = r#"{
            "minute_aggregates_days": 14,
            "hour_aggregates_days": 60,
            "day_aggregates_days": 180,
            "error_events_days": 30,
            "client_analytics_days": 60,
            "traffic_patterns_days": 45,
            "snapshots_days": 120,
            "cleanup_interval_hours": 12
        }"#;
        let config: RetentionConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.minute_aggregates_days, 14);
        assert_eq!(config.hour_aggregates_days, 60);
        assert_eq!(config.day_aggregates_days, 180);
        assert_eq!(config.error_events_days, 30);
        assert_eq!(config.client_analytics_days, 60);
        assert_eq!(config.traffic_patterns_days, 45);
        assert_eq!(config.snapshots_days, 120);
        assert_eq!(config.cleanup_interval_hours, 12);
    }

    #[test]
    fn test_analytics_config_clone() {
        let config = AnalyticsConfig::default();
        let cloned = config.clone();
        assert_eq!(config.enabled, cloned.enabled);
        assert_eq!(config.database_path, cloned.database_path);
    }

    #[test]
    fn test_retention_config_clone() {
        let config = RetentionConfig::default();
        let cloned = config.clone();
        assert_eq!(config.minute_aggregates_days, cloned.minute_aggregates_days);
        assert_eq!(config.hour_aggregates_days, cloned.hour_aggregates_days);
    }

    #[test]
    fn test_analytics_config_with_defaults_in_partial_json() {
        let json = r#"{
            "enabled": true,
            "database_path": "/data/analytics.db"
        }"#;
        let config: AnalyticsConfig = serde_json::from_str(json).unwrap();
        // Defaults should be applied for missing fields
        assert_eq!(config.aggregation_interval_seconds, 60);
        assert_eq!(config.rollup_interval_hours, 1);
        assert_eq!(config.batch_size, 1000);
    }

    #[test]
    fn test_retention_config_debug() {
        let config = RetentionConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("RetentionConfig"));
        assert!(debug.contains("minute_aggregates_days"));
    }

    #[test]
    fn test_analytics_config_debug() {
        let config = AnalyticsConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("AnalyticsConfig"));
        assert!(debug.contains("enabled"));
        assert!(debug.contains("database_path"));
    }
}
