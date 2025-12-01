//! Request mutation detection and context analysis
//!
//! This module analyzes changes between requests to detect mutations, identify
//! validation issues, and infer appropriate response types based on context.

use super::context::StatefulAiContext;
use super::types::BehaviorRules;
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Analysis of changes between requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationAnalysis {
    /// Fields that were changed
    pub changed_fields: Vec<FieldChange>,
    /// Fields that were added
    pub added_fields: Vec<String>,
    /// Fields that were removed
    pub removed_fields: Vec<String>,
    /// Detected validation issues
    pub validation_issues: Vec<ValidationIssue>,
    /// Overall mutation type
    pub mutation_type: MutationType,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f64,
}

/// Change detected in a field
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    /// Field name
    pub field: String,
    /// Previous value
    pub previous_value: Value,
    /// New value
    pub new_value: Value,
    /// Change type
    pub change_type: ChangeType,
}

/// Type of change detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ChangeType {
    /// Value was modified
    Modified,
    /// Value type changed
    TypeChanged,
    /// Value was cleared (set to null or empty)
    Cleared,
    /// Value was set (from null/undefined to a value)
    Set,
}

/// Type of mutation detected
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MutationType {
    /// Create operation (new resource)
    Create,
    /// Update operation (existing resource modified)
    Update,
    /// Delete operation (resource removed)
    Delete,
    /// Partial update (only some fields changed)
    PartialUpdate,
    /// No significant changes
    NoChange,
    /// Invalid mutation (validation errors)
    Invalid,
}

/// Validation issue detected from mutation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Field that has the issue (None for object-level issues)
    pub field: Option<String>,
    /// Type of validation issue
    pub issue_type: ValidationIssueType,
    /// Severity of the issue
    pub severity: ValidationSeverity,
    /// Context data for the issue
    pub context: Value,
    /// Suggested error message
    pub error_message: String,
}

/// Type of validation issue
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ValidationIssueType {
    /// Required field is missing
    Required,
    /// Invalid format
    Format,
    /// Value too short
    MinLength,
    /// Value too long
    MaxLength,
    /// Invalid pattern
    Pattern,
    /// Value out of range
    Range,
    /// Invalid type
    Type,
    /// Custom validation error
    Custom,
}

/// Severity of validation issue
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum ValidationSeverity {
    /// Info (non-blocking)
    Info,
    /// Warning (may cause issues)
    Warning,
    /// Error (blocks operation)
    Error,
    /// Critical (severe error)
    Critical,
}

/// Response type inferred from mutation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ResponseType {
    /// Success response (200/201)
    Success,
    /// Validation error (400)
    ValidationError,
    /// Not found (404)
    NotFound,
    /// Conflict (409)
    Conflict,
    /// Unauthorized (401)
    Unauthorized,
    /// Forbidden (403)
    Forbidden,
    /// Server error (500)
    ServerError,
}

/// Mutation analyzer
pub struct MutationAnalyzer {
    /// Behavior rules to use for validation
    rules: Option<BehaviorRules>,
}

impl MutationAnalyzer {
    /// Create a new mutation analyzer
    pub fn new() -> Self {
        Self { rules: None }
    }

    /// Create with behavior rules
    pub fn with_rules(mut self, rules: BehaviorRules) -> Self {
        self.rules = Some(rules);
        self
    }

    /// Analyze mutation between current and previous request
    ///
    /// Compares the current request with the previous request in the session
    /// to detect changes and identify potential validation issues.
    pub async fn analyze_mutation(
        &self,
        current: &Value,
        previous: Option<&Value>,
        context: &StatefulAiContext,
    ) -> Result<MutationAnalysis> {
        let mut changed_fields = Vec::new();
        let mut added_fields = Vec::new();
        let mut removed_fields = Vec::new();

        // If no previous request, this is likely a create operation
        if previous.is_none() {
            if let Value::Object(obj) = current {
                for (key, value) in obj {
                    added_fields.push(key.clone());
                }
            }

            return Ok(MutationAnalysis {
                changed_fields,
                added_fields,
                removed_fields,
                validation_issues: Vec::new(),
                mutation_type: MutationType::Create,
                confidence: 0.9,
            });
        }

        let previous = previous.unwrap();

        // Compare objects
        if let (Value::Object(current_obj), Value::Object(prev_obj)) = (current, previous) {
            // Check for changed fields
            for (key, current_val) in current_obj {
                if let Some(prev_val) = prev_obj.get(key) {
                    if current_val != prev_val {
                        let change_type = self.determine_change_type(prev_val, current_val);
                        changed_fields.push(FieldChange {
                            field: key.clone(),
                            previous_value: prev_val.clone(),
                            new_value: current_val.clone(),
                            change_type,
                        });
                    }
                } else {
                    // Field was added
                    added_fields.push(key.clone());
                }
            }

            // Check for removed fields
            for key in prev_obj.keys() {
                if !current_obj.contains_key(key) {
                    removed_fields.push(key.clone());
                }
            }
        }

        // Detect validation issues
        let validation_issues = self
            .detect_validation_issues(&MutationAnalysis {
                changed_fields: changed_fields.clone(),
                added_fields: added_fields.clone(),
                removed_fields: removed_fields.clone(),
                validation_issues: Vec::new(),
                mutation_type: MutationType::NoChange,
                confidence: 0.0,
            })
            .await?;

        // Determine mutation type
        let mutation_type = self.determine_mutation_type(
            &changed_fields,
            &added_fields,
            &removed_fields,
            &validation_issues,
        );

        // Calculate confidence
        let confidence = self.calculate_confidence(&mutation_type, &validation_issues);

        Ok(MutationAnalysis {
            changed_fields,
            added_fields,
            removed_fields,
            validation_issues,
            mutation_type,
            confidence,
        })
    }

    /// Detect validation issues based on mutation and rules
    pub async fn detect_validation_issues(
        &self,
        mutation: &MutationAnalysis,
    ) -> Result<Vec<ValidationIssue>> {
        let mut issues = Vec::new();

        // Check for required fields if this is a create operation
        if mutation.mutation_type == MutationType::Create {
            // If rules are available, check required fields
            if let Some(ref rules) = self.rules {
                for (resource_name, schema) in &rules.schemas {
                    if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
                        for field_name in required {
                            if let Some(field_str) = field_name.as_str() {
                                // Check if field is missing in added fields
                                if !mutation.added_fields.contains(&field_str.to_string()) {
                                    issues.push(ValidationIssue {
                                        field: Some(field_str.to_string()),
                                        issue_type: ValidationIssueType::Required,
                                        severity: ValidationSeverity::Error,
                                        context: serde_json::json!({
                                            "resource": resource_name,
                                            "field": field_str
                                        }),
                                        error_message: format!("Field '{}' is required", field_str),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check for invalid field changes
        for change in &mutation.changed_fields {
            // Check for type changes
            if change.change_type == ChangeType::TypeChanged {
                issues.push(ValidationIssue {
                    field: Some(change.field.clone()),
                    issue_type: ValidationIssueType::Type,
                    severity: ValidationSeverity::Error,
                    context: serde_json::json!({
                        "previous_type": self.value_type(&change.previous_value),
                        "new_type": self.value_type(&change.new_value)
                    }),
                    error_message: format!(
                        "Field '{}' cannot change type from {} to {}",
                        change.field,
                        self.value_type(&change.previous_value),
                        self.value_type(&change.new_value)
                    ),
                });
            }

            // Check for cleared required fields
            if change.change_type == ChangeType::Cleared {
                if let Some(ref rules) = self.rules {
                    // Check if this field is required in any schema
                    for schema in rules.schemas.values() {
                        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
                            if required
                                .iter()
                                .any(|f| f.as_str().map(|s| s == change.field).unwrap_or(false))
                            {
                                issues.push(ValidationIssue {
                                    field: Some(change.field.clone()),
                                    issue_type: ValidationIssueType::Required,
                                    severity: ValidationSeverity::Error,
                                    context: serde_json::json!({
                                        "field": change.field
                                    }),
                                    error_message: format!(
                                        "Field '{}' cannot be cleared",
                                        change.field
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(issues)
    }

    /// Infer response type from mutation analysis
    pub fn infer_response_type(
        &self,
        mutation: &MutationAnalysis,
        context: &StatefulAiContext,
    ) -> ResponseType {
        // If there are validation errors, return validation error
        if !mutation.validation_issues.is_empty() {
            let has_errors = mutation
                .validation_issues
                .iter()
                .any(|i| i.severity >= ValidationSeverity::Error);
            if has_errors {
                return ResponseType::ValidationError;
            }
        }

        // Determine based on mutation type
        match mutation.mutation_type {
            MutationType::Create => ResponseType::Success,
            MutationType::Update | MutationType::PartialUpdate => ResponseType::Success,
            MutationType::Delete => ResponseType::Success,
            MutationType::Invalid => ResponseType::ValidationError,
            MutationType::NoChange => ResponseType::Success,
        }
    }

    // ===== Private helper methods =====

    /// Determine change type between two values
    fn determine_change_type(&self, previous: &Value, current: &Value) -> ChangeType {
        // Check if previous was null/empty and current has value (Set takes precedence)
        if previous.is_null() || (previous.is_string() && previous.as_str() == Some("")) {
            return ChangeType::Set;
        }

        // Check if current is null/empty
        if current.is_null() || (current.is_string() && current.as_str() == Some("")) {
            return ChangeType::Cleared;
        }

        // Check for type change (after null checks)
        if std::mem::discriminant(previous) != std::mem::discriminant(current) {
            return ChangeType::TypeChanged;
        }

        // Otherwise, it's a modification
        ChangeType::Modified
    }

    /// Determine mutation type from changes
    fn determine_mutation_type(
        &self,
        changed_fields: &[FieldChange],
        added_fields: &[String],
        removed_fields: &[String],
        validation_issues: &[ValidationIssue],
    ) -> MutationType {
        // If there are critical validation issues, it's invalid
        if validation_issues.iter().any(|i| i.severity == ValidationSeverity::Critical) {
            return MutationType::Invalid;
        }

        // If many fields added and few changed, likely create
        if added_fields.len() > changed_fields.len() && removed_fields.is_empty() {
            return MutationType::Create;
        }

        // If fields removed, likely delete or update
        if !removed_fields.is_empty()
            && removed_fields.len() > added_fields.len() + changed_fields.len()
        {
            return MutationType::Delete;
        }

        // If fields changed, determine if it's partial or full update
        if !changed_fields.is_empty() {
            // If we have added or removed fields along with changes, it's a full update
            if !added_fields.is_empty() || !removed_fields.is_empty() {
                return MutationType::Update;
            }
            // If a significant portion of fields changed (more than half), it's a full update
            // For now, treat any update with only changed fields (no adds/removes) as Update
            // PartialUpdate would be for cases where we're updating a subset of a larger object
            // Since we don't have the total field count here, default to Update
            return MutationType::Update;
        }

        MutationType::NoChange
    }

    /// Calculate confidence score
    fn calculate_confidence(
        &self,
        mutation_type: &MutationType,
        validation_issues: &[ValidationIssue],
    ) -> f64 {
        let mut confidence = 0.5; // Base confidence

        // Increase confidence for clear mutation types
        match mutation_type {
            MutationType::Create | MutationType::Delete => confidence += 0.3,
            MutationType::Update | MutationType::PartialUpdate => confidence += 0.2,
            MutationType::Invalid => confidence -= 0.2,
            MutationType::NoChange => confidence = 0.1,
        }

        // Decrease confidence if there are validation issues
        if !validation_issues.is_empty() {
            confidence -= 0.1 * validation_issues.len() as f64;
        }

        confidence.clamp(0.0, 1.0)
    }

    /// Get value type as string
    fn value_type(&self, value: &Value) -> &str {
        match value {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }
}

impl Default for MutationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_analyze_mutation_create() {
        let analyzer = MutationAnalyzer::new();
        let config = super::super::config::IntelligentBehaviorConfig::default();
        let context = StatefulAiContext::new("test_session", config);

        let current = json!({
            "name": "Alice",
            "email": "alice@example.com"
        });

        let analysis = analyzer.analyze_mutation(&current, None, &context).await.unwrap();

        assert_eq!(analysis.mutation_type, MutationType::Create);
        assert!(!analysis.added_fields.is_empty());
    }

    #[tokio::test]
    async fn test_analyze_mutation_update() {
        let analyzer = MutationAnalyzer::new();
        let config = super::super::config::IntelligentBehaviorConfig::default();
        let context = StatefulAiContext::new("test_session", config);

        let previous = json!({
            "name": "Alice",
            "email": "alice@example.com"
        });

        let current = json!({
            "name": "Alice Smith",
            "email": "alice@example.com"
        });

        let analysis =
            analyzer.analyze_mutation(&current, Some(&previous), &context).await.unwrap();

        assert_eq!(analysis.mutation_type, MutationType::Update);
        assert!(!analysis.changed_fields.is_empty());
    }

    #[tokio::test]
    async fn test_determine_change_type() {
        let analyzer = MutationAnalyzer::new();

        let prev = json!("old");
        let curr = json!("new");
        assert_eq!(analyzer.determine_change_type(&prev, &curr), ChangeType::Modified);

        let prev = json!(null);
        let curr = json!("value");
        assert_eq!(analyzer.determine_change_type(&prev, &curr), ChangeType::Set);

        let prev = json!("value");
        let curr = json!(null);
        assert_eq!(analyzer.determine_change_type(&prev, &curr), ChangeType::Cleared);
    }

    #[tokio::test]
    async fn test_infer_response_type() {
        let analyzer = MutationAnalyzer::new();
        let config = super::super::config::IntelligentBehaviorConfig::default();
        let context = StatefulAiContext::new("test_session", config);

        let mutation = MutationAnalysis {
            changed_fields: Vec::new(),
            added_fields: vec!["name".to_string()],
            removed_fields: Vec::new(),
            validation_issues: Vec::new(),
            mutation_type: MutationType::Create,
            confidence: 0.9,
        };

        let response_type = analyzer.infer_response_type(&mutation, &context);
        assert_eq!(response_type, ResponseType::Success);
    }
}
