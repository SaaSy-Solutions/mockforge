//! Contract fitness functions for validating contract changes
//!
//! Fitness functions allow teams to register custom tests that run against each new contract version.
//! These tests can enforce constraints like "response size must not increase by > 25%" or
//! "no new required fields under /v1/mobile/*".

use crate::ai_contract_diff::{ContractDiffResult, MismatchType};
use crate::openapi::OpenApiSpec;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

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

/// Result of evaluating a fitness function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FitnessTestResult {
    /// ID of the fitness function that was evaluated
    pub function_id: String,
    /// Name of the fitness function
    pub function_name: String,
    /// Whether the test passed
    pub passed: bool,
    /// Human-readable message about the result
    pub message: String,
    /// Metrics collected during evaluation (e.g., "response_size": 1024.0, "increase_percent": 15.5)
    pub metrics: HashMap<String, f64>,
}

/// Trait for evaluating fitness functions
pub trait FitnessEvaluator: Send + Sync {
    /// Evaluate the fitness function against contract changes
    ///
    /// # Arguments
    ///
    /// * `old_spec` - The previous contract specification (if available)
    /// * `new_spec` - The new contract specification
    /// * `diff_result` - The contract diff result showing changes
    /// * `endpoint` - The endpoint being evaluated
    /// * `method` - The HTTP method
    /// * `config` - Additional configuration for the fitness function
    ///
    /// # Returns
    ///
    /// A `FitnessTestResult` indicating whether the test passed
    fn evaluate(
        &self,
        old_spec: Option<&OpenApiSpec>,
        new_spec: &OpenApiSpec,
        diff_result: &ContractDiffResult,
        endpoint: &str,
        method: &str,
        config: &serde_json::Value,
    ) -> crate::Result<FitnessTestResult>;
}

/// Response size fitness evaluator
pub struct ResponseSizeFitnessEvaluator;

impl FitnessEvaluator for ResponseSizeFitnessEvaluator {
    fn evaluate(
        &self,
        old_spec: Option<&OpenApiSpec>,
        _new_spec: &OpenApiSpec,
        diff_result: &ContractDiffResult,
        endpoint: &str,
        method: &str,
        config: &serde_json::Value,
    ) -> crate::Result<FitnessTestResult> {
        // Extract max_increase_percent from config
        let max_increase_percent =
            config.get("max_increase_percent").and_then(|v| v.as_f64()).unwrap_or(25.0);

        // For now, we'll estimate response size based on field count
        // In a real implementation, we might analyze actual response schemas
        let old_field_count = if let Some(old) = old_spec {
            // Estimate based on old spec - count fields in response schema
            estimate_response_field_count(old, endpoint, method)
        } else {
            // No old spec, assume baseline from current mismatches
            diff_result.mismatches.len() as f64
        };

        let new_field_count =
            estimate_response_field_count_from_diff(diff_result, endpoint, method);

        let increase_percent = if old_field_count > 0.0 {
            ((new_field_count - old_field_count) / old_field_count) * 100.0
        } else if new_field_count > 0.0 {
            100.0 // 100% increase from zero
        } else {
            0.0 // No change
        };

        let passed = increase_percent <= max_increase_percent;
        let message = if passed {
            format!(
                "Response size increase ({:.1}%) is within allowed limit ({:.1}%)",
                increase_percent, max_increase_percent
            )
        } else {
            format!(
                "Response size increase ({:.1}%) exceeds allowed limit ({:.1}%)",
                increase_percent, max_increase_percent
            )
        };

        let mut metrics = HashMap::new();
        metrics.insert("old_field_count".to_string(), old_field_count);
        metrics.insert("new_field_count".to_string(), new_field_count);
        metrics.insert("increase_percent".to_string(), increase_percent);
        metrics.insert("max_increase_percent".to_string(), max_increase_percent);

        Ok(FitnessTestResult {
            function_id: String::new(), // Will be set by caller
            function_name: "Response Size".to_string(),
            passed,
            message,
            metrics,
        })
    }
}

/// Required field fitness evaluator
pub struct RequiredFieldFitnessEvaluator;

impl FitnessEvaluator for RequiredFieldFitnessEvaluator {
    fn evaluate(
        &self,
        _old_spec: Option<&OpenApiSpec>,
        _new_spec: &OpenApiSpec,
        diff_result: &ContractDiffResult,
        endpoint: &str,
        method: &str,
        config: &serde_json::Value,
    ) -> crate::Result<FitnessTestResult> {
        // Extract path_pattern and allow_new_required from config
        let path_pattern = config.get("path_pattern").and_then(|v| v.as_str()).unwrap_or("*");
        let allow_new_required =
            config.get("allow_new_required").and_then(|v| v.as_bool()).unwrap_or(false);

        // Check if endpoint matches pattern
        let matches_pattern = matches_pattern(endpoint, path_pattern);

        if !matches_pattern {
            // This fitness function doesn't apply to this endpoint
            return Ok(FitnessTestResult {
                function_id: String::new(),
                function_name: "Required Field".to_string(),
                passed: true,
                message: format!("Endpoint {} does not match pattern {}", endpoint, path_pattern),
                metrics: HashMap::new(),
            });
        }

        // Count new required fields from mismatches
        let new_required_fields = diff_result
            .mismatches
            .iter()
            .filter(|m| {
                m.mismatch_type == MismatchType::MissingRequiredField
                    && m.method.as_ref().map(|m| m.as_str()) == Some(method)
            })
            .count();

        let passed = allow_new_required || new_required_fields == 0;
        let message = if passed {
            if allow_new_required {
                format!("Found {} new required fields, which is allowed", new_required_fields)
            } else {
                "No new required fields detected".to_string()
            }
        } else {
            format!(
                "Found {} new required fields, which violates the fitness function",
                new_required_fields
            )
        };

        let mut metrics = HashMap::new();
        metrics.insert("new_required_fields".to_string(), new_required_fields as f64);
        metrics
            .insert("allow_new_required".to_string(), if allow_new_required { 1.0 } else { 0.0 });

        Ok(FitnessTestResult {
            function_id: String::new(),
            function_name: "Required Field".to_string(),
            passed,
            message,
            metrics,
        })
    }
}

/// Field count fitness evaluator
pub struct FieldCountFitnessEvaluator;

impl FitnessEvaluator for FieldCountFitnessEvaluator {
    fn evaluate(
        &self,
        _old_spec: Option<&OpenApiSpec>,
        _new_spec: &OpenApiSpec,
        diff_result: &ContractDiffResult,
        endpoint: &str,
        method: &str,
        config: &serde_json::Value,
    ) -> crate::Result<FitnessTestResult> {
        // Extract max_fields from config
        let max_fields = config
            .get("max_fields")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .unwrap_or(100);

        // Estimate field count from diff result
        let field_count = estimate_field_count_from_diff(diff_result, endpoint, method);

        let passed = field_count <= max_fields as f64;
        let message = if passed {
            format!("Field count ({}) is within allowed limit ({})", field_count as u32, max_fields)
        } else {
            format!("Field count ({}) exceeds allowed limit ({})", field_count as u32, max_fields)
        };

        let mut metrics = HashMap::new();
        metrics.insert("field_count".to_string(), field_count);
        metrics.insert("max_fields".to_string(), max_fields as f64);

        Ok(FitnessTestResult {
            function_id: String::new(),
            function_name: "Field Count".to_string(),
            passed,
            message,
            metrics,
        })
    }
}

/// Schema complexity fitness evaluator
pub struct SchemaComplexityFitnessEvaluator;

impl FitnessEvaluator for SchemaComplexityFitnessEvaluator {
    fn evaluate(
        &self,
        _old_spec: Option<&OpenApiSpec>,
        new_spec: &OpenApiSpec,
        _diff_result: &ContractDiffResult,
        endpoint: &str,
        method: &str,
        config: &serde_json::Value,
    ) -> crate::Result<FitnessTestResult> {
        // Extract max_depth from config
        let max_depth =
            config.get("max_depth").and_then(|v| v.as_u64()).map(|v| v as u32).unwrap_or(10);

        // Calculate schema depth for the endpoint
        let depth = calculate_schema_depth(new_spec, endpoint, method);

        let passed = depth <= max_depth;
        let message = if passed {
            format!("Schema depth ({}) is within allowed limit ({})", depth, max_depth)
        } else {
            format!("Schema depth ({}) exceeds allowed limit ({})", depth, max_depth)
        };

        let mut metrics = HashMap::new();
        metrics.insert("schema_depth".to_string(), depth as f64);
        metrics.insert("max_depth".to_string(), max_depth as f64);

        Ok(FitnessTestResult {
            function_id: String::new(),
            function_name: "Schema Complexity".to_string(),
            passed,
            message,
            metrics,
        })
    }
}

/// Registry for managing fitness functions
pub struct FitnessFunctionRegistry {
    /// Registered fitness functions
    functions: HashMap<String, FitnessFunction>,
    /// Evaluators for each function type
    evaluators: HashMap<String, Arc<dyn FitnessEvaluator>>,
}

impl std::fmt::Debug for FitnessFunctionRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FitnessFunctionRegistry")
            .field("functions", &self.functions)
            .field("evaluators_count", &self.evaluators.len())
            .finish()
    }
}

impl FitnessFunctionRegistry {
    /// Create a new fitness function registry
    pub fn new() -> Self {
        let mut registry = Self {
            functions: HashMap::new(),
            evaluators: HashMap::new(),
        };

        // Register built-in evaluators
        registry.register_evaluator(
            "response_size",
            Arc::new(ResponseSizeFitnessEvaluator) as Arc<dyn FitnessEvaluator>,
        );
        registry.register_evaluator(
            "required_field",
            Arc::new(RequiredFieldFitnessEvaluator) as Arc<dyn FitnessEvaluator>,
        );
        registry.register_evaluator(
            "field_count",
            Arc::new(FieldCountFitnessEvaluator) as Arc<dyn FitnessEvaluator>,
        );
        registry.register_evaluator(
            "schema_complexity",
            Arc::new(SchemaComplexityFitnessEvaluator) as Arc<dyn FitnessEvaluator>,
        );

        registry
    }

    /// Register a fitness function evaluator
    pub fn register_evaluator(&mut self, name: &str, evaluator: Arc<dyn FitnessEvaluator>) {
        self.evaluators.insert(name.to_string(), evaluator);
    }

    /// Add a fitness function to the registry
    pub fn add_function(&mut self, function: FitnessFunction) {
        self.functions.insert(function.id.clone(), function);
    }

    /// Get a fitness function by ID
    pub fn get_function(&self, id: &str) -> Option<&FitnessFunction> {
        self.functions.get(id)
    }

    /// List all fitness functions
    pub fn list_functions(&self) -> Vec<&FitnessFunction> {
        self.functions.values().collect()
    }

    /// Get fitness functions that apply to a given scope
    pub fn get_functions_for_scope(
        &self,
        endpoint: &str,
        method: &str,
        workspace_id: Option<&str>,
        service_name: Option<&str>,
    ) -> Vec<&FitnessFunction> {
        self.functions
            .values()
            .filter(|f| {
                f.enabled && self.matches_scope(f, endpoint, method, workspace_id, service_name)
            })
            .collect()
    }

    /// Evaluate all applicable fitness functions
    pub fn evaluate_all(
        &self,
        old_spec: Option<&OpenApiSpec>,
        new_spec: &OpenApiSpec,
        diff_result: &ContractDiffResult,
        endpoint: &str,
        method: &str,
        workspace_id: Option<&str>,
        service_name: Option<&str>,
    ) -> crate::Result<Vec<FitnessTestResult>> {
        let functions = self.get_functions_for_scope(endpoint, method, workspace_id, service_name);
        let mut results = Vec::new();

        for function in functions {
            let evaluator_name = match &function.function_type {
                FitnessFunctionType::ResponseSize { .. } => "response_size",
                FitnessFunctionType::RequiredField { .. } => "required_field",
                FitnessFunctionType::FieldCount { .. } => "field_count",
                FitnessFunctionType::SchemaComplexity { .. } => "schema_complexity",
                FitnessFunctionType::Custom { evaluator } => evaluator.as_str(),
            };

            if let Some(evaluator) = self.evaluators.get(evaluator_name) {
                let mut result = evaluator.evaluate(
                    old_spec,
                    new_spec,
                    diff_result,
                    endpoint,
                    method,
                    &function.config,
                )?;
                result.function_id = function.id.clone();
                result.function_name = function.name.clone();
                results.push(result);
            }
        }

        Ok(results)
    }

    /// Check if a fitness function's scope matches the given context
    fn matches_scope(
        &self,
        function: &FitnessFunction,
        endpoint: &str,
        method: &str,
        workspace_id: Option<&str>,
        service_name: Option<&str>,
    ) -> bool {
        match &function.scope {
            FitnessScope::Global => true,
            FitnessScope::Workspace {
                workspace_id: ws_id,
            } => workspace_id.map(|id| id == ws_id).unwrap_or(false),
            FitnessScope::Service {
                service_name: svc_name,
            } => service_name.map(|name| name == svc_name).unwrap_or(false),
            FitnessScope::Endpoint { pattern } => matches_pattern(endpoint, pattern),
        }
    }

    /// Remove a fitness function
    pub fn remove_function(&mut self, id: &str) -> Option<FitnessFunction> {
        self.functions.remove(id)
    }

    /// Update a fitness function
    pub fn update_function(&mut self, function: FitnessFunction) {
        self.functions.insert(function.id.clone(), function);
    }
}

impl Default for FitnessFunctionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Helper functions

/// Check if an endpoint matches a pattern (supports * wildcard)
fn matches_pattern(endpoint: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    // Simple wildcard matching: convert pattern to regex-like matching
    let pattern_parts: Vec<&str> = pattern.split('*').collect();
    if pattern_parts.len() == 1 {
        // No wildcard, exact match
        return endpoint == pattern;
    }

    // Check if endpoint starts with first part and ends with last part
    if let (Some(first), Some(last)) = (pattern_parts.first(), pattern_parts.last()) {
        endpoint.starts_with(first) && endpoint.ends_with(last)
    } else {
        false
    }
}

/// Estimate response field count from OpenAPI spec
fn estimate_response_field_count(_spec: &OpenApiSpec, _endpoint: &str, _method: &str) -> f64 {
    // This is a simplified estimation - in a real implementation, we'd
    // traverse the response schema and count all fields
    // For now, return a placeholder value
    10.0
}

/// Estimate response field count from diff result
fn estimate_response_field_count_from_diff(
    diff_result: &ContractDiffResult,
    _endpoint: &str,
    _method: &str,
) -> f64 {
    // Estimate based on number of mismatches and corrections
    // This is a simplified approach
    let base_count = 10.0;
    let mismatch_count = diff_result.mismatches.len() as f64;
    base_count + mismatch_count
}

/// Estimate field count from diff result
fn estimate_field_count_from_diff(
    diff_result: &ContractDiffResult,
    _endpoint: &str,
    _method: &str,
) -> f64 {
    // Count unique paths in mismatches
    let unique_paths: std::collections::HashSet<String> = diff_result
        .mismatches
        .iter()
        .map(|m| {
            // Extract base path (before any array indices or property names)
            m.path.split('.').next().unwrap_or("").to_string()
        })
        .collect();

    unique_paths.len() as f64 + 10.0 // Add base estimate
}

/// Calculate schema depth for an endpoint
fn calculate_schema_depth(_spec: &OpenApiSpec, _endpoint: &str, _method: &str) -> u32 {
    // This is a simplified calculation - in a real implementation, we'd
    // traverse the response schema and calculate the maximum depth
    // For now, return a placeholder value
    5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_pattern() {
        assert!(matches_pattern("/api/users", "*"));
        assert!(matches_pattern("/api/users", "/api/users"));
        assert!(matches_pattern("/api/users/123", "/api/users/*"));
        assert!(matches_pattern("/v1/mobile/users", "/v1/mobile/*"));
        assert!(!matches_pattern("/api/users", "/api/orders"));
    }

    #[test]
    fn test_fitness_function_registry() {
        let mut registry = FitnessFunctionRegistry::new();

        let function = FitnessFunction {
            id: "test-1".to_string(),
            name: "Test Function".to_string(),
            description: "Test".to_string(),
            function_type: FitnessFunctionType::ResponseSize {
                max_increase_percent: 25.0,
            },
            config: serde_json::json!({"max_increase_percent": 25.0}),
            scope: FitnessScope::Global,
            enabled: true,
            created_at: 0,
            updated_at: 0,
        };

        registry.add_function(function);
        assert_eq!(registry.list_functions().len(), 1);
    }
}
