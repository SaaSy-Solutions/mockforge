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
pub use csv_export::{CsvExporter, CsvExportConfig, CsvBatchExporter};
pub use dashboard_layouts::{
    DashboardLayout, DashboardLayoutManager, DashboardLayoutBuilder,
    DashboardTemplates, Widget, WidgetType, DataSource, GridConfig,
};
pub use email::{EmailNotifier, EmailConfig, EmailReport};
pub use flamegraph::{FlamegraphGenerator, TraceData, TraceSpan, FlamegraphStats};
pub use pdf::{PdfReportGenerator, PdfConfig};
pub use trend_analysis::{TrendAnalyzer, TrendReport, TrendDirection};

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
