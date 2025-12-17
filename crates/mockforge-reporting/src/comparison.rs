//! Comparison reports for orchestration executions

use crate::pdf::ExecutionReport;
use crate::{ReportingError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Comparison report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonReport {
    pub baseline_run: ExecutionSummary,
    pub comparison_runs: Vec<ExecutionSummary>,
    pub metric_differences: Vec<MetricDifference>,
    pub regressions: Vec<Regression>,
    pub improvements: Vec<Improvement>,
    pub overall_assessment: ComparisonAssessment,
}

/// Execution summary for comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionSummary {
    pub orchestration_name: String,
    pub run_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub duration_seconds: u64,
    pub metrics_snapshot: HashMap<String, f64>,
}

/// Difference in a metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDifference {
    pub metric_name: String,
    pub baseline_value: f64,
    pub comparison_value: f64,
    pub absolute_difference: f64,
    pub percentage_difference: f64,
    pub direction: ChangeDirection,
    pub significance: SignificanceLevel,
}

/// Direction of change
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChangeDirection {
    Increase,
    Decrease,
    NoChange,
}

/// Statistical significance level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SignificanceLevel {
    NotSignificant,
    Low,
    Medium,
    High,
}

/// Performance regression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Regression {
    pub metric_name: String,
    pub baseline_value: f64,
    pub regressed_value: f64,
    pub impact_percentage: f64,
    pub severity: String,
    pub description: String,
}

/// Performance improvement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Improvement {
    pub metric_name: String,
    pub baseline_value: f64,
    pub improved_value: f64,
    pub improvement_percentage: f64,
    pub description: String,
}

/// Overall comparison assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonAssessment {
    pub verdict: ComparisonVerdict,
    pub summary: String,
    pub regressions_count: usize,
    pub improvements_count: usize,
    pub confidence: f64,
}

/// Comparison verdict
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ComparisonVerdict {
    Better,
    Worse,
    Similar,
    Mixed,
}

/// Comparison report generator
pub struct ComparisonReportGenerator {
    baseline: Option<ExecutionReport>,
}

impl ComparisonReportGenerator {
    /// Create a new comparison generator
    pub fn new() -> Self {
        Self { baseline: None }
    }

    /// Set baseline report
    pub fn set_baseline(&mut self, report: ExecutionReport) {
        self.baseline = Some(report);
    }

    /// Compare against baseline
    pub fn compare(&self, comparison_reports: Vec<ExecutionReport>) -> Result<ComparisonReport> {
        let baseline = self
            .baseline
            .as_ref()
            .ok_or_else(|| ReportingError::Analysis("No baseline set".to_string()))?;

        let baseline_summary = self.extract_summary(baseline);
        let comparison_summaries: Vec<_> =
            comparison_reports.iter().map(|r| self.extract_summary(r)).collect();

        // Calculate metric differences for each comparison
        let mut all_differences = Vec::new();
        let mut all_regressions = Vec::new();
        let mut all_improvements = Vec::new();

        for comp_summary in &comparison_summaries {
            let differences = self.calculate_differences(&baseline_summary, comp_summary);
            let (regressions, improvements) =
                self.identify_regressions_and_improvements(&differences);

            all_differences.extend(differences);
            all_regressions.extend(regressions);
            all_improvements.extend(improvements);
        }

        // Overall assessment
        let assessment = self.generate_assessment(&all_regressions, &all_improvements);

        Ok(ComparisonReport {
            baseline_run: baseline_summary,
            comparison_runs: comparison_summaries,
            metric_differences: all_differences,
            regressions: all_regressions,
            improvements: all_improvements,
            overall_assessment: assessment,
        })
    }

    /// Extract execution summary
    fn extract_summary(&self, report: &ExecutionReport) -> ExecutionSummary {
        let mut metrics_snapshot = HashMap::new();

        metrics_snapshot.insert("error_rate".to_string(), report.metrics.error_rate);
        metrics_snapshot.insert("avg_latency_ms".to_string(), report.metrics.avg_latency_ms);
        metrics_snapshot.insert("p95_latency_ms".to_string(), report.metrics.p95_latency_ms);
        metrics_snapshot.insert("p99_latency_ms".to_string(), report.metrics.p99_latency_ms);
        metrics_snapshot.insert("total_requests".to_string(), report.metrics.total_requests as f64);
        metrics_snapshot
            .insert("failed_requests".to_string(), report.metrics.failed_requests as f64);
        metrics_snapshot
            .insert("successful_requests".to_string(), report.metrics.successful_requests as f64);
        metrics_snapshot.insert("duration_seconds".to_string(), report.duration_seconds as f64);
        metrics_snapshot.insert("failed_steps".to_string(), report.failed_steps as f64);

        ExecutionSummary {
            orchestration_name: report.orchestration_name.clone(),
            run_id: format!("{}", report.start_time.timestamp()),
            timestamp: report.start_time,
            status: report.status.clone(),
            duration_seconds: report.duration_seconds,
            metrics_snapshot,
        }
    }

    /// Calculate differences between baseline and comparison
    fn calculate_differences(
        &self,
        baseline: &ExecutionSummary,
        comparison: &ExecutionSummary,
    ) -> Vec<MetricDifference> {
        let mut differences = Vec::new();

        for (metric_name, baseline_value) in &baseline.metrics_snapshot {
            if let Some(&comparison_value) = comparison.metrics_snapshot.get(metric_name) {
                let absolute_difference = comparison_value - baseline_value;
                let percentage_difference = if *baseline_value != 0.0 {
                    (absolute_difference / baseline_value) * 100.0
                } else if comparison_value != 0.0 {
                    100.0 // Changed from 0 to non-zero
                } else {
                    0.0
                };

                let direction = if absolute_difference > 0.0 {
                    ChangeDirection::Increase
                } else if absolute_difference < 0.0 {
                    ChangeDirection::Decrease
                } else {
                    ChangeDirection::NoChange
                };

                let significance = self.determine_significance(percentage_difference);

                differences.push(MetricDifference {
                    metric_name: metric_name.clone(),
                    baseline_value: *baseline_value,
                    comparison_value,
                    absolute_difference,
                    percentage_difference,
                    direction,
                    significance,
                });
            }
        }

        differences
    }

    /// Determine statistical significance
    fn determine_significance(&self, percentage_diff: f64) -> SignificanceLevel {
        let abs_diff = percentage_diff.abs();

        if abs_diff < 5.0 {
            SignificanceLevel::NotSignificant
        } else if abs_diff < 15.0 {
            SignificanceLevel::Low
        } else if abs_diff < 30.0 {
            SignificanceLevel::Medium
        } else {
            SignificanceLevel::High
        }
    }

    /// Identify regressions and improvements
    fn identify_regressions_and_improvements(
        &self,
        differences: &[MetricDifference],
    ) -> (Vec<Regression>, Vec<Improvement>) {
        let mut regressions = Vec::new();
        let mut improvements = Vec::new();

        for diff in differences {
            // Metrics where increase is bad
            let increase_is_bad = matches!(
                diff.metric_name.as_str(),
                "error_rate"
                    | "avg_latency_ms"
                    | "p95_latency_ms"
                    | "p99_latency_ms"
                    | "failed_requests"
                    | "duration_seconds"
                    | "failed_steps"
            );

            let is_significant = diff.significance != SignificanceLevel::NotSignificant;

            if !is_significant {
                continue;
            }

            match diff.direction {
                ChangeDirection::Increase if increase_is_bad => {
                    let severity = match diff.significance {
                        SignificanceLevel::High => "Critical",
                        SignificanceLevel::Medium => "High",
                        SignificanceLevel::Low => "Medium",
                        _ => "Low",
                    };

                    regressions.push(Regression {
                        metric_name: diff.metric_name.clone(),
                        baseline_value: diff.baseline_value,
                        regressed_value: diff.comparison_value,
                        impact_percentage: diff.percentage_difference,
                        severity: severity.to_string(),
                        description: format!(
                            "{} increased by {:.1}% (from {:.2} to {:.2})",
                            diff.metric_name,
                            diff.percentage_difference,
                            diff.baseline_value,
                            diff.comparison_value
                        ),
                    });
                }
                ChangeDirection::Decrease if !increase_is_bad => {
                    improvements.push(Improvement {
                        metric_name: diff.metric_name.clone(),
                        baseline_value: diff.baseline_value,
                        improved_value: diff.comparison_value,
                        improvement_percentage: diff.percentage_difference.abs(),
                        description: format!(
                            "{} decreased by {:.1}% (from {:.2} to {:.2})",
                            diff.metric_name,
                            diff.percentage_difference.abs(),
                            diff.baseline_value,
                            diff.comparison_value
                        ),
                    });
                }
                ChangeDirection::Increase if !increase_is_bad => {
                    improvements.push(Improvement {
                        metric_name: diff.metric_name.clone(),
                        baseline_value: diff.baseline_value,
                        improved_value: diff.comparison_value,
                        improvement_percentage: diff.percentage_difference,
                        description: format!(
                            "{} increased by {:.1}% (from {:.2} to {:.2})",
                            diff.metric_name,
                            diff.percentage_difference,
                            diff.baseline_value,
                            diff.comparison_value
                        ),
                    });
                }
                ChangeDirection::Decrease if increase_is_bad => {
                    improvements.push(Improvement {
                        metric_name: diff.metric_name.clone(),
                        baseline_value: diff.baseline_value,
                        improved_value: diff.comparison_value,
                        improvement_percentage: diff.percentage_difference.abs(),
                        description: format!(
                            "{} decreased by {:.1}% (from {:.2} to {:.2})",
                            diff.metric_name,
                            diff.percentage_difference.abs(),
                            diff.baseline_value,
                            diff.comparison_value
                        ),
                    });
                }
                _ => {}
            }
        }

        (regressions, improvements)
    }

    /// Generate overall assessment
    fn generate_assessment(
        &self,
        regressions: &[Regression],
        improvements: &[Improvement],
    ) -> ComparisonAssessment {
        let regressions_count = regressions.len();
        let improvements_count = improvements.len();

        let critical_regressions = regressions.iter().filter(|r| r.severity == "Critical").count();

        let verdict = if critical_regressions > 0 || regressions_count > improvements_count {
            ComparisonVerdict::Worse
        } else if improvements_count > regressions_count {
            ComparisonVerdict::Better
        } else if regressions_count > 0 && improvements_count > 0 {
            ComparisonVerdict::Mixed
        } else {
            ComparisonVerdict::Similar
        };

        let summary = match verdict {
            ComparisonVerdict::Better => {
                format!(
                    "Performance has improved with {} improvements and {} regressions detected.",
                    improvements_count, regressions_count
                )
            }
            ComparisonVerdict::Worse => {
                format!(
                    "Performance has degraded with {} regressions ({} critical) and {} improvements.",
                    regressions_count, critical_regressions, improvements_count
                )
            }
            ComparisonVerdict::Mixed => {
                format!(
                    "Mixed results with {} improvements and {} regressions.",
                    improvements_count, regressions_count
                )
            }
            ComparisonVerdict::Similar => {
                "Performance is similar to baseline with no significant changes.".to_string()
            }
        };

        let confidence = if regressions_count + improvements_count > 5 {
            0.9
        } else if regressions_count + improvements_count > 2 {
            0.7
        } else {
            0.5
        };

        ComparisonAssessment {
            verdict,
            summary,
            regressions_count,
            improvements_count,
            confidence,
        }
    }
}

impl Default for ComparisonReportGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::ReportMetrics;
    use chrono::Utc;

    fn create_baseline_report() -> ExecutionReport {
        ExecutionReport {
            orchestration_name: "test".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration_seconds: 100,
            status: "Completed".to_string(),
            total_steps: 5,
            completed_steps: 5,
            failed_steps: 0,
            metrics: ReportMetrics {
                total_requests: 1000,
                successful_requests: 980,
                failed_requests: 20,
                avg_latency_ms: 100.0,
                p95_latency_ms: 200.0,
                p99_latency_ms: 300.0,
                error_rate: 0.02,
            },
            failures: vec![],
            recommendations: vec![],
        }
    }

    #[test]
    fn test_comparison_report_generator() {
        let mut generator = ComparisonReportGenerator::new();
        let baseline = create_baseline_report();
        generator.set_baseline(baseline.clone());

        let comparison = ExecutionReport {
            metrics: ReportMetrics {
                total_requests: 1000,
                successful_requests: 990,
                failed_requests: 10,
                avg_latency_ms: 90.0,
                p95_latency_ms: 180.0,
                p99_latency_ms: 280.0,
                error_rate: 0.01,
            },
            ..baseline
        };

        let report = generator.compare(vec![comparison]).unwrap();

        assert!(!report.metric_differences.is_empty());
        assert_eq!(report.overall_assessment.verdict, ComparisonVerdict::Better);
    }

    #[test]
    fn test_comparison_generator_new() {
        let generator = ComparisonReportGenerator::new();
        assert!(generator.baseline.is_none());
    }

    #[test]
    fn test_comparison_generator_default() {
        let generator = ComparisonReportGenerator::default();
        assert!(generator.baseline.is_none());
    }

    #[test]
    fn test_comparison_no_baseline_error() {
        let generator = ComparisonReportGenerator::new();
        let result = generator.compare(vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_comparison_worse_verdict() {
        let mut generator = ComparisonReportGenerator::new();
        let baseline = create_baseline_report();
        generator.set_baseline(baseline.clone());

        // Create comparison with worse metrics
        let comparison = ExecutionReport {
            metrics: ReportMetrics {
                total_requests: 1000,
                successful_requests: 900,
                failed_requests: 100,  // Much more failures
                avg_latency_ms: 200.0, // Higher latency
                p95_latency_ms: 400.0,
                p99_latency_ms: 600.0,
                error_rate: 0.10, // Higher error rate
            },
            ..baseline
        };

        let report = generator.compare(vec![comparison]).unwrap();
        assert_eq!(report.overall_assessment.verdict, ComparisonVerdict::Worse);
        assert!(report.regressions.len() > 0);
    }

    #[test]
    fn test_comparison_similar_verdict() {
        let mut generator = ComparisonReportGenerator::new();
        let baseline = create_baseline_report();
        generator.set_baseline(baseline.clone());

        // Create comparison with nearly identical metrics
        let comparison = ExecutionReport {
            metrics: ReportMetrics {
                total_requests: 1000,
                successful_requests: 980,
                failed_requests: 20,
                avg_latency_ms: 101.0, // Almost the same
                p95_latency_ms: 201.0,
                p99_latency_ms: 301.0,
                error_rate: 0.0201,
            },
            ..baseline
        };

        let report = generator.compare(vec![comparison]).unwrap();
        assert_eq!(report.overall_assessment.verdict, ComparisonVerdict::Similar);
    }

    #[test]
    fn test_change_direction_enum() {
        // Test serialization
        let increase = ChangeDirection::Increase;
        let json = serde_json::to_string(&increase).unwrap();
        assert_eq!(json, "\"increase\"");

        let decrease = ChangeDirection::Decrease;
        let json = serde_json::to_string(&decrease).unwrap();
        assert_eq!(json, "\"decrease\"");

        let no_change = ChangeDirection::NoChange;
        let json = serde_json::to_string(&no_change).unwrap();
        assert_eq!(json, "\"nochange\"");
    }

    #[test]
    fn test_significance_level_enum() {
        let not_sig = SignificanceLevel::NotSignificant;
        let json = serde_json::to_string(&not_sig).unwrap();
        assert_eq!(json, "\"notsignificant\"");

        let high = SignificanceLevel::High;
        let json = serde_json::to_string(&high).unwrap();
        assert_eq!(json, "\"high\"");
    }

    #[test]
    fn test_comparison_verdict_enum() {
        let better = ComparisonVerdict::Better;
        let json = serde_json::to_string(&better).unwrap();
        assert_eq!(json, "\"better\"");

        let worse = ComparisonVerdict::Worse;
        let json = serde_json::to_string(&worse).unwrap();
        assert_eq!(json, "\"worse\"");

        let similar = ComparisonVerdict::Similar;
        let json = serde_json::to_string(&similar).unwrap();
        assert_eq!(json, "\"similar\"");

        let mixed = ComparisonVerdict::Mixed;
        let json = serde_json::to_string(&mixed).unwrap();
        assert_eq!(json, "\"mixed\"");
    }

    #[test]
    fn test_execution_summary_clone() {
        let summary = ExecutionSummary {
            orchestration_name: "test".to_string(),
            run_id: "123".to_string(),
            timestamp: Utc::now(),
            status: "Completed".to_string(),
            duration_seconds: 100,
            metrics_snapshot: HashMap::new(),
        };

        let cloned = summary.clone();
        assert_eq!(summary.orchestration_name, cloned.orchestration_name);
        assert_eq!(summary.run_id, cloned.run_id);
    }

    #[test]
    fn test_metric_difference_clone() {
        let diff = MetricDifference {
            metric_name: "error_rate".to_string(),
            baseline_value: 0.02,
            comparison_value: 0.01,
            absolute_difference: -0.01,
            percentage_difference: -50.0,
            direction: ChangeDirection::Decrease,
            significance: SignificanceLevel::High,
        };

        let cloned = diff.clone();
        assert_eq!(diff.metric_name, cloned.metric_name);
        assert_eq!(diff.baseline_value, cloned.baseline_value);
    }

    #[test]
    fn test_regression_clone() {
        let regression = Regression {
            metric_name: "latency".to_string(),
            baseline_value: 100.0,
            regressed_value: 200.0,
            impact_percentage: 100.0,
            severity: "High".to_string(),
            description: "Latency doubled".to_string(),
        };

        let cloned = regression.clone();
        assert_eq!(regression.metric_name, cloned.metric_name);
        assert_eq!(regression.severity, cloned.severity);
    }

    #[test]
    fn test_improvement_clone() {
        let improvement = Improvement {
            metric_name: "error_rate".to_string(),
            baseline_value: 0.10,
            improved_value: 0.02,
            improvement_percentage: 80.0,
            description: "Error rate improved".to_string(),
        };

        let cloned = improvement.clone();
        assert_eq!(improvement.metric_name, cloned.metric_name);
        assert_eq!(improvement.improvement_percentage, cloned.improvement_percentage);
    }

    #[test]
    fn test_comparison_assessment_clone() {
        let assessment = ComparisonAssessment {
            verdict: ComparisonVerdict::Better,
            summary: "Performance improved".to_string(),
            regressions_count: 0,
            improvements_count: 5,
            confidence: 0.9,
        };

        let cloned = assessment.clone();
        assert_eq!(assessment.verdict, cloned.verdict);
        assert_eq!(assessment.confidence, cloned.confidence);
    }

    #[test]
    fn test_comparison_report_serialize() {
        let mut generator = ComparisonReportGenerator::new();
        let baseline = create_baseline_report();
        generator.set_baseline(baseline.clone());

        let report = generator.compare(vec![baseline.clone()]).unwrap();
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("baseline_run"));
        assert!(json.contains("comparison_runs"));
    }

    #[test]
    fn test_multiple_comparisons() {
        let mut generator = ComparisonReportGenerator::new();
        let baseline = create_baseline_report();
        generator.set_baseline(baseline.clone());

        let comparison1 = ExecutionReport {
            metrics: ReportMetrics {
                avg_latency_ms: 90.0,
                ..baseline.metrics.clone()
            },
            ..baseline.clone()
        };

        let comparison2 = ExecutionReport {
            metrics: ReportMetrics {
                avg_latency_ms: 110.0,
                ..baseline.metrics.clone()
            },
            ..baseline.clone()
        };

        let report = generator.compare(vec![comparison1, comparison2]).unwrap();
        assert_eq!(report.comparison_runs.len(), 2);
    }
}
