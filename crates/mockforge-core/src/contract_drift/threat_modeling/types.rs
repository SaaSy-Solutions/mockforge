//! Types for contract threat modeling
//!
//! This module defines data structures for API security threat assessments
//! and remediation suggestions.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Threat assessment for a contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatAssessment {
    /// Workspace ID
    pub workspace_id: Option<String>,
    /// Service ID
    pub service_id: Option<String>,
    /// Service name
    pub service_name: Option<String>,
    /// Endpoint (optional, None for service-level)
    pub endpoint: Option<String>,
    /// Method (optional, None for service-level)
    pub method: Option<String>,
    /// Aggregation level
    pub aggregation_level: AggregationLevel,
    /// Overall threat level
    pub threat_level: ThreatLevel,
    /// Threat score (0.0-1.0)
    pub threat_score: f64,
    /// Threat categories detected
    pub threat_categories: Vec<ThreatCategory>,
    /// Individual findings
    pub findings: Vec<ThreatFinding>,
    /// Remediation suggestions
    pub remediation_suggestions: Vec<RemediationSuggestion>,
    /// When assessment was performed
    pub assessed_at: DateTime<Utc>,
}

/// Aggregation level for threat assessment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AggregationLevel {
    /// Workspace-level assessment
    Workspace,
    /// Service-level assessment
    Service,
    /// Endpoint-level assessment
    Endpoint,
}

/// Threat level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum ThreatLevel {
    /// Low threat
    Low,
    /// Medium threat
    Medium,
    /// High threat
    High,
    /// Critical threat
    Critical,
}

/// Threat category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ThreatCategory {
    /// PII exposure risk
    PiiExposure,
    /// Denial of Service risk
    DoSRisk,
    /// Error message leakage
    ErrorLeakage,
    /// Schema inconsistency
    SchemaInconsistency,
    /// Unbounded arrays
    UnboundedArrays,
    /// Missing rate limits
    MissingRateLimits,
    /// Stack trace leakage
    StackTraceLeakage,
    /// Sensitive data exposure
    SensitiveDataExposure,
    /// Insecure schema design
    InsecureSchemaDesign,
    /// Missing validation
    MissingValidation,
    /// Excessive optional fields
    ExcessiveOptionalFields,
}

impl std::fmt::Display for ThreatCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThreatCategory::PiiExposure => write!(f, "pii_exposure"),
            ThreatCategory::DoSRisk => write!(f, "dos_risk"),
            ThreatCategory::ErrorLeakage => write!(f, "error_leakage"),
            ThreatCategory::SchemaInconsistency => write!(f, "schema_inconsistency"),
            ThreatCategory::UnboundedArrays => write!(f, "unbounded_arrays"),
            ThreatCategory::MissingRateLimits => write!(f, "missing_rate_limits"),
            ThreatCategory::StackTraceLeakage => write!(f, "stack_trace_leakage"),
            ThreatCategory::SensitiveDataExposure => write!(f, "sensitive_data_exposure"),
            ThreatCategory::InsecureSchemaDesign => write!(f, "insecure_schema_design"),
            ThreatCategory::MissingValidation => write!(f, "missing_validation"),
            ThreatCategory::ExcessiveOptionalFields => write!(f, "excessive_optional_fields"),
        }
    }
}

/// Individual threat finding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatFinding {
    /// Finding type
    pub finding_type: ThreatCategory,
    /// Severity
    pub severity: ThreatLevel,
    /// Description
    pub description: String,
    /// Field path or location
    pub field_path: Option<String>,
    /// Additional context
    pub context: HashMap<String, serde_json::Value>,
    /// Confidence in this finding (0.0-1.0)
    pub confidence: f64,
}

/// Remediation suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationSuggestion {
    /// Finding ID this remediation addresses
    pub finding_id: String,
    /// Suggestion text
    pub suggestion: String,
    /// Code example (if applicable)
    pub code_example: Option<String>,
    /// Confidence in this suggestion (0.0-1.0)
    pub confidence: f64,
    /// Whether this is AI-generated
    pub ai_generated: bool,
    /// Priority (1 = highest)
    pub priority: u32,
}

/// Configuration for threat modeling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatModelingConfig {
    /// Whether threat modeling is enabled
    pub enabled: bool,
    /// Whether to generate AI remediation suggestions
    pub generate_remediations: bool,
    /// PII detection patterns
    pub pii_patterns: Vec<String>,
    /// Maximum optional fields threshold
    pub max_optional_fields_threshold: usize,
    /// Unbounded array detection enabled
    pub detect_unbounded_arrays: bool,
    /// Error leakage detection enabled
    pub detect_error_leakage: bool,
}

impl Default for ThreatModelingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            generate_remediations: true,
            pii_patterns: vec![
                "email".to_string(),
                "ssn".to_string(),
                "credit_card".to_string(),
                "password".to_string(),
                "token".to_string(),
                "secret".to_string(),
                "api_key".to_string(),
            ],
            max_optional_fields_threshold: 10,
            detect_unbounded_arrays: true,
            detect_error_leakage: true,
        }
    }
}

