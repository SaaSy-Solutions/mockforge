//! Core types for AI-powered contract diff analysis
//!
//! This module defines the data structures used throughout the contract diff system,
//! including mismatch results, recommendations, correction proposals, and confidence scores.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of contract diff analysis between a request and a contract specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractDiffResult {
    /// Whether the request matches the contract
    pub matches: bool,

    /// Overall confidence score for the analysis (0.0-1.0)
    pub confidence: f64,

    /// List of detected mismatches
    pub mismatches: Vec<Mismatch>,

    /// AI-generated recommendations for fixing mismatches
    pub recommendations: Vec<Recommendation>,

    /// Correction proposals (patch operations)
    pub corrections: Vec<CorrectionProposal>,

    /// Metadata about the analysis
    pub metadata: DiffMetadata,
}

/// A detected mismatch between request and contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mismatch {
    /// Type of mismatch detected
    pub mismatch_type: MismatchType,

    /// JSON path to the field/endpoint where mismatch occurred
    pub path: String,

    /// HTTP method (if applicable)
    pub method: Option<String>,

    /// Expected value or schema constraint
    pub expected: Option<String>,

    /// Actual value found in request
    pub actual: Option<String>,

    /// Human-readable description of the mismatch
    pub description: String,

    /// Severity of the mismatch
    pub severity: MismatchSeverity,

    /// Confidence score for this specific mismatch (0.0-1.0)
    pub confidence: f64,

    /// Additional context about the mismatch
    #[serde(default)]
    pub context: HashMap<String, serde_json::Value>,
}

/// Types of mismatches that can be detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MismatchType {
    /// Missing required field in request
    MissingRequiredField,

    /// Field type mismatch (e.g., expected string, got number)
    TypeMismatch,

    /// Unexpected field present in request
    UnexpectedField,

    /// Value doesn't match expected format (e.g., email, UUID)
    FormatMismatch,

    /// Value outside allowed range or enum values
    ConstraintViolation,

    /// Endpoint not found in contract
    EndpointNotFound,

    /// HTTP method not allowed for endpoint
    MethodNotAllowed,

    /// Request body structure doesn't match schema
    SchemaMismatch,

    /// Header missing or incorrect
    HeaderMismatch,

    /// Query parameter mismatch
    QueryParamMismatch,
}

/// Severity levels for mismatches
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum MismatchSeverity {
    /// Critical - will cause request to fail
    Critical,

    /// High - likely to cause issues
    High,

    /// Medium - may cause issues
    Medium,

    /// Low - minor issue, may be acceptable
    Low,

    /// Info - informational only
    Info,
}

impl Default for MismatchSeverity {
    fn default() -> Self {
        Self::High
    }
}

/// AI-generated recommendation for fixing a mismatch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recommendation {
    /// Unique identifier for this recommendation
    pub id: String,

    /// Mismatch this recommendation addresses
    pub mismatch_id: String,

    /// Human-readable recommendation text
    pub recommendation: String,

    /// Suggested fix or action
    pub suggested_fix: Option<String>,

    /// Confidence score for this recommendation (0.0-1.0)
    pub confidence: f64,

    /// Reasoning behind the recommendation
    pub reasoning: Option<String>,

    /// Example of the fix (if applicable)
    pub example: Option<serde_json::Value>,
}

/// Correction proposal with patch operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionProposal {
    /// Unique identifier for this correction
    pub id: String,

    /// JSON Patch path to apply the correction
    pub patch_path: String,

    /// JSON Patch operation (add, remove, replace, etc.)
    pub operation: PatchOperation,

    /// Value to apply (for add/replace operations)
    pub value: Option<serde_json::Value>,

    /// Value to remove (for remove/replace operations)
    pub from: Option<String>,

    /// Confidence score for this correction (0.0-1.0)
    pub confidence: f64,

    /// Human-readable description of the correction
    pub description: String,

    /// Reasoning for this correction
    pub reasoning: Option<String>,

    /// Affected endpoints
    pub affected_endpoints: Vec<String>,

    /// Before/after comparison
    pub before: Option<serde_json::Value>,
    /// State after the correction (for comparison)
    pub after: Option<serde_json::Value>,
}

/// JSON Patch operation types (RFC 6902)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PatchOperation {
    /// Add a new field
    Add,

    /// Remove a field
    Remove,

    /// Replace an existing field
    Replace,

    /// Move a field
    Move,

    /// Copy a field
    Copy,

    /// Test a value (for validation)
    Test,
}

/// Metadata about the diff analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffMetadata {
    /// Timestamp when analysis was performed
    pub analyzed_at: DateTime<Utc>,

    /// Source of the request (browser_extension, proxy, manual_upload, api)
    pub request_source: String,

    /// Contract specification version
    pub contract_version: Option<String>,

    /// Contract format (openapi-3.0, openapi-3.1, json-schema, etc.)
    pub contract_format: String,

    /// Endpoint path analyzed
    pub endpoint_path: String,

    /// HTTP method analyzed
    pub http_method: String,

    /// Number of requests analyzed (for batch analysis)
    pub request_count: usize,

    /// LLM provider used for analysis
    pub llm_provider: Option<String>,

    /// LLM model used for analysis
    pub llm_model: Option<String>,
}

/// Configuration for contract diff analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractDiffConfig {
    /// Whether contract diff is enabled
    pub enabled: bool,

    /// LLM provider to use (openai, anthropic, ollama, openai-compatible)
    pub llm_provider: String,

    /// LLM model to use
    pub llm_model: String,

    /// API key for LLM provider
    pub api_key: Option<String>,

    /// Confidence threshold - only show suggestions above this (0.0-1.0)
    pub confidence_threshold: f64,

    /// Whether to generate correction proposals
    pub generate_corrections: bool,

    /// Whether to use AI for recommendations
    pub use_ai_recommendations: bool,

    /// Maximum number of recommendations to generate
    pub max_recommendations: usize,

    /// Whether to include examples in recommendations
    pub include_examples: bool,
}

impl Default for ContractDiffConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            llm_provider: "openai".to_string(),
            llm_model: "gpt-4".to_string(),
            api_key: None,
            confidence_threshold: 0.5,
            generate_corrections: true,
            use_ai_recommendations: true,
            max_recommendations: 10,
            include_examples: true,
        }
    }
}

/// Captured request for contract analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapturedRequest {
    /// HTTP method
    pub method: String,

    /// Request path
    pub path: String,

    /// Query parameters
    #[serde(default)]
    pub query_params: HashMap<String, String>,

    /// Request headers
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Request body
    pub body: Option<serde_json::Value>,

    /// Response status code (if available)
    pub status_code: Option<u16>,

    /// Response body (if available)
    pub response_body: Option<serde_json::Value>,

    /// Timestamp when request was captured
    pub timestamp: DateTime<Utc>,

    /// Source of the capture (browser_extension, proxy, manual_upload, api)
    pub source: String,

    /// User agent (if available)
    pub user_agent: Option<String>,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl CapturedRequest {
    /// Create a new captured request
    pub fn new(
        method: impl Into<String>,
        path: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: None,
            status_code: None,
            response_body: None,
            timestamp: Utc::now(),
            source: source.into(),
            user_agent: None,
            metadata: HashMap::new(),
        }
    }

    /// Add query parameters
    pub fn with_query_params(mut self, params: HashMap<String, String>) -> Self {
        self.query_params = params;
        self
    }

    /// Add headers
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    /// Set request body
    pub fn with_body(mut self, body: serde_json::Value) -> Self {
        self.body = Some(body);
        self
    }

    /// Set response
    pub fn with_response(mut self, status_code: u16, body: Option<serde_json::Value>) -> Self {
        self.status_code = Some(status_code);
        self.response_body = body;
        self
    }
}

/// Confidence level categories for visual indicators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfidenceLevel {
    /// High confidence (0.8-1.0) - Green, auto-suggested
    High,

    /// Medium confidence (0.5-0.8) - Yellow, review recommended
    Medium,

    /// Low confidence (0.0-0.5) - Red, manual review required
    Low,
}

impl ConfidenceLevel {
    /// Determine confidence level from score
    pub fn from_score(score: f64) -> Self {
        if score >= 0.8 {
            Self::High
        } else if score >= 0.5 {
            Self::Medium
        } else {
            Self::Low
        }
    }

    /// Get color code for UI display
    pub fn color(&self) -> &'static str {
        match self {
            Self::High => "green",
            Self::Medium => "yellow",
            Self::Low => "red",
        }
    }

    /// Get label for UI display
    pub fn label(&self) -> &'static str {
        match self {
            Self::High => "High",
            Self::Medium => "Medium",
            Self::Low => "Low",
        }
    }
}
