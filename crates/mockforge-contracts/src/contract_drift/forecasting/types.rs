//! Types for API change forecasting
//!
//! This module defines data structures for predicting future contract changes
//! based on historical drift patterns.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Forecast for API contract changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeForecast {
    /// Service identifier (optional)
    pub service_id: Option<String>,
    /// Service name (optional)
    pub service_name: Option<String>,
    /// Endpoint path
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Forecast window in days (30, 90, or 180)
    pub forecast_window_days: u32,
    /// Probability of any change occurring (0.0-1.0)
    pub predicted_change_probability: f64,
    /// Probability of a breaking change occurring (0.0-1.0)
    pub predicted_break_probability: f64,
    /// Expected date of next change (if predictable)
    pub next_expected_change_date: Option<DateTime<Utc>>,
    /// Expected date of next breaking change (if predictable)
    pub next_expected_break_date: Option<DateTime<Utc>>,
    /// Volatility score (0.0-1.0) - how frequently changes occur
    pub volatility_score: f64,
    /// Confidence in this forecast (0.0-1.0)
    pub confidence: f64,
    /// Detected seasonal patterns
    pub seasonal_patterns: Vec<SeasonalPattern>,
    /// When this forecast was generated
    pub predicted_at: DateTime<Utc>,
    /// When this forecast expires (should be refreshed)
    pub expires_at: DateTime<Utc>,
}

/// Seasonal pattern detected in historical data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonalPattern {
    /// Type of pattern
    pub pattern_type: PatternType,
    /// Frequency in days
    pub frequency_days: f64,
    /// Last occurrence
    pub last_occurrence: DateTime<Utc>,
    /// Confidence in this pattern (0.0-1.0)
    pub confidence: f64,
    /// Pattern description
    pub description: String,
}

/// Type of change pattern detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PatternType {
    /// Regular field additions
    FieldAddition,
    /// Field renames during refactors
    FieldRename,
    /// Breaking changes
    BreakingChange,
    /// Non-breaking changes
    NonBreakingChange,
    /// Quarterly refactors
    QuarterlyRefactor,
    /// Monthly maintenance cycles
    MonthlyMaintenance,
    /// Weekly updates
    WeeklyUpdate,
    /// Custom pattern
    Custom(String),
}

/// Forecast pattern extracted from historical data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastPattern {
    /// Type of pattern
    pub pattern_type: PatternType,
    /// Average frequency in days
    pub frequency_days: f64,
    /// Last occurrence
    pub last_occurrence: DateTime<Utc>,
    /// Confidence in this pattern (0.0-1.0)
    pub confidence: f64,
    /// Number of occurrences observed
    pub occurrence_count: usize,
    /// Standard deviation of frequency
    pub frequency_stddev: f64,
}

/// Historical pattern analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternAnalysis {
    /// Detected patterns
    pub patterns: Vec<ForecastPattern>,
    /// Overall volatility score (0.0-1.0)
    pub volatility_score: f64,
    /// Average time between changes (days)
    pub avg_change_interval_days: f64,
    /// Average time between breaking changes (days)
    pub avg_breaking_change_interval_days: Option<f64>,
    /// Total changes observed
    pub total_changes: usize,
    /// Total breaking changes observed
    pub total_breaking_changes: usize,
    /// Analysis window start
    pub window_start: DateTime<Utc>,
    /// Analysis window end
    pub window_end: DateTime<Utc>,
}

/// Forecast statistics for a service or endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastStatistics {
    /// Workspace ID
    pub workspace_id: Option<String>,
    /// Service ID
    pub service_id: Option<String>,
    /// Service name
    pub service_name: Option<String>,
    /// Endpoint (optional, None for service-level)
    pub endpoint: Option<String>,
    /// Method (optional, None for service-level)
    pub method: Option<String>,
    /// Aggregation level
    pub aggregation_level: ForecastAggregationLevel,
    /// Time window in days
    pub time_window_days: u32,
    /// Change frequency (changes per day)
    pub change_frequency: f64,
    /// Breaking change frequency (breaking changes per day)
    pub breaking_change_frequency: f64,
    /// Volatility score
    pub volatility_score: f64,
    /// Pattern signatures detected
    pub pattern_signatures: Vec<PatternSignature>,
    /// Detected pattern types
    pub detected_pattern_types: Vec<PatternType>,
    /// Last change date
    pub last_change_date: Option<DateTime<Utc>>,
    /// Last breaking change date
    pub last_breaking_change_date: Option<DateTime<Utc>>,
    /// Total changes in window
    pub total_changes: usize,
    /// Total breaking changes in window
    pub total_breaking_changes: usize,
    /// Window start
    pub window_start: DateTime<Utc>,
    /// Window end
    pub window_end: DateTime<Utc>,
    /// Calculated at
    pub calculated_at: DateTime<Utc>,
}

/// Aggregation level for forecast statistics
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ForecastAggregationLevel {
    /// Workspace-level statistics
    Workspace,
    /// Service-level statistics
    Service,
    /// Endpoint-level statistics
    Endpoint,
}

/// Pattern signature with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternSignature {
    /// Pattern type
    pub pattern_type: PatternType,
    /// Frequency in days
    pub frequency_days: f64,
    /// First occurrence
    pub first_occurrence: DateTime<Utc>,
    /// Last occurrence
    pub last_occurrence: DateTime<Utc>,
    /// Occurrence count
    pub occurrence_count: usize,
    /// Confidence
    pub confidence: f64,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Configuration for forecasting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastingConfig {
    /// Whether forecasting is enabled
    pub enabled: bool,
    /// Default forecast expiration hours (default: 12)
    pub default_expiration_hours: u32,
    /// Minimum incidents required for forecasting
    pub min_incidents_for_forecast: usize,
    /// Confidence threshold for including patterns
    pub pattern_confidence_threshold: f64,
    /// Time windows to analyze (in days)
    pub analysis_windows: Vec<u32>,
}

impl Default for ForecastingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_expiration_hours: 12,
            min_incidents_for_forecast: 3,
            pattern_confidence_threshold: 0.5,
            analysis_windows: vec![30, 90, 180],
        }
    }
}
