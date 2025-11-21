//! Contract Threat Modeling
//!
//! This module provides security posture analysis for API contracts,
//! detecting threats like PII exposure, DoS risks, error leakage,
//! and generating AI-powered remediation suggestions.

pub mod dos_analyzer;
pub mod error_analyzer;
pub mod pii_detector;
pub mod remediation_generator;
pub mod schema_analyzer;
pub mod threat_analyzer;
pub mod types;

pub use dos_analyzer::DosAnalyzer;
pub use error_analyzer::ErrorAnalyzer;
pub use pii_detector::PiiDetector;
pub use remediation_generator::RemediationGenerator;
pub use schema_analyzer::SchemaAnalyzer;
pub use threat_analyzer::ThreatAnalyzer;
pub use types::*;

