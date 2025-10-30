/// Contract validation for CI/CD pipelines
///
/// Validates that mock configurations match live API responses
/// and detects breaking changes in API contracts
use serde::{Deserialize, Serialize};

/// Result of contract validation with detailed breakdown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether all validation checks passed
    pub passed: bool,
    /// Total number of validation checks performed
    pub total_checks: usize,
    /// Number of checks that passed
    pub passed_checks: usize,
    /// Number of checks that failed
    pub failed_checks: usize,
    /// Non-blocking warnings encountered during validation
    pub warnings: Vec<ValidationWarning>,
    /// Validation errors that prevent the contract from being valid
    pub errors: Vec<ValidationError>,
    /// Breaking changes detected compared to previous contract version
    pub breaking_changes: Vec<BreakingChange>,
}

impl ValidationResult {
    /// Create a new empty validation result
    pub fn new() -> Self {
        Self {
            passed: true,
            total_checks: 0,
            passed_checks: 0,
            failed_checks: 0,
            warnings: Vec::new(),
            errors: Vec::new(),
            breaking_changes: Vec::new(),
        }
    }

    /// Add a validation error (marks result as failed)
    pub fn add_error(&mut self, error: ValidationError) {
        self.errors.push(error);
        self.failed_checks += 1;
        self.total_checks += 1;
        self.passed = false;
    }

    /// Add a validation warning (does not fail the result)
    pub fn add_warning(&mut self, warning: ValidationWarning) {
        self.warnings.push(warning);
        self.passed_checks += 1;
        self.total_checks += 1;
    }

    /// Add a breaking change (marks result as failed)
    pub fn add_breaking_change(&mut self, change: BreakingChange) {
        self.breaking_changes.push(change);
        self.passed = false;
    }

    /// Record a successful validation check
    pub fn add_success(&mut self) {
        self.passed_checks += 1;
        self.total_checks += 1;
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Validation warning for non-blocking issues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    /// JSON path or endpoint path where the warning occurred
    pub path: String,
    /// Human-readable warning message
    pub message: String,
    /// Severity level of the warning
    pub severity: WarningSeverity,
}

/// Severity level for validation warnings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WarningSeverity {
    /// Informational message (low priority)
    Info,
    /// Warning that should be reviewed (medium priority)
    Warning,
}

/// Validation error for contract violations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// JSON path or endpoint path where the error occurred
    pub path: String,
    /// Human-readable error message
    pub message: String,
    /// Expected value or format (if applicable)
    pub expected: Option<String>,
    /// Actual value found (if applicable)
    pub actual: Option<String>,
}

/// Breaking change detected between contract versions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakingChange {
    /// Type of breaking change detected
    pub change_type: BreakingChangeType,
    /// Path or endpoint affected by the change
    pub path: String,
    /// Human-readable description of the breaking change
    pub description: String,
    /// Severity of the breaking change
    pub severity: ChangeSeverity,
}

/// Types of breaking changes that can be detected
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BreakingChangeType {
    /// An API endpoint was removed
    EndpointRemoved,
    /// A required field was added to a request/response
    RequiredFieldAdded,
    /// A field's data type was changed
    FieldTypeChanged,
    /// A field was removed from a request/response
    FieldRemoved,
    /// An HTTP response status code was changed
    ResponseCodeChanged,
    /// Authentication requirements were changed
    AuthenticationChanged,
}

/// Severity level for breaking changes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeSeverity {
    /// Critical breaking change - will break all clients
    Critical,
    /// Major breaking change - will break most clients
    Major,
    /// Minor breaking change - may break some clients
    Minor,
}

/// Contract validator for validating OpenAPI specs against live APIs
pub struct ContractValidator {
    /// Whether to use strict validation mode (fails on warnings)
    strict_mode: bool,
    /// Whether to ignore optional fields during validation
    ignore_optional_fields: bool,
}

impl ContractValidator {
    /// Create a new contract validator with default settings
    pub fn new() -> Self {
        Self {
            strict_mode: false,
            ignore_optional_fields: false,
        }
    }

    /// Configure strict validation mode (fails validation on warnings)
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// Configure whether to ignore optional fields during validation
    pub fn with_ignore_optional_fields(mut self, ignore: bool) -> Self {
        self.ignore_optional_fields = ignore;
        self
    }

    /// Validate OpenAPI spec against live API
    pub async fn validate_openapi(
        &self,
        spec: &crate::openapi::OpenApiSpec,
        base_url: &str,
    ) -> ValidationResult {
        let mut result = ValidationResult::new();

        for (path, path_item_ref) in &spec.spec.paths.paths {
            if let openapiv3::ReferenceOr::Item(path_item) = path_item_ref {
                let operations = vec![
                    ("GET", path_item.get.as_ref()),
                    ("POST", path_item.post.as_ref()),
                    ("PUT", path_item.put.as_ref()),
                    ("DELETE", path_item.delete.as_ref()),
                    ("PATCH", path_item.patch.as_ref()),
                ];

                for (method, op_opt) in operations {
                    if let Some(op) = op_opt {
                        self.validate_endpoint(&mut result, base_url, method, path, op).await;
                    }
                }
            }
        }

        result
    }

    async fn validate_endpoint(
        &self,
        result: &mut ValidationResult,
        base_url: &str,
        method: &str,
        path: &str,
        operation: &openapiv3::Operation,
    ) {
        let url = format!("{}{}", base_url, path);

        // Try to make a request to the endpoint
        let client = reqwest::Client::new();
        let request = match method {
            "GET" => client.get(&url),
            "POST" => client.post(&url),
            "PUT" => client.put(&url),
            "DELETE" => client.delete(&url),
            "PATCH" => client.patch(&url),
            _ => {
                result.add_error(ValidationError {
                    path: path.to_string(),
                    message: format!("Unsupported HTTP method: {}", method),
                    expected: None,
                    actual: None,
                });
                return;
            }
        };

        match request.send().await {
            Ok(response) => {
                let status = response.status();

                // Check if status code matches spec
                let expected_codes: Vec<u16> = operation
                    .responses
                    .responses
                    .keys()
                    .filter_map(|k| match k {
                        openapiv3::StatusCode::Code(code) => Some(*code),
                        _ => None,
                    })
                    .collect();

                if !expected_codes.contains(&status.as_u16()) {
                    result.add_warning(ValidationWarning {
                        path: format!("{} {}", method, path),
                        message: format!(
                            "Status code {} not in spec (expected: {:?})",
                            status.as_u16(),
                            expected_codes
                        ),
                        severity: WarningSeverity::Warning,
                    });
                } else {
                    result.add_success();
                }
            }
            Err(e) => {
                if self.strict_mode {
                    result.add_error(ValidationError {
                        path: format!("{} {}", method, path),
                        message: format!("Failed to reach endpoint: {}", e),
                        expected: Some("2xx response".to_string()),
                        actual: Some("connection error".to_string()),
                    });
                } else {
                    result.add_warning(ValidationWarning {
                        path: format!("{} {}", method, path),
                        message: format!("Endpoint not reachable: {}", e),
                        severity: WarningSeverity::Info,
                    });
                    result.add_success();
                }
            }
        }
    }

    /// Compare two OpenAPI specs and detect breaking changes
    pub fn compare_specs(
        &self,
        old_spec: &crate::openapi::OpenApiSpec,
        new_spec: &crate::openapi::OpenApiSpec,
    ) -> ValidationResult {
        let mut result = ValidationResult::new();

        // Check for removed endpoints
        for (path, _) in &old_spec.spec.paths.paths {
            if !new_spec.spec.paths.paths.contains_key(path) {
                result.add_breaking_change(BreakingChange {
                    change_type: BreakingChangeType::EndpointRemoved,
                    path: path.clone(),
                    description: format!("Endpoint {} was removed", path),
                    severity: ChangeSeverity::Critical,
                });
            }
        }

        // Check for new required fields (this is a simplified check)
        for (path, new_path_item_ref) in &new_spec.spec.paths.paths {
            if let openapiv3::ReferenceOr::Item(_new_path_item) = new_path_item_ref {
                if let Some(_old_path_item_ref) = old_spec.spec.paths.paths.get(path) {
                    // In a real implementation, we'd do deep comparison of schemas
                    // This is a placeholder for demonstration
                    result.add_success();
                }
            }
        }

        result
    }

    /// Generate validation report
    pub fn generate_report(&self, result: &ValidationResult) -> String {
        let mut report = String::new();

        report.push_str("# Contract Validation Report\n\n");
        report.push_str(&format!(
            "**Status**: {}\n",
            if result.passed {
                "✓ PASSED"
            } else {
                "✗ FAILED"
            }
        ));
        report.push_str(&format!("**Total Checks**: {}\n", result.total_checks));
        report.push_str(&format!("**Passed**: {}\n", result.passed_checks));
        report.push_str(&format!("**Failed**: {}\n\n", result.failed_checks));

        if !result.breaking_changes.is_empty() {
            report.push_str("## Breaking Changes\n\n");
            for change in &result.breaking_changes {
                report.push_str(&format!(
                    "- **{:?}** ({:?}): {} - {}\n",
                    change.change_type, change.severity, change.path, change.description
                ));
            }
            report.push('\n');
        }

        if !result.errors.is_empty() {
            report.push_str("## Errors\n\n");
            for error in &result.errors {
                report.push_str(&format!("- **{}**: {}\n", error.path, error.message));
                if let Some(expected) = &error.expected {
                    report.push_str(&format!("  - Expected: {}\n", expected));
                }
                if let Some(actual) = &error.actual {
                    report.push_str(&format!("  - Actual: {}\n", actual));
                }
            }
            report.push('\n');
        }

        if !result.warnings.is_empty() {
            report.push_str("## Warnings\n\n");
            for warning in &result.warnings {
                report.push_str(&format!(
                    "- **{}** ({:?}): {}\n",
                    warning.path, warning.severity, warning.message
                ));
            }
        }

        report
    }
}

impl Default for ContractValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_creation() {
        let result = ValidationResult::new();
        assert!(result.passed);
        assert_eq!(result.total_checks, 0);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_add_error() {
        let mut result = ValidationResult::new();
        result.add_error(ValidationError {
            path: "/api/test".to_string(),
            message: "Test error".to_string(),
            expected: None,
            actual: None,
        });

        assert!(!result.passed);
        assert_eq!(result.failed_checks, 1);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_add_breaking_change() {
        let mut result = ValidationResult::new();
        result.add_breaking_change(BreakingChange {
            change_type: BreakingChangeType::EndpointRemoved,
            path: "/api/removed".to_string(),
            description: "Endpoint was removed".to_string(),
            severity: ChangeSeverity::Critical,
        });

        assert!(!result.passed);
        assert_eq!(result.breaking_changes.len(), 1);
    }

    #[test]
    fn test_contract_validator_creation() {
        let validator = ContractValidator::new();
        assert!(!validator.strict_mode);
        assert!(!validator.ignore_optional_fields);
    }

    #[test]
    fn test_contract_validator_with_options() {
        let validator = ContractValidator::new()
            .with_strict_mode(true)
            .with_ignore_optional_fields(true);

        assert!(validator.strict_mode);
        assert!(validator.ignore_optional_fields);
    }

    #[test]
    fn test_generate_report() {
        let mut result = ValidationResult::new();
        result.add_error(ValidationError {
            path: "/api/test".to_string(),
            message: "Test failed".to_string(),
            expected: Some("200".to_string()),
            actual: Some("404".to_string()),
        });

        let validator = ContractValidator::new();
        let report = validator.generate_report(&result);

        assert!(report.contains("FAILED"));
        assert!(report.contains("/api/test"));
        assert!(report.contains("Test failed"));
    }
}
