//! CSV export for reports and metrics

use crate::Result;
use crate::comparison::ComparisonReport;
use crate::pdf::{ExecutionReport, ReportMetrics};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;

/// CSV export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvExportConfig {
    pub delimiter: char,
    pub include_headers: bool,
    pub quote_strings: bool,
}

impl Default for CsvExportConfig {
    fn default() -> Self {
        Self {
            delimiter: ',',
            include_headers: true,
            quote_strings: true,
        }
    }
}

/// CSV exporter
pub struct CsvExporter {
    config: CsvExportConfig,
}

impl CsvExporter {
    /// Create a new CSV exporter
    pub fn new(config: CsvExportConfig) -> Self {
        Self { config }
    }

    /// Export execution report to CSV
    pub fn export_execution_report(
        &self,
        report: &ExecutionReport,
        output_path: &str,
    ) -> Result<()> {
        let mut file = File::create(output_path)?;

        // Write header
        if self.config.include_headers {
            writeln!(
                file,
                "orchestration_name,start_time,end_time,duration_seconds,status,total_steps,completed_steps,failed_steps,total_requests,successful_requests,failed_requests,error_rate,avg_latency_ms,p95_latency_ms,p99_latency_ms"
            )?;
        }

        // Write data row
        writeln!(
            file,
            "{},{},{},{},{},{},{},{},{},{},{},{:.4},{:.2},{:.2},{:.2}",
            self.quote_if_needed(&report.orchestration_name),
            report.start_time.to_rfc3339(),
            report.end_time.to_rfc3339(),
            report.duration_seconds,
            self.quote_if_needed(&report.status),
            report.total_steps,
            report.completed_steps,
            report.failed_steps,
            report.metrics.total_requests,
            report.metrics.successful_requests,
            report.metrics.failed_requests,
            report.metrics.error_rate,
            report.metrics.avg_latency_ms,
            report.metrics.p95_latency_ms,
            report.metrics.p99_latency_ms,
        )?;

        Ok(())
    }

    /// Export multiple execution reports to CSV
    pub fn export_execution_reports(
        &self,
        reports: &[ExecutionReport],
        output_path: &str,
    ) -> Result<()> {
        let mut file = File::create(output_path)?;

        // Write header
        if self.config.include_headers {
            writeln!(
                file,
                "orchestration_name,start_time,end_time,duration_seconds,status,total_steps,completed_steps,failed_steps,total_requests,successful_requests,failed_requests,error_rate,avg_latency_ms,p95_latency_ms,p99_latency_ms"
            )?;
        }

        // Write data rows
        for report in reports {
            writeln!(
                file,
                "{},{},{},{},{},{},{},{},{},{},{},{:.4},{:.2},{:.2},{:.2}",
                self.quote_if_needed(&report.orchestration_name),
                report.start_time.to_rfc3339(),
                report.end_time.to_rfc3339(),
                report.duration_seconds,
                self.quote_if_needed(&report.status),
                report.total_steps,
                report.completed_steps,
                report.failed_steps,
                report.metrics.total_requests,
                report.metrics.successful_requests,
                report.metrics.failed_requests,
                report.metrics.error_rate,
                report.metrics.avg_latency_ms,
                report.metrics.p95_latency_ms,
                report.metrics.p99_latency_ms,
            )?;
        }

        Ok(())
    }

    /// Export comparison report to CSV
    pub fn export_comparison_report(
        &self,
        report: &ComparisonReport,
        output_path: &str,
    ) -> Result<()> {
        let mut file = File::create(output_path)?;

        // Write header
        if self.config.include_headers {
            writeln!(
                file,
                "metric_name,baseline_value,comparison_value,absolute_difference,percentage_difference,direction,significance"
            )?;
        }

        // Write metric differences
        for diff in &report.metric_differences {
            writeln!(
                file,
                "{},{:.4},{:.4},{:.4},{:.2},{:?},{:?}",
                self.quote_if_needed(&diff.metric_name),
                diff.baseline_value,
                diff.comparison_value,
                diff.absolute_difference,
                diff.percentage_difference,
                diff.direction,
                diff.significance,
            )?;
        }

        Ok(())
    }

    /// Export metrics time series to CSV
    pub fn export_metrics_time_series(
        &self,
        metrics: &[(i64, ReportMetrics)],
        output_path: &str,
    ) -> Result<()> {
        let mut file = File::create(output_path)?;

        // Write header
        if self.config.include_headers {
            writeln!(
                file,
                "timestamp,total_requests,successful_requests,failed_requests,error_rate,avg_latency_ms,p95_latency_ms,p99_latency_ms"
            )?;
        }

        // Write time series data
        for (timestamp, metric) in metrics {
            writeln!(
                file,
                "{},{},{},{},{:.4},{:.2},{:.2},{:.2}",
                timestamp,
                metric.total_requests,
                metric.successful_requests,
                metric.failed_requests,
                metric.error_rate,
                metric.avg_latency_ms,
                metric.p95_latency_ms,
                metric.p99_latency_ms,
            )?;
        }

        Ok(())
    }

    /// Export regressions to CSV
    pub fn export_regressions(
        &self,
        report: &ComparisonReport,
        output_path: &str,
    ) -> Result<()> {
        let mut file = File::create(output_path)?;

        // Write header
        if self.config.include_headers {
            writeln!(
                file,
                "metric_name,baseline_value,regressed_value,impact_percentage,severity,description"
            )?;
        }

        // Write regressions
        for regression in &report.regressions {
            writeln!(
                file,
                "{},{:.4},{:.4},{:.2},{},{}",
                self.quote_if_needed(&regression.metric_name),
                regression.baseline_value,
                regression.regressed_value,
                regression.impact_percentage,
                self.quote_if_needed(&regression.severity),
                self.quote_if_needed(&regression.description),
            )?;
        }

        Ok(())
    }

    /// Export improvements to CSV
    pub fn export_improvements(
        &self,
        report: &ComparisonReport,
        output_path: &str,
    ) -> Result<()> {
        let mut file = File::create(output_path)?;

        // Write header
        if self.config.include_headers {
            writeln!(
                file,
                "metric_name,baseline_value,improved_value,improvement_percentage,description"
            )?;
        }

        // Write improvements
        for improvement in &report.improvements {
            writeln!(
                file,
                "{},{:.4},{:.4},{:.2},{}",
                self.quote_if_needed(&improvement.metric_name),
                improvement.baseline_value,
                improvement.improved_value,
                improvement.improvement_percentage,
                self.quote_if_needed(&improvement.description),
            )?;
        }

        Ok(())
    }

    /// Quote string if configuration requires it
    fn quote_if_needed(&self, s: &str) -> String {
        if self.config.quote_strings {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    }
}

impl Default for CsvExporter {
    fn default() -> Self {
        Self::new(CsvExportConfig::default())
    }
}

/// CSV batch exporter for multiple reports
pub struct CsvBatchExporter {
    exporter: CsvExporter,
}

impl CsvBatchExporter {
    /// Create a new batch exporter
    pub fn new(config: CsvExportConfig) -> Self {
        Self {
            exporter: CsvExporter::new(config),
        }
    }

    /// Export all reports to a directory
    pub fn export_all(
        &self,
        execution_reports: &[ExecutionReport],
        comparison_report: Option<&ComparisonReport>,
        output_dir: &str,
    ) -> Result<()> {
        // Create output directory if it doesn't exist
        std::fs::create_dir_all(output_dir)?;

        // Export execution reports
        let exec_path = format!("{}/execution_reports.csv", output_dir);
        self.exporter.export_execution_reports(execution_reports, &exec_path)?;

        // Export comparison report if available
        if let Some(comparison) = comparison_report {
            let comp_path = format!("{}/comparison.csv", output_dir);
            self.exporter.export_comparison_report(comparison, &comp_path)?;

            let reg_path = format!("{}/regressions.csv", output_dir);
            self.exporter.export_regressions(comparison, &reg_path)?;

            let imp_path = format!("{}/improvements.csv", output_dir);
            self.exporter.export_improvements(comparison, &imp_path)?;
        }

        Ok(())
    }
}

impl Default for CsvBatchExporter {
    fn default() -> Self {
        Self::new(CsvExportConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::tempdir;

    #[test]
    fn test_csv_export_execution_report() {
        let config = CsvExportConfig::default();
        let exporter = CsvExporter::new(config);

        let report = ExecutionReport {
            orchestration_name: "test-orch".to_string(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            duration_seconds: 120,
            status: "Completed".to_string(),
            total_steps: 5,
            completed_steps: 5,
            failed_steps: 0,
            metrics: ReportMetrics {
                total_requests: 1000,
                successful_requests: 980,
                failed_requests: 20,
                avg_latency_ms: 125.5,
                p95_latency_ms: 250.0,
                p99_latency_ms: 350.0,
                error_rate: 0.02,
            },
            failures: vec![],
            recommendations: vec![],
        };

        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("report.csv");

        let result = exporter.export_execution_report(&report, output_path.to_str().unwrap());
        assert!(result.is_ok());
        assert!(output_path.exists());

        // Verify content
        let content = std::fs::read_to_string(output_path).unwrap();
        assert!(content.contains("orchestration_name"));
        assert!(content.contains("test-orch"));
    }

    #[test]
    fn test_csv_export_multiple_reports() {
        let config = CsvExportConfig::default();
        let exporter = CsvExporter::new(config);

        let reports = vec![
            ExecutionReport {
                orchestration_name: "test-1".to_string(),
                start_time: Utc::now(),
                end_time: Utc::now(),
                duration_seconds: 100,
                status: "Completed".to_string(),
                total_steps: 3,
                completed_steps: 3,
                failed_steps: 0,
                metrics: ReportMetrics {
                    total_requests: 500,
                    successful_requests: 490,
                    failed_requests: 10,
                    avg_latency_ms: 100.0,
                    p95_latency_ms: 200.0,
                    p99_latency_ms: 300.0,
                    error_rate: 0.02,
                },
                failures: vec![],
                recommendations: vec![],
            },
            ExecutionReport {
                orchestration_name: "test-2".to_string(),
                start_time: Utc::now(),
                end_time: Utc::now(),
                duration_seconds: 150,
                status: "Completed".to_string(),
                total_steps: 4,
                completed_steps: 4,
                failed_steps: 0,
                metrics: ReportMetrics {
                    total_requests: 750,
                    successful_requests: 740,
                    failed_requests: 10,
                    avg_latency_ms: 110.0,
                    p95_latency_ms: 220.0,
                    p99_latency_ms: 320.0,
                    error_rate: 0.013,
                },
                failures: vec![],
                recommendations: vec![],
            },
        ];

        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("reports.csv");

        let result = exporter.export_execution_reports(&reports, output_path.to_str().unwrap());
        assert!(result.is_ok());

        let content = std::fs::read_to_string(output_path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), 3); // Header + 2 data rows
    }
}
