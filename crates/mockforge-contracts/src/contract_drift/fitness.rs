//! Contract fitness function types
//!
//! This module defines the types for fitness functions that validate contract changes.
//! The full evaluator logic remains in `mockforge-core` as it depends on `OpenApiSpec`.

use serde::{Deserialize, Serialize};

/// A fitness function that evaluates contract changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessFunction {
    /// Unique identifier for this fitness function
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description of what this fitness function checks
    pub description: String,
    /// Type of fitness function
    pub function_type: FitnessFunctionType,
    /// Additional configuration (JSON)
    pub config: serde_json::Value,
    /// Scope where this function applies
    pub scope: FitnessScope,
    /// Whether this function is enabled
    pub enabled: bool,
    /// Timestamp when this function was created
    #[serde(default)]
    pub created_at: i64,
    /// Timestamp when this function was last updated
    #[serde(default)]
    pub updated_at: i64,
}

/// Scope where a fitness function applies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FitnessScope {
    /// Applies globally to all endpoints
    Global,
    /// Applies to a specific workspace
    Workspace {
        /// The workspace ID
        workspace_id: String,
    },
    /// Applies to a specific service (by OpenAPI tag or service name)
    Service {
        /// The service name or OpenAPI tag
        service_name: String,
    },
    /// Applies to a specific endpoint pattern (e.g., "/v1/mobile/*")
    Endpoint {
        /// The endpoint pattern (supports * wildcard)
        pattern: String,
    },
}

/// Type of fitness function
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FitnessFunctionType {
    /// Response size must not increase by more than a percentage
    ResponseSize {
        /// Maximum allowed increase percentage (e.g., 25.0 for 25%)
        max_increase_percent: f64,
    },
    /// No new required fields under a path pattern
    RequiredField {
        /// Path pattern to check (e.g., "/v1/mobile/*")
        path_pattern: String,
        /// Whether new required fields are allowed
        allow_new_required: bool,
    },
    /// Field count must not exceed a threshold
    FieldCount {
        /// Maximum number of fields allowed
        max_fields: u32,
    },
    /// Schema complexity (depth) must not exceed a threshold
    SchemaComplexity {
        /// Maximum schema depth allowed
        max_depth: u32,
    },
    /// Custom fitness function (for future plugin support)
    Custom {
        /// Identifier for the custom evaluator
        evaluator: String,
    },
}

/// Result of evaluating a fitness function.
///
/// Re-exported from `mockforge-foundation::contract_drift_types` so consumers
/// (core and contracts) share the same type.
pub use mockforge_foundation::contract_drift_types::FitnessTestResult;
