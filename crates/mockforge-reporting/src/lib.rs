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
