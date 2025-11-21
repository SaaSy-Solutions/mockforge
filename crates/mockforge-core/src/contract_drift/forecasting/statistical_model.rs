//! Statistical model for forecasting
//!
//! This module provides time series analysis and statistical methods
//! for predicting future contract changes.

use super::types::{ForecastStatistics, PatternAnalysis};
use chrono::{DateTime, Duration, Utc};

/// Statistical model for forecasting
pub struct StatisticalModel;

impl StatisticalModel {
    /// Create a new statistical model
    pub fn new() -> Self {
        Self
    }

    /// Calculate forecast statistics from pattern analysis
    pub fn calculate_statistics(
        &self,
        analysis: &PatternAnalysis,
        workspace_id: Option<String>,
        service_id: Option<String>,
        service_name: Option<String>,
        endpoint: Option<String>,
        method: Option<String>,
    ) -> ForecastStatistics {
        let aggregation_level = if endpoint.is_some() && method.is_some() {
            super::types::ForecastAggregationLevel::Endpoint
        } else if service_id.is_some() || service_name.is_some() {
            super::types::ForecastAggregationLevel::Service
        } else {
            super::types::ForecastAggregationLevel::Workspace
        };

        let window_days = (analysis.window_end - analysis.window_start)
            .num_seconds() as f64
            / 86400.0;

        let change_frequency = if window_days > 0.0 {
            analysis.total_changes as f64 / window_days
        } else {
            0.0
        };

        let breaking_change_frequency = if window_days > 0.0 {
            analysis.total_breaking_changes as f64 / window_days
        } else {
            0.0
        };

        let pattern_signatures: Vec<_> = analysis
            .patterns
            .iter()
            .map(|p| super::types::PatternSignature {
                pattern_type: p.pattern_type.clone(),
                frequency_days: p.frequency_days,
                first_occurrence: analysis.window_start,
                last_occurrence: p.last_occurrence,
                occurrence_count: p.occurrence_count,
                confidence: p.confidence,
                metadata: std::collections::HashMap::new(),
            })
            .collect();

        let detected_pattern_types: Vec<_> = analysis
            .patterns
            .iter()
            .map(|p| p.pattern_type.clone())
            .collect();

        ForecastStatistics {
            workspace_id,
            service_id,
            service_name,
            endpoint,
            method,
            aggregation_level,
            time_window_days: window_days as u32,
            change_frequency,
            breaking_change_frequency,
            volatility_score: analysis.volatility_score,
            pattern_signatures,
            detected_pattern_types,
            last_change_date: if analysis.total_changes > 0 {
                Some(analysis.window_end)
            } else {
                None
            },
            last_breaking_change_date: if analysis.total_breaking_changes > 0 {
                Some(analysis.window_end)
            } else {
                None
            },
            total_changes: analysis.total_changes,
            total_breaking_changes: analysis.total_breaking_changes,
            window_start: analysis.window_start,
            window_end: analysis.window_end,
            calculated_at: Utc::now(),
        }
    }

    /// Predict probability of change in next N days
    pub fn predict_change_probability(
        &self,
        analysis: &PatternAnalysis,
        forecast_window_days: u32,
    ) -> f64 {
        if analysis.patterns.is_empty() {
            // No patterns detected - use historical frequency
            return self.predict_from_frequency(analysis, forecast_window_days);
        }

        // Use pattern-based prediction
        let mut probabilities = Vec::new();

        for pattern in &analysis.patterns {
            let prob = self.predict_from_pattern(pattern, forecast_window_days);
            probabilities.push(prob * pattern.confidence);
        }

        // Take weighted average
        if probabilities.is_empty() {
            self.predict_from_frequency(analysis, forecast_window_days)
        } else {
            probabilities.iter().sum::<f64>() / probabilities.len() as f64
        }
    }

    /// Predict probability from historical frequency
    fn predict_from_frequency(
        &self,
        analysis: &PatternAnalysis,
        forecast_window_days: u32,
    ) -> f64 {
        if analysis.avg_change_interval_days == 0.0 {
            return 0.0;
        }

        // Poisson-like model: probability = 1 - e^(-lambda * t)
        // where lambda = 1 / avg_interval, t = forecast_window
        let lambda = 1.0 / analysis.avg_change_interval_days;
        let t = forecast_window_days as f64;
        let prob = 1.0 - (-lambda * t).exp();

        prob.min(1.0).max(0.0)
    }

    /// Predict probability from pattern
    fn predict_from_pattern(
        &self,
        pattern: &super::types::ForecastPattern,
        forecast_window_days: u32,
    ) -> f64 {
        let days_since_last = (Utc::now() - pattern.last_occurrence)
            .num_seconds() as f64
            / 86400.0;

        let forecast_days = forecast_window_days as f64;

        // If we're past the expected next occurrence, probability is high
        if days_since_last >= pattern.frequency_days {
            return 0.8; // High probability if pattern suggests it's overdue
        }

        // Calculate probability based on how close we are to expected date
        let days_until_expected = pattern.frequency_days - days_since_last;

        if forecast_days >= days_until_expected {
            // Forecast window includes expected date
            let overlap = forecast_days - days_until_expected.max(0.0);
            let overlap_ratio = overlap / forecast_days;
            0.5 + (overlap_ratio * 0.4) // 0.5-0.9 range
        } else {
            // Forecast window is before expected date
            let ratio = forecast_days / days_until_expected;
            ratio * 0.3 // Lower probability if before expected
        }
    }

    /// Predict probability of breaking change
    pub fn predict_break_probability(
        &self,
        analysis: &PatternAnalysis,
        forecast_window_days: u32,
    ) -> f64 {
        if let Some(avg_breaking_interval) = analysis.avg_breaking_change_interval_days {
            if avg_breaking_interval > 0.0 {
                let lambda = 1.0 / avg_breaking_interval;
                let t = forecast_window_days as f64;
                let prob = 1.0 - (-lambda * t).exp();
                return prob.min(1.0).max(0.0);
            }
        }

        // Fallback: use ratio of breaking to total changes
        if analysis.total_changes > 0 {
            let breaking_ratio = analysis.total_breaking_changes as f64
                / analysis.total_changes as f64;
            let change_prob = self.predict_change_probability(analysis, forecast_window_days);
            change_prob * breaking_ratio
        } else {
            0.0
        }
    }

    /// Predict next expected change date
    pub fn predict_next_change_date(
        &self,
        analysis: &PatternAnalysis,
    ) -> Option<DateTime<Utc>> {
        if analysis.patterns.is_empty() {
            // Use average interval
            if analysis.avg_change_interval_days > 0.0 {
                let last_change = analysis.window_end;
                return Some(last_change + Duration::days(analysis.avg_change_interval_days as i64));
            }
            return None;
        }

        // Use most confident pattern
        let best_pattern = analysis
            .patterns
            .iter()
            .max_by(|a, b| a.confidence.partial_cmp(&b.confidence).unwrap_or(std::cmp::Ordering::Equal))?;

        let days_since_last = (Utc::now() - best_pattern.last_occurrence)
            .num_seconds() as f64
            / 86400.0;

        if days_since_last >= best_pattern.frequency_days {
            // Overdue - predict soon
            Some(Utc::now() + Duration::days(7))
        } else {
            // Predict based on pattern
            let days_until_next = best_pattern.frequency_days - days_since_last;
            Some(best_pattern.last_occurrence + Duration::days(days_until_next as i64))
        }
    }

    /// Predict next expected breaking change date
    pub fn predict_next_break_date(
        &self,
        analysis: &PatternAnalysis,
    ) -> Option<DateTime<Utc>> {
        if let Some(avg_breaking_interval) = analysis.avg_breaking_change_interval_days {
            if avg_breaking_interval > 0.0 {
                let last_breaking = analysis.window_end;
                return Some(
                    last_breaking + Duration::days(avg_breaking_interval as i64),
                );
            }
        }

        // Check for breaking change patterns
        let breaking_pattern = analysis
            .patterns
            .iter()
            .find(|p| matches!(p.pattern_type, super::types::PatternType::BreakingChange))?;

        let days_since_last = (Utc::now() - breaking_pattern.last_occurrence)
            .num_seconds() as f64
            / 86400.0;

        if days_since_last >= breaking_pattern.frequency_days {
            Some(Utc::now() + Duration::days(7))
        } else {
            let days_until_next = breaking_pattern.frequency_days - days_since_last;
            Some(
                breaking_pattern.last_occurrence + Duration::days(days_until_next as i64),
            )
        }
    }

    /// Calculate forecast confidence
    pub fn calculate_confidence(
        &self,
        analysis: &PatternAnalysis,
        min_incidents: usize,
    ) -> f64 {
        if analysis.total_changes < min_incidents {
            return 0.3; // Low confidence with insufficient data
        }

        // Confidence factors:
        // 1. Number of incidents (more = better, up to a point)
        let data_factor = (analysis.total_changes.min(20) as f64 / 20.0).min(1.0);

        // 2. Pattern confidence (if patterns detected)
        let pattern_factor = if !analysis.patterns.is_empty() {
            analysis
                .patterns
                .iter()
                .map(|p| p.confidence)
                .sum::<f64>()
                / analysis.patterns.len() as f64
        } else {
            0.5 // Medium if no clear patterns
        };

        // 3. Volatility consistency (lower variance = higher confidence)
        let consistency_factor = 1.0 - (analysis.volatility_score * 0.3).min(0.5);

        (data_factor * 0.4 + pattern_factor * 0.4 + consistency_factor * 0.2).min(1.0)
    }
}

impl Default for StatisticalModel {
    fn default() -> Self {
        Self::new()
    }
}

