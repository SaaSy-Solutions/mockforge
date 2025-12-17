//! MockForge Reporting
//!
//! Comprehensive reporting capabilities including PDF generation,
//! email notifications, trend analysis, comparison reports, CSV export,
//! flamegraph visualization, and custom dashboard layouts.

pub mod comparison;
pub mod csv_export;
pub mod dashboard_layouts;
pub mod email;
pub mod flamegraph;
pub mod pdf;
pub mod trend_analysis;

pub use comparison::{ComparisonReport, ComparisonReportGenerator};
pub use csv_export::{CsvBatchExporter, CsvExportConfig, CsvExporter};
pub use dashboard_layouts::{
    DashboardLayout, DashboardLayoutBuilder, DashboardLayoutManager, DashboardTemplates,
    DataSource, GridConfig, Widget, WidgetType,
};
pub use email::{EmailConfig, EmailNotifier, EmailReport};
pub use flamegraph::{FlamegraphGenerator, FlamegraphStats, TraceData, TraceSpan};
pub use pdf::{PdfConfig, PdfReportGenerator};
pub use trend_analysis::{TrendAnalyzer, TrendDirection, TrendReport};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ReportingError {
    #[error("PDF generation error: {0}")]
    Pdf(String),

    #[error("Email sending error: {0}")]
    Email(String),

    #[error("Analysis error: {0}")]
    Analysis(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, ReportingError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reporting_error_pdf() {
        let error = ReportingError::Pdf("font not found".to_string());
        assert_eq!(error.to_string(), "PDF generation error: font not found");
    }

    #[test]
    fn test_reporting_error_email() {
        let error = ReportingError::Email("SMTP connection failed".to_string());
        assert_eq!(error.to_string(), "Email sending error: SMTP connection failed");
    }

    #[test]
    fn test_reporting_error_analysis() {
        let error = ReportingError::Analysis("insufficient data".to_string());
        assert_eq!(error.to_string(), "Analysis error: insufficient data");
    }

    #[test]
    fn test_reporting_error_from_io() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let error: ReportingError = io_error.into();
        assert!(matches!(error, ReportingError::Io(_)));
        assert!(error.to_string().contains("IO error"));
    }

    #[test]
    fn test_reporting_error_from_serde_json() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let error: ReportingError = json_error.into();
        assert!(matches!(error, ReportingError::Serialization(_)));
        assert!(error.to_string().contains("Serialization error"));
    }

    #[test]
    fn test_reporting_error_debug() {
        let error = ReportingError::Pdf("test".to_string());
        let debug = format!("{:?}", error);
        assert!(debug.contains("Pdf"));
    }

    #[test]
    fn test_result_ok() {
        let result: Result<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_err() {
        let result: Result<i32> = Err(ReportingError::Pdf("test".to_string()));
        assert!(result.is_err());
    }
}
