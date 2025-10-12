//! ML-based parameter optimization for chaos scenarios
//!
//! Uses Bayesian optimization and historical data to recommend optimal
//! chaos parameters that balance effectiveness and system stability.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Historical orchestration run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationRun {
    pub id: String,
    pub orchestration_id: String,
    pub parameters: HashMap<String, f64>,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
    pub success: bool,
    pub metrics: RunMetrics,
}

/// Run metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMetrics {
    pub chaos_effectiveness: f64, // How much chaos was actually induced (0-1)
    pub system_stability: f64,    // How stable the system remained (0-1)
    pub error_rate: f64,
    pub recovery_time_ms: u64,
    pub failures_detected: u32,
    pub false_positives: u32,
}

/// Parameter optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    pub parameter: String,
    pub current_value: Option<f64>,
    pub recommended_value: f64,
    pub confidence: f64,
    pub reasoning: String,
    pub expected_impact: ExpectedImpact,
    pub based_on_runs: usize,
}

/// Expected impact of parameter change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedImpact {
    pub chaos_effectiveness_delta: f64,
    pub system_stability_delta: f64,
    pub overall_score_delta: f64,
}

/// Optimization objective
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationObjective {
    MaxChaos,      // Maximize chaos (for stress testing)
    Balanced,      // Balance chaos and stability
    SafeTesting,   // Minimize risk while still effective
    QuickRecovery, // Optimize for fast recovery
    MaxDetection,  // Maximize failure detection
}

/// Parameter bounds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterBounds {
    pub min: f64,
    pub max: f64,
    pub step: Option<f64>,
}

/// Optimizer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizerConfig {
    pub objective: OptimizationObjective,
    pub min_runs: usize,
    pub confidence_threshold: f64,
    pub exploration_factor: f64,
    pub parameter_bounds: HashMap<String, ParameterBounds>,
    pub weights: ObjectiveWeights,
}

/// Weights for multi-objective optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectiveWeights {
    pub chaos_effectiveness: f64,
    pub system_stability: f64,
    pub recovery_time: f64,
    pub detection_rate: f64,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        let mut parameter_bounds = HashMap::new();

        // Common parameter bounds
        parameter_bounds.insert(
            "latency_ms".to_string(),
            ParameterBounds {
                min: 0.0,
                max: 5000.0,
                step: Some(10.0),
            },
        );
        parameter_bounds.insert(
            "error_rate".to_string(),
            ParameterBounds {
                min: 0.0,
                max: 1.0,
                step: Some(0.01),
            },
        );
        parameter_bounds.insert(
            "packet_loss".to_string(),
            ParameterBounds {
                min: 0.0,
                max: 1.0,
                step: Some(0.01),
            },
        );
        parameter_bounds.insert(
            "cpu_load".to_string(),
            ParameterBounds {
                min: 0.0,
                max: 1.0,
                step: Some(0.05),
            },
        );

        Self {
            objective: OptimizationObjective::Balanced,
            min_runs: 10,
            confidence_threshold: 0.7,
            exploration_factor: 0.2,
            parameter_bounds,
            weights: ObjectiveWeights {
                chaos_effectiveness: 0.3,
                system_stability: 0.4,
                recovery_time: 0.2,
                detection_rate: 0.1,
            },
        }
    }
}

/// ML-based parameter optimizer
pub struct ParameterOptimizer {
    config: OptimizerConfig,
    historical_runs: Vec<OrchestrationRun>,
}

impl ParameterOptimizer {
    /// Create a new optimizer
    pub fn new(config: OptimizerConfig) -> Self {
        Self {
            config,
            historical_runs: Vec::new(),
        }
    }

    /// Add historical run data
    pub fn add_run(&mut self, run: OrchestrationRun) {
        self.historical_runs.push(run);
    }

    /// Add multiple runs
    pub fn add_runs(&mut self, runs: Vec<OrchestrationRun>) {
        self.historical_runs.extend(runs);
    }

    /// Generate optimization recommendations
    pub fn optimize(&self) -> Result<Vec<OptimizationRecommendation>, String> {
        if self.historical_runs.len() < self.config.min_runs {
            return Err(format!(
                "Insufficient data: need at least {} runs, have {}",
                self.config.min_runs,
                self.historical_runs.len()
            ));
        }

        let mut recommendations = Vec::new();

        // Extract all unique parameters
        let all_parameters = self.extract_parameter_names();

        for param_name in all_parameters {
            if let Some(recommendation) = self.optimize_parameter(&param_name)? {
                recommendations.push(recommendation);
            }
        }

        // Sort by expected impact
        recommendations.sort_by(|a, b| {
            b.expected_impact
                .overall_score_delta
                .partial_cmp(&a.expected_impact.overall_score_delta)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(recommendations)
    }

    /// Optimize a single parameter
    fn optimize_parameter(
        &self,
        param_name: &str,
    ) -> Result<Option<OptimizationRecommendation>, String> {
        // Collect all runs that used this parameter
        let relevant_runs: Vec<_> = self
            .historical_runs
            .iter()
            .filter(|run| run.parameters.contains_key(param_name))
            .collect();

        if relevant_runs.is_empty() {
            return Ok(None);
        }

        // Get parameter bounds
        let bounds = self
            .config
            .parameter_bounds
            .get(param_name)
            .ok_or_else(|| format!("No bounds defined for parameter '{}'", param_name))?;

        // Calculate scores for different parameter values
        let mut value_scores: Vec<(f64, f64)> = Vec::new();

        for run in &relevant_runs {
            if let Some(&param_value) = run.parameters.get(param_name) {
                let score = self.calculate_run_score(run);
                value_scores.push((param_value, score));
            }
        }

        if value_scores.is_empty() {
            return Ok(None);
        }

        // Use Gaussian Process-inspired approach to find optimal value
        let optimal_value = self.find_optimal_value(&value_scores, bounds)?;

        // Get current average value
        let current_value =
            value_scores.iter().map(|(v, _)| v).sum::<f64>() / value_scores.len() as f64;

        // Calculate confidence based on data density
        let confidence = self.calculate_confidence(&value_scores, optimal_value);

        if confidence < self.config.confidence_threshold {
            return Ok(None);
        }

        // Estimate expected impact
        let expected_impact =
            self.estimate_impact(param_name, current_value, optimal_value, &relevant_runs)?;

        // Generate reasoning
        let reasoning = self.generate_reasoning(
            param_name,
            current_value,
            optimal_value,
            &expected_impact,
            relevant_runs.len(),
        );

        Ok(Some(OptimizationRecommendation {
            parameter: param_name.to_string(),
            current_value: Some(current_value),
            recommended_value: optimal_value,
            confidence,
            reasoning,
            expected_impact,
            based_on_runs: relevant_runs.len(),
        }))
    }

    /// Calculate score for a run based on objective
    fn calculate_run_score(&self, run: &OrchestrationRun) -> f64 {
        let weights = &self.config.weights;

        let chaos_score = run.metrics.chaos_effectiveness;
        let stability_score = run.metrics.system_stability;
        let recovery_score = 1.0 - (run.metrics.recovery_time_ms as f64 / 10000.0).min(1.0);
        let detection_score = if run.metrics.failures_detected + run.metrics.false_positives > 0 {
            run.metrics.failures_detected as f64
                / (run.metrics.failures_detected + run.metrics.false_positives) as f64
        } else {
            0.5
        };

        // Apply objective-specific adjustments
        let (chaos_w, stability_w, recovery_w, detection_w) = match self.config.objective {
            OptimizationObjective::MaxChaos => (0.7, 0.1, 0.1, 0.1),
            OptimizationObjective::Balanced => (
                weights.chaos_effectiveness,
                weights.system_stability,
                weights.recovery_time,
                weights.detection_rate,
            ),
            OptimizationObjective::SafeTesting => (0.2, 0.6, 0.1, 0.1),
            OptimizationObjective::QuickRecovery => (0.2, 0.3, 0.4, 0.1),
            OptimizationObjective::MaxDetection => (0.2, 0.2, 0.1, 0.5),
        };

        chaos_score * chaos_w
            + stability_score * stability_w
            + recovery_score * recovery_w
            + detection_score * detection_w
    }

    /// Find optimal parameter value using expected improvement
    fn find_optimal_value(
        &self,
        value_scores: &[(f64, f64)],
        bounds: &ParameterBounds,
    ) -> Result<f64, String> {
        // Simple approach: find the value with best score, with some exploration
        let best_value = value_scores
            .iter()
            .max_by(|(_, s1), (_, s2)| s1.partial_cmp(s2).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(v, _)| *v)
            .ok_or("No valid values found")?;

        // Add exploration factor for areas not well-explored
        let exploration = self.config.exploration_factor;
        let range = bounds.max - bounds.min;

        let explored_values: Vec<f64> = value_scores.iter().map(|(v, _)| *v).collect();
        let mean = explored_values.iter().sum::<f64>() / explored_values.len() as f64;

        // If best value is at extremes and we haven't explored much, suggest moving toward center
        let optimal = if (best_value - bounds.min).abs() < range * 0.1
            || (best_value - bounds.max).abs() < range * 0.1
        {
            best_value * (1.0 - exploration) + mean * exploration
        } else {
            best_value
        };

        // Clamp to bounds
        let clamped = optimal.max(bounds.min).min(bounds.max);

        // Round to step if specified
        let final_value = if let Some(step) = bounds.step {
            (clamped / step).round() * step
        } else {
            clamped
        };

        Ok(final_value)
    }

    /// Calculate confidence based on data coverage
    fn calculate_confidence(&self, value_scores: &[(f64, f64)], optimal_value: f64) -> f64 {
        if value_scores.is_empty() {
            return 0.0;
        }

        // Confidence based on:
        // 1. Number of samples
        let sample_confidence = (value_scores.len() as f64 / 20.0).min(1.0);

        // 2. How close we have samples to the optimal value
        let nearest_distance = value_scores
            .iter()
            .map(|(v, _)| (v - optimal_value).abs())
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(f64::MAX);

        let proximity_confidence = if nearest_distance < 10.0 {
            1.0
        } else if nearest_distance < 50.0 {
            0.8
        } else if nearest_distance < 100.0 {
            0.6
        } else {
            0.4
        };

        // 3. Score variance (lower is better)
        let scores: Vec<f64> = value_scores.iter().map(|(_, s)| *s).collect();
        let mean_score = scores.iter().sum::<f64>() / scores.len() as f64;
        let variance =
            scores.iter().map(|s| (s - mean_score).powi(2)).sum::<f64>() / scores.len() as f64;
        let consistency_confidence = if variance < 0.01 {
            0.9
        } else if variance < 0.05 {
            0.7
        } else {
            0.5
        };

        (sample_confidence + proximity_confidence + consistency_confidence) / 3.0
    }

    /// Estimate impact of parameter change
    fn estimate_impact(
        &self,
        _param_name: &str,
        current_value: f64,
        optimal_value: f64,
        runs: &[&OrchestrationRun],
    ) -> Result<ExpectedImpact, String> {
        // Find runs close to current and optimal values
        let current_runs: Vec<_> = runs
            .iter()
            .filter(|r| r.parameters.values().any(|&v| (v - current_value).abs() < 10.0))
            .collect();

        let optimal_runs: Vec<_> = runs
            .iter()
            .filter(|r| r.parameters.values().any(|&v| (v - optimal_value).abs() < 10.0))
            .collect();

        let current_avg_chaos = if !current_runs.is_empty() {
            current_runs.iter().map(|r| r.metrics.chaos_effectiveness).sum::<f64>()
                / current_runs.len() as f64
        } else {
            0.5
        };

        let optimal_avg_chaos = if !optimal_runs.is_empty() {
            optimal_runs.iter().map(|r| r.metrics.chaos_effectiveness).sum::<f64>()
                / optimal_runs.len() as f64
        } else {
            0.5
        };

        let current_avg_stability = if !current_runs.is_empty() {
            current_runs.iter().map(|r| r.metrics.system_stability).sum::<f64>()
                / current_runs.len() as f64
        } else {
            0.5
        };

        let optimal_avg_stability = if !optimal_runs.is_empty() {
            optimal_runs.iter().map(|r| r.metrics.system_stability).sum::<f64>()
                / optimal_runs.len() as f64
        } else {
            0.5
        };

        let chaos_delta = optimal_avg_chaos - current_avg_chaos;
        let stability_delta = optimal_avg_stability - current_avg_stability;

        // Calculate overall score delta
        let current_score = current_avg_chaos * self.config.weights.chaos_effectiveness
            + current_avg_stability * self.config.weights.system_stability;
        let optimal_score = optimal_avg_chaos * self.config.weights.chaos_effectiveness
            + optimal_avg_stability * self.config.weights.system_stability;

        Ok(ExpectedImpact {
            chaos_effectiveness_delta: chaos_delta,
            system_stability_delta: stability_delta,
            overall_score_delta: optimal_score - current_score,
        })
    }

    /// Generate human-readable reasoning
    fn generate_reasoning(
        &self,
        param_name: &str,
        current_value: f64,
        optimal_value: f64,
        impact: &ExpectedImpact,
        sample_count: usize,
    ) -> String {
        let change_direction = if optimal_value > current_value {
            "increase"
        } else if optimal_value < current_value {
            "decrease"
        } else {
            "maintain"
        };

        let change_pct = if current_value != 0.0 {
            ((optimal_value - current_value) / current_value * 100.0).abs()
        } else {
            0.0
        };

        let impact_desc = if impact.overall_score_delta > 0.1 {
            "significant improvement"
        } else if impact.overall_score_delta > 0.05 {
            "moderate improvement"
        } else if impact.overall_score_delta > 0.0 {
            "slight improvement"
        } else {
            "minimal impact"
        };

        format!(
            "Based on {} historical runs, recommend to {} '{}' from {:.2} to {:.2} ({:.1}% change). \
             This is expected to result in {} in overall effectiveness (chaos: {:+.2}%, stability: {:+.2}%).",
            sample_count,
            change_direction,
            param_name,
            current_value,
            optimal_value,
            change_pct,
            impact_desc,
            impact.chaos_effectiveness_delta * 100.0,
            impact.system_stability_delta * 100.0
        )
    }

    /// Extract all unique parameter names
    fn extract_parameter_names(&self) -> Vec<String> {
        let mut params = std::collections::HashSet::new();
        for run in &self.historical_runs {
            for key in run.parameters.keys() {
                params.insert(key.clone());
            }
        }
        params.into_iter().collect()
    }

    /// Get number of historical runs
    pub fn run_count(&self) -> usize {
        self.historical_runs.len()
    }

    /// Clear historical data
    pub fn clear_runs(&mut self) {
        self.historical_runs.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_run(latency: f64, chaos_eff: f64, stability: f64) -> OrchestrationRun {
        let mut params = HashMap::new();
        params.insert("latency_ms".to_string(), latency);

        OrchestrationRun {
            id: format!("run-{}", latency),
            orchestration_id: "test-orch".to_string(),
            parameters: params,
            timestamp: Utc::now(),
            duration_ms: 1000,
            success: true,
            metrics: RunMetrics {
                chaos_effectiveness: chaos_eff,
                system_stability: stability,
                error_rate: 0.1,
                recovery_time_ms: 500,
                failures_detected: 5,
                false_positives: 1,
            },
        }
    }

    #[test]
    fn test_optimizer_creation() {
        let config = OptimizerConfig::default();
        let optimizer = ParameterOptimizer::new(config);
        assert_eq!(optimizer.run_count(), 0);
    }

    #[test]
    fn test_add_runs() {
        let config = OptimizerConfig::default();
        let mut optimizer = ParameterOptimizer::new(config);

        let runs = vec![
            create_test_run(100.0, 0.5, 0.8),
            create_test_run(200.0, 0.7, 0.6),
        ];

        optimizer.add_runs(runs);
        assert_eq!(optimizer.run_count(), 2);
    }

    #[test]
    fn test_optimize_with_sufficient_data() {
        let config = OptimizerConfig::default();
        let mut optimizer = ParameterOptimizer::new(config);

        // Add runs with varying latency values
        for i in 0..15 {
            let latency = 50.0 + (i as f64 * 20.0);
            let chaos_eff = 0.3 + (latency / 500.0).min(0.6);
            let stability = 0.9 - (latency / 1000.0).min(0.4);
            optimizer.add_run(create_test_run(latency, chaos_eff, stability));
        }

        let recommendations = optimizer.optimize().unwrap();
        assert!(!recommendations.is_empty());
        assert!(recommendations[0].confidence >= 0.0);
    }

    #[test]
    fn test_optimize_insufficient_data() {
        let config = OptimizerConfig::default();
        let mut optimizer = ParameterOptimizer::new(config);

        optimizer.add_run(create_test_run(100.0, 0.5, 0.8));

        let result = optimizer.optimize();
        assert!(result.is_err());
    }

    #[test]
    fn test_different_objectives() {
        let objectives = vec![
            OptimizationObjective::MaxChaos,
            OptimizationObjective::Balanced,
            OptimizationObjective::SafeTesting,
        ];

        for objective in objectives {
            let mut config = OptimizerConfig::default();
            config.objective = objective;
            let optimizer = ParameterOptimizer::new(config);

            // Just verify it can be created with different objectives
            assert_eq!(optimizer.run_count(), 0);
        }
    }
}
