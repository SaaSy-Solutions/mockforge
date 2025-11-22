//! Pattern analysis for forecasting
//!
//! This module analyzes historical drift incidents to detect patterns
//! that can be used for predicting future changes.

use super::types::{ForecastPattern, PatternAnalysis, PatternType};
use crate::incidents::types::{DriftIncident, IncidentType};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

/// Pattern analyzer for detecting change patterns
pub struct PatternAnalyzer {
    /// Minimum occurrences to consider a pattern valid
    min_occurrences: usize,
    /// Confidence threshold for patterns
    confidence_threshold: f64,
}

impl PatternAnalyzer {
    /// Create a new pattern analyzer
    pub fn new(min_occurrences: usize, confidence_threshold: f64) -> Self {
        Self {
            min_occurrences,
            confidence_threshold,
        }
    }

    /// Analyze historical incidents to detect patterns
    pub fn analyze_patterns(
        &self,
        incidents: &[DriftIncident],
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
    ) -> PatternAnalysis {
        if incidents.is_empty() {
            return PatternAnalysis {
                patterns: Vec::new(),
                volatility_score: 0.0,
                avg_change_interval_days: 0.0,
                avg_breaking_change_interval_days: None,
                total_changes: 0,
                total_breaking_changes: 0,
                window_start,
                window_end,
            };
        }

        // Sort incidents by detection time
        let mut sorted_incidents: Vec<_> = incidents
            .iter()
            .filter(|inc| {
                let detected = DateTime::<Utc>::from_timestamp(inc.detected_at, 0)
                    .unwrap_or_else(|| Utc::now());
                detected >= window_start && detected <= window_end
            })
            .collect();
        sorted_incidents.sort_by_key(|inc| inc.detected_at);

        // Calculate intervals between changes
        let intervals = self.calculate_intervals(&sorted_incidents);
        let breaking_intervals = self.calculate_breaking_intervals(&sorted_incidents);

        // Detect patterns
        let patterns = self.detect_patterns(&sorted_incidents, &intervals);

        // Calculate volatility (based on frequency and variance)
        let volatility_score = self.calculate_volatility(&intervals, window_start, window_end);

        // Calculate averages
        let avg_change_interval_days = if !intervals.is_empty() {
            intervals.iter().sum::<f64>() / intervals.len() as f64
        } else {
            0.0
        };

        let avg_breaking_change_interval_days = if !breaking_intervals.is_empty() {
            Some(breaking_intervals.iter().sum::<f64>() / breaking_intervals.len() as f64)
        } else {
            None
        };

        let total_breaking_changes = sorted_incidents
            .iter()
            .filter(|inc| inc.incident_type == IncidentType::BreakingChange)
            .count();

        PatternAnalysis {
            patterns,
            volatility_score,
            avg_change_interval_days,
            avg_breaking_change_interval_days,
            total_changes: sorted_incidents.len(),
            total_breaking_changes,
            window_start,
            window_end,
        }
    }

    /// Calculate time intervals between incidents
    fn calculate_intervals(&self, incidents: &[&DriftIncident]) -> Vec<f64> {
        if incidents.len() < 2 {
            return Vec::new();
        }

        let mut intervals = Vec::new();
        for i in 1..incidents.len() {
            let prev_time = DateTime::<Utc>::from_timestamp(incidents[i - 1].detected_at, 0)
                .unwrap_or_else(|| Utc::now());
            let curr_time = DateTime::<Utc>::from_timestamp(incidents[i].detected_at, 0)
                .unwrap_or_else(|| Utc::now());

            let duration = curr_time.signed_duration_since(prev_time);
            let days = duration.num_seconds() as f64 / 86400.0;
            intervals.push(days);
        }

        intervals
    }

    /// Calculate time intervals between breaking changes
    fn calculate_breaking_intervals(&self, incidents: &[&DriftIncident]) -> Vec<f64> {
        let breaking: Vec<_> = incidents
            .iter()
            .filter(|inc| inc.incident_type == IncidentType::BreakingChange)
            .collect();

        if breaking.len() < 2 {
            return Vec::new();
        }

        let mut intervals = Vec::new();
        for i in 1..breaking.len() {
            let prev_time = DateTime::<Utc>::from_timestamp(breaking[i - 1].detected_at, 0)
                .unwrap_or_else(|| Utc::now());
            let curr_time = DateTime::<Utc>::from_timestamp(breaking[i].detected_at, 0)
                .unwrap_or_else(|| Utc::now());

            let duration = curr_time.signed_duration_since(prev_time);
            let days = duration.num_seconds() as f64 / 86400.0;
            intervals.push(days);
        }

        intervals
    }

    /// Detect patterns in incidents
    fn detect_patterns(
        &self,
        incidents: &[&DriftIncident],
        intervals: &[f64],
    ) -> Vec<ForecastPattern> {
        let mut patterns = Vec::new();

        if intervals.is_empty() {
            return patterns;
        }

        // Detect regular patterns (weekly, monthly, quarterly)
        patterns.extend(self.detect_regular_patterns(intervals, incidents));

        // Detect breaking change patterns
        patterns.extend(self.detect_breaking_patterns(incidents));

        // Detect field addition patterns (from incident details)
        patterns.extend(self.detect_field_patterns(incidents));

        // Filter by confidence threshold
        patterns.retain(|p| p.confidence >= self.confidence_threshold);

        patterns
    }

    /// Detect regular patterns (weekly, monthly, quarterly)
    fn detect_regular_patterns(
        &self,
        intervals: &[f64],
        incidents: &[&DriftIncident],
    ) -> Vec<ForecastPattern> {
        let mut patterns = Vec::new();

        if intervals.len() < self.min_occurrences {
            return patterns;
        }

        // Calculate average interval
        let avg_interval = intervals.iter().sum::<f64>() / intervals.len() as f64;

        // Calculate standard deviation
        let variance = intervals
            .iter()
            .map(|x| (x - avg_interval).powi(2))
            .sum::<f64>()
            / intervals.len() as f64;
        let stddev = variance.sqrt();

        // Check for weekly pattern (6-8 days)
        if avg_interval >= 6.0 && avg_interval <= 8.0 && stddev < 2.0 {
            if let Some(last) = incidents.last() {
                let last_time = DateTime::<Utc>::from_timestamp(last.detected_at, 0)
                    .unwrap_or_else(|| Utc::now());
                let confidence = self.calculate_pattern_confidence(intervals, avg_interval, stddev);
                patterns.push(ForecastPattern {
                    pattern_type: PatternType::WeeklyUpdate,
                    frequency_days: avg_interval,
                    last_occurrence: last_time,
                    confidence,
                    occurrence_count: intervals.len() + 1,
                    frequency_stddev: stddev,
                });
            }
        }

        // Check for monthly pattern (28-32 days)
        if avg_interval >= 28.0 && avg_interval <= 32.0 && stddev < 5.0 {
            if let Some(last) = incidents.last() {
                let last_time = DateTime::<Utc>::from_timestamp(last.detected_at, 0)
                    .unwrap_or_else(|| Utc::now());
                let confidence = self.calculate_pattern_confidence(intervals, avg_interval, stddev);
                patterns.push(ForecastPattern {
                    pattern_type: PatternType::MonthlyMaintenance,
                    frequency_days: avg_interval,
                    last_occurrence: last_time,
                    confidence,
                    occurrence_count: intervals.len() + 1,
                    frequency_stddev: stddev,
                });
            }
        }

        // Check for quarterly pattern (88-92 days)
        if avg_interval >= 88.0 && avg_interval <= 92.0 && stddev < 10.0 {
            if let Some(last) = incidents.last() {
                let last_time = DateTime::<Utc>::from_timestamp(last.detected_at, 0)
                    .unwrap_or_else(|| Utc::now());
                let confidence = self.calculate_pattern_confidence(intervals, avg_interval, stddev);
                patterns.push(ForecastPattern {
                    pattern_type: PatternType::QuarterlyRefactor,
                    frequency_days: avg_interval,
                    last_occurrence: last_time,
                    confidence,
                    occurrence_count: intervals.len() + 1,
                    frequency_stddev: stddev,
                });
            }
        }

        patterns
    }

    /// Detect breaking change patterns
    fn detect_breaking_patterns(&self, incidents: &[&DriftIncident]) -> Vec<ForecastPattern> {
        let breaking: Vec<&DriftIncident> = incidents
            .iter()
            .filter(|inc| inc.incident_type == IncidentType::BreakingChange)
            .copied()
            .collect();

        if breaking.len() < self.min_occurrences {
            return Vec::new();
        }

        let intervals = self.calculate_breaking_intervals(&breaking);
        if intervals.is_empty() {
            return Vec::new();
        }

        let avg_interval = intervals.iter().sum::<f64>() / intervals.len() as f64;
        let variance = intervals
            .iter()
            .map(|x| (x - avg_interval).powi(2))
            .sum::<f64>()
            / intervals.len() as f64;
        let stddev = variance.sqrt();

        if let Some(last) = breaking.last() {
            let last_time = DateTime::<Utc>::from_timestamp(last.detected_at, 0)
                .unwrap_or_else(|| Utc::now());
            let confidence = self.calculate_pattern_confidence(&intervals, avg_interval, stddev);
            vec![ForecastPattern {
                pattern_type: PatternType::BreakingChange,
                frequency_days: avg_interval,
                last_occurrence: last_time,
                confidence,
                occurrence_count: breaking.len(),
                frequency_stddev: stddev,
            }]
        } else {
            Vec::new()
        }
    }

    /// Detect field-related patterns from incident details
    fn detect_field_patterns(&self, incidents: &[&DriftIncident]) -> Vec<ForecastPattern> {
        // Analyze incident details to detect field addition/rename patterns
        // This is a simplified version - in practice, you'd parse the details JSON
        // to extract specific change types

        let field_additions: Vec<&DriftIncident> = incidents
            .iter()
            .filter(|inc| {
                // Check if details indicate field addition
                inc.details
                    .as_object()
                    .and_then(|obj| obj.get("change_type"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.contains("field_added") || s.contains("field_addition"))
                    .unwrap_or(false)
            })
            .copied()
            .collect();

        if field_additions.len() >= self.min_occurrences {
            let intervals = self.calculate_intervals(&field_additions);
            if !intervals.is_empty() {
                let avg_interval = intervals.iter().sum::<f64>() / intervals.len() as f64;
                let variance = intervals
                    .iter()
                    .map(|x| (x - avg_interval).powi(2))
                    .sum::<f64>()
                    / intervals.len() as f64;
                let stddev = variance.sqrt();

                if let Some(last) = field_additions.last() {
                    let last_time = DateTime::<Utc>::from_timestamp(last.detected_at, 0)
                        .unwrap_or_else(|| Utc::now());
                    let confidence =
                        self.calculate_pattern_confidence(&intervals, avg_interval, stddev);
                    return vec![ForecastPattern {
                        pattern_type: PatternType::FieldAddition,
                        frequency_days: avg_interval,
                        last_occurrence: last_time,
                        confidence,
                        occurrence_count: field_additions.len(),
                        frequency_stddev: stddev,
                    }];
                }
            }
        }

        Vec::new()
    }

    /// Calculate pattern confidence based on consistency
    fn calculate_pattern_confidence(
        &self,
        intervals: &[f64],
        avg_interval: f64,
        stddev: f64,
    ) -> f64 {
        if intervals.is_empty() || avg_interval == 0.0 {
            return 0.0;
        }

        // Confidence is higher when:
        // 1. More occurrences (up to a point)
        // 2. Lower standard deviation (more consistent)
        let occurrence_factor = (intervals.len().min(10) as f64 / 10.0).min(1.0);
        let consistency_factor = (1.0 - (stddev / avg_interval).min(1.0)).max(0.0);

        (occurrence_factor * 0.4 + consistency_factor * 0.6).min(1.0)
    }

    /// Calculate volatility score
    fn calculate_volatility(
        &self,
        intervals: &[f64],
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
    ) -> f64 {
        if intervals.is_empty() {
            return 0.0;
        }

        let window_days = (window_end - window_start).num_seconds() as f64 / 86400.0;
        if window_days == 0.0 {
            return 0.0;
        }

        // Volatility is based on:
        // 1. Frequency of changes (more changes = higher volatility)
        // 2. Variance in intervals (more variance = higher volatility)
        let change_count = intervals.len() + 1;
        let frequency = change_count as f64 / window_days;

        let avg_interval = intervals.iter().sum::<f64>() / intervals.len() as f64;
        let variance = intervals
            .iter()
            .map(|x| (x - avg_interval).powi(2))
            .sum::<f64>()
            / intervals.len() as f64;
        let coefficient_of_variation = if avg_interval > 0.0 {
            variance.sqrt() / avg_interval
        } else {
            0.0
        };

        // Normalize to 0.0-1.0
        // High frequency (daily changes) = 1.0, low frequency (yearly) = 0.0
        let frequency_score = (frequency * 30.0).min(1.0); // Daily = 1.0
        let variance_score = coefficient_of_variation.min(1.0);

        (frequency_score * 0.6 + variance_score * 0.4).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::incidents::types::{IncidentSeverity, IncidentStatus};

    fn create_test_incident(
        id: &str,
        detected_at: i64,
        incident_type: IncidentType,
    ) -> DriftIncident {
        DriftIncident {
            id: id.to_string(),
            budget_id: None,
            workspace_id: None,
            endpoint: "/api/test".to_string(),
            method: "GET".to_string(),
            incident_type,
            severity: IncidentSeverity::Medium,
            status: IncidentStatus::Open,
            detected_at,
            resolved_at: None,
            details: serde_json::json!({}),
            external_ticket_id: None,
            external_ticket_url: None,
            created_at: detected_at,
            updated_at: detected_at,
            sync_cycle_id: None,
            contract_diff_id: None,
            before_sample: None,
            after_sample: None,
            fitness_test_results: Vec::new(),
            affected_consumers: None,
            protocol: None,
        }
    }

    #[test]
    fn test_analyze_empty_incidents() {
        let analyzer = PatternAnalyzer::new(3, 0.5);
        let window_start = Utc::now() - Duration::days(90);
        let window_end = Utc::now();
        let analysis = analyzer.analyze_patterns(&[], window_start, window_end);

        assert_eq!(analysis.total_changes, 0);
        assert_eq!(analysis.volatility_score, 0.0);
    }

    #[test]
    fn test_detect_weekly_pattern() {
        let analyzer = PatternAnalyzer::new(3, 0.5);
        let now = Utc::now();
        let mut incidents = Vec::new();

        // Create weekly incidents
        for i in 0..5 {
            let timestamp = (now - Duration::days(i * 7)).timestamp();
            incidents.push(create_test_incident(
                &format!("inc_{}", i),
                timestamp,
                IncidentType::ThresholdExceeded,
            ));
        }

        let window_start = now - Duration::days(35);
        let window_end = now;
        let analysis = analyzer.analyze_patterns(&incidents, window_start, window_end);

        assert!(analysis.volatility_score > 0.0);
        assert!(!analysis.patterns.is_empty());
    }
}
