//! Main forecasting engine
//!
//! This module orchestrates pattern analysis and statistical modeling
//! to generate change forecasts.

use super::pattern_analyzer::PatternAnalyzer;
use super::statistical_model::StatisticalModel;
use super::types::{ChangeForecast, ForecastingConfig, PatternAnalysis};
use crate::incidents::types::DriftIncident;
use chrono::{DateTime, Duration, Utc};

/// Main forecaster for API change predictions
pub struct Forecaster {
    /// Pattern analyzer
    pattern_analyzer: PatternAnalyzer,
    /// Statistical model
    statistical_model: StatisticalModel,
    /// Configuration
    config: ForecastingConfig,
}

impl Forecaster {
    /// Create a new forecaster
    pub fn new(config: ForecastingConfig) -> Self {
        let pattern_analyzer = PatternAnalyzer::new(
            config.min_incidents_for_forecast,
            config.pattern_confidence_threshold,
        );
        let statistical_model = StatisticalModel::new();

        Self {
            pattern_analyzer,
            statistical_model,
            config,
        }
    }

    /// Generate forecast for a service or endpoint
    pub fn generate_forecast(
        &self,
        incidents: &[DriftIncident],
        workspace_id: Option<String>,
        service_id: Option<String>,
        service_name: Option<String>,
        endpoint: String,
        method: String,
        forecast_window_days: u32,
    ) -> Option<ChangeForecast> {
        if !self.config.enabled {
            return None;
        }

        if incidents.len() < self.config.min_incidents_for_forecast {
            return None;
        }

        // Analyze patterns for each time window
        let mut analyses = Vec::new();
        let now = Utc::now();

        for &window_days in &self.config.analysis_windows {
            let window_start = now - Duration::days(window_days as i64);
            let window_end = now;

            let analysis = self
                .pattern_analyzer
                .analyze_patterns(incidents, window_start, window_end);
            analyses.push((window_days, analysis));
        }

        // Use the longest window analysis for forecasting
        let (_, analysis) = analyses
            .iter()
            .max_by_key(|(days, _)| *days)
            .or_else(|| analyses.first())?;

        // Generate predictions
        let change_probability = self
            .statistical_model
            .predict_change_probability(analysis, forecast_window_days);
        let break_probability = self
            .statistical_model
            .predict_break_probability(analysis, forecast_window_days);
        let next_change_date = self.statistical_model.predict_next_change_date(analysis);
        let next_break_date = self.statistical_model.predict_next_break_date(analysis);
        let confidence = self
            .statistical_model
            .calculate_confidence(analysis, self.config.min_incidents_for_forecast);

        // Extract seasonal patterns
        let seasonal_patterns: Vec<_> = analysis
            .patterns
            .iter()
            .filter(|p| {
                matches!(
                    p.pattern_type,
                    super::types::PatternType::MonthlyMaintenance
                        | super::types::PatternType::QuarterlyRefactor
                        | super::types::PatternType::WeeklyUpdate
                )
            })
            .map(|p| super::types::SeasonalPattern {
                pattern_type: p.pattern_type.clone(),
                frequency_days: p.frequency_days,
                last_occurrence: p.last_occurrence,
                confidence: p.confidence,
                description: format!("{:?} pattern", p.pattern_type),
            })
            .collect();

        // Calculate expiration (default 12 hours)
        let expires_at = Utc::now()
            + Duration::hours(self.config.default_expiration_hours as i64);

        Some(ChangeForecast {
            service_id,
            service_name,
            endpoint,
            method,
            forecast_window_days,
            predicted_change_probability: change_probability,
            predicted_break_probability: break_probability,
            next_expected_change_date: next_change_date,
            next_expected_break_date: next_break_date,
            volatility_score: analysis.volatility_score,
            confidence,
            seasonal_patterns,
            predicted_at: Utc::now(),
            expires_at,
        })
    }

    /// Analyze historical patterns (for statistics generation)
    pub fn analyze_historical_patterns(
        &self,
        incidents: &[DriftIncident],
        window_start: DateTime<Utc>,
        window_end: DateTime<Utc>,
    ) -> PatternAnalysis {
        self.pattern_analyzer
            .analyze_patterns(incidents, window_start, window_end)
    }
}

impl Default for Forecaster {
    fn default() -> Self {
        Self::new(ForecastingConfig::default())
    }
}
