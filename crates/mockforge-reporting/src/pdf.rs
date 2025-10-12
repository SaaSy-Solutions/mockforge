//! PDF report generation for orchestration execution results

use crate::{ReportingError, Result};
use chrono::{DateTime, Utc};
use printpdf::*;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufWriter;

/// PDF generation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfConfig {
    pub title: String,
    pub author: String,
    pub include_charts: bool,
    pub include_metrics: bool,
    pub include_recommendations: bool,
}

impl Default for PdfConfig {
    fn default() -> Self {
        Self {
            title: "Chaos Orchestration Report".to_string(),
            author: "MockForge".to_string(),
            include_charts: true,
            include_metrics: true,
            include_recommendations: true,
        }
    }
}

/// Execution report data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub orchestration_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_seconds: u64,
    pub status: String,
    pub total_steps: usize,
    pub completed_steps: usize,
    pub failed_steps: usize,
    pub metrics: ReportMetrics,
    pub failures: Vec<FailureDetail>,
    pub recommendations: Vec<String>,
}

/// Report metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub avg_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub error_rate: f64,
}

/// Failure detail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureDetail {
    pub step_name: String,
    pub error_message: String,
    pub timestamp: DateTime<Utc>,
}

/// PDF report generator
pub struct PdfReportGenerator {
    config: PdfConfig,
}

impl PdfReportGenerator {
    /// Create a new PDF generator
    pub fn new(config: PdfConfig) -> Self {
        Self { config }
    }

    /// Generate PDF report from execution data
    pub fn generate(&self, report: &ExecutionReport, output_path: &str) -> Result<()> {
        let (doc, page1, layer1) =
            PdfDocument::new(&self.config.title, Mm(210.0), Mm(297.0), "Layer 1");

        let font = doc
            .add_builtin_font(BuiltinFont::Helvetica)
            .map_err(|e| ReportingError::Pdf(e.to_string()))?;
        let font_bold = doc
            .add_builtin_font(BuiltinFont::HelveticaBold)
            .map_err(|e| ReportingError::Pdf(e.to_string()))?;

        let current_layer = doc.get_page(page1).get_layer(layer1);

        // Title
        current_layer.use_text(&self.config.title, 24.0, Mm(20.0), Mm(270.0), &font_bold);

        // Metadata
        let mut y = 255.0;
        current_layer.use_text(
            format!("Orchestration: {}", report.orchestration_name),
            12.0,
            Mm(20.0),
            Mm(y),
            &font,
        );

        y -= 7.0;
        current_layer.use_text(
            format!("Start: {}", report.start_time.format("%Y-%m-%d %H:%M:%S UTC")),
            10.0,
            Mm(20.0),
            Mm(y),
            &font,
        );

        y -= 5.0;
        current_layer.use_text(
            format!("End: {}", report.end_time.format("%Y-%m-%d %H:%M:%S UTC")),
            10.0,
            Mm(20.0),
            Mm(y),
            &font,
        );

        y -= 5.0;
        current_layer.use_text(
            format!("Duration: {}s", report.duration_seconds),
            10.0,
            Mm(20.0),
            Mm(y),
            &font,
        );

        y -= 5.0;
        current_layer.use_text(
            format!("Status: {}", report.status),
            10.0,
            Mm(20.0),
            Mm(y),
            &font_bold,
        );

        // Summary section
        y -= 15.0;
        current_layer.use_text("Summary", 14.0, Mm(20.0), Mm(y), &font_bold);

        y -= 7.0;
        current_layer.use_text(
            format!("Total Steps: {}", report.total_steps),
            10.0,
            Mm(20.0),
            Mm(y),
            &font,
        );

        y -= 5.0;
        current_layer.use_text(
            format!("Completed: {}", report.completed_steps),
            10.0,
            Mm(20.0),
            Mm(y),
            &font,
        );

        y -= 5.0;
        current_layer.use_text(
            format!("Failed: {}", report.failed_steps),
            10.0,
            Mm(20.0),
            Mm(y),
            &font,
        );

        // Metrics section
        if self.config.include_metrics {
            y -= 15.0;
            current_layer.use_text("Metrics", 14.0, Mm(20.0), Mm(y), &font_bold);

            y -= 7.0;
            current_layer.use_text(
                format!("Total Requests: {}", report.metrics.total_requests),
                10.0,
                Mm(20.0),
                Mm(y),
                &font,
            );

            y -= 5.0;
            current_layer.use_text(
                format!("Error Rate: {:.2}%", report.metrics.error_rate * 100.0),
                10.0,
                Mm(20.0),
                Mm(y),
                &font,
            );

            y -= 5.0;
            current_layer.use_text(
                format!("Avg Latency: {:.2}ms", report.metrics.avg_latency_ms),
                10.0,
                Mm(20.0),
                Mm(y),
                &font,
            );

            y -= 5.0;
            current_layer.use_text(
                format!("P95 Latency: {:.2}ms", report.metrics.p95_latency_ms),
                10.0,
                Mm(20.0),
                Mm(y),
                &font,
            );
        }

        // Failures section
        if !report.failures.is_empty() {
            y -= 15.0;
            current_layer.use_text("Failures", 14.0, Mm(20.0), Mm(y), &font_bold);

            for failure in &report.failures {
                y -= 7.0;
                if y < 20.0 {
                    break; // Page boundary - would need to add new page
                }
                current_layer.use_text(
                    format!("• {}: {}", failure.step_name, failure.error_message),
                    9.0,
                    Mm(25.0),
                    Mm(y),
                    &font,
                );
            }
        }

        // Recommendations section
        if self.config.include_recommendations && !report.recommendations.is_empty() {
            y -= 15.0;
            current_layer.use_text("Recommendations", 14.0, Mm(20.0), Mm(y), &font_bold);

            for recommendation in &report.recommendations {
                y -= 7.0;
                if y < 20.0 {
                    break;
                }
                current_layer.use_text(
                    format!("• {}", recommendation),
                    9.0,
                    Mm(25.0),
                    Mm(y),
                    &font,
                );
            }
        }

        // Footer
        current_layer.use_text(
            format!("Generated by MockForge on {}", Utc::now().format("%Y-%m-%d %H:%M UTC")),
            8.0,
            Mm(20.0),
            Mm(10.0),
            &font,
        );

        // Save PDF
        doc.save(&mut BufWriter::new(File::create(output_path)?))
            .map_err(|e| ReportingError::Pdf(e.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_pdf_generation() {
        let config = PdfConfig::default();
        let generator = PdfReportGenerator::new(config);

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
            recommendations: vec!["Increase timeout thresholds".to_string()],
        };

        let temp_dir = tempdir().unwrap();
        let output_path = temp_dir.path().join("report.pdf");

        let result = generator.generate(&report, output_path.to_str().unwrap());
        assert!(result.is_ok());
        assert!(output_path.exists());
    }
}
