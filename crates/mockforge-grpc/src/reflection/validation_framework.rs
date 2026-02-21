//! Cross-endpoint validation framework
//!
//! This module provides validation capabilities to ensure data coherence
//! across different endpoints, maintaining referential integrity and
//! business logic constraints in generated mock data.

use crate::reflection::schema_graph::{ForeignKeyMapping, Relationship, SchemaGraph};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info};

/// Configuration for cross-endpoint validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Enable cross-endpoint validation
    pub enabled: bool,
    /// Strict mode - fail on any validation error
    pub strict_mode: bool,
    /// Maximum validation depth for nested relationships
    pub max_validation_depth: usize,
    /// Custom validation rules
    pub custom_rules: Vec<CustomValidationRule>,
    /// Cache validation results for performance
    pub cache_results: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            strict_mode: false,
            max_validation_depth: 3,
            custom_rules: vec![],
            cache_results: true,
        }
    }
}

/// Custom validation rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomValidationRule {
    /// Rule name/identifier
    pub name: String,
    /// Entity types this rule applies to
    pub applies_to_entities: Vec<String>,
    /// Fields this rule validates
    pub validates_fields: Vec<String>,
    /// Rule type
    pub rule_type: ValidationRuleType,
    /// Rule parameters
    pub parameters: HashMap<String, String>,
    /// Error message template
    pub error_message: String,
}

/// Types of validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationRuleType {
    /// Foreign key existence validation
    ForeignKeyExists,
    /// Field format validation
    FieldFormat,
    /// Range validation
    Range,
    /// Unique constraint validation
    Unique,
    /// Business logic validation
    BusinessLogic,
    /// Custom validation function
    Custom,
}

/// Result of a validation operation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed
    pub is_valid: bool,
    /// Validation errors found
    pub errors: Vec<ValidationError>,
    /// Warnings (non-fatal issues)
    pub warnings: Vec<ValidationWarning>,
    /// Entities that were validated
    pub validated_entities: Vec<String>,
}

/// A validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    /// Error type
    pub error_type: ValidationErrorType,
    /// Entity where error occurred
    pub entity_name: String,
    /// Field where error occurred
    pub field_name: String,
    /// Error message
    pub message: String,
    /// Value that caused the error
    pub invalid_value: String,
    /// Suggested fix (if available)
    pub suggested_fix: Option<String>,
}

/// A validation warning
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    /// Warning type
    pub warning_type: ValidationWarningType,
    /// Entity where warning occurred
    pub entity_name: String,
    /// Field where warning occurred (optional)
    pub field_name: Option<String>,
    /// Warning message
    pub message: String,
}

/// Types of validation errors
#[derive(Debug, Clone, PartialEq)]
pub enum ValidationErrorType {
    /// Foreign key references non-existent entity
    ForeignKeyNotFound,
    /// Field format is invalid
    InvalidFormat,
    /// Value is outside allowed range
    OutOfRange,
    /// Unique constraint violation
    DuplicateValue,
    /// Business logic constraint violation
    BusinessRuleViolation,
    /// Circular reference detected
    CircularReference,
}

/// Types of validation warnings
#[derive(Debug, Clone)]
pub enum ValidationWarningType {
    /// Potential data inconsistency
    DataInconsistency,
    /// Performance concern
    PerformanceConcern,
    /// Best practice violation
    BestPracticeViolation,
}

/// Data store for validation - tracks generated entities
#[derive(Debug, Default)]
pub struct ValidationDataStore {
    /// Generated entities by type
    entities: HashMap<String, Vec<GeneratedEntity>>,
    /// Index for fast foreign key lookups
    foreign_key_index: HashMap<String, HashMap<String, Vec<usize>>>,
}

/// A generated entity for validation
#[derive(Debug, Clone)]
pub struct GeneratedEntity {
    /// Entity type name
    pub entity_type: String,
    /// Primary key value (if available)
    pub primary_key: Option<String>,
    /// All field values
    pub field_values: HashMap<String, String>,
    /// Endpoint this entity was generated for
    pub endpoint: String,
    /// Generation timestamp
    pub generated_at: std::time::SystemTime,
}

/// Cross-endpoint validation framework
pub struct ValidationFramework {
    /// Configuration
    config: ValidationConfig,
    /// Schema graph for relationship validation
    schema_graph: Option<SchemaGraph>,
    /// Data store for tracking generated entities
    data_store: ValidationDataStore,
    /// Validation cache
    validation_cache: HashMap<String, ValidationResult>,
}

impl ValidationFramework {
    /// Create a new validation framework
    pub fn new(config: ValidationConfig) -> Self {
        Self {
            config,
            schema_graph: None,
            data_store: ValidationDataStore::default(),
            validation_cache: HashMap::new(),
        }
    }

    /// Set the schema graph for relationship validation
    pub fn set_schema_graph(&mut self, schema_graph: SchemaGraph) {
        info!(
            "Setting schema graph with {} entities for validation",
            schema_graph.entities.len()
        );
        self.schema_graph = Some(schema_graph);
    }

    /// Register a generated entity for validation
    pub fn register_entity(
        &mut self,
        entity: GeneratedEntity,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        debug!("Registering entity {} for endpoint {}", entity.entity_type, entity.endpoint);

        let entity_type = entity.entity_type.clone();
        let primary_key = entity.primary_key.clone();

        // Add to entities list
        let entities_list = self.data_store.entities.entry(entity_type.clone()).or_default();
        let entity_index = entities_list.len();
        entities_list.push(entity);

        // Update foreign key index if primary key exists
        if let Some(pk) = primary_key {
            let type_index = self.data_store.foreign_key_index.entry(entity_type).or_default();
            let pk_index = type_index.entry(pk).or_default();
            pk_index.push(entity_index);
        }

        Ok(())
    }

    /// Validate all registered entities for cross-endpoint consistency
    pub fn validate_all_entities(&mut self) -> ValidationResult {
        if !self.config.enabled {
            return ValidationResult {
                is_valid: true,
                errors: vec![],
                warnings: vec![],
                validated_entities: vec![],
            };
        }

        info!(
            "Starting cross-endpoint validation of {} entity types",
            self.data_store.entities.len()
        );

        let mut result = ValidationResult {
            is_valid: true,
            errors: vec![],
            warnings: vec![],
            validated_entities: vec![],
        };

        // Validate foreign key relationships
        self.validate_foreign_key_relationships(&mut result);

        // Validate custom rules
        self.validate_custom_rules(&mut result);

        // Validate referential integrity
        self.validate_referential_integrity(&mut result);

        // Check for potential issues
        self.check_data_consistency(&mut result);

        result.is_valid = result.errors.is_empty() || !self.config.strict_mode;

        info!(
            "Validation completed: {} errors, {} warnings",
            result.errors.len(),
            result.warnings.len()
        );

        result
    }

    /// Validate foreign key relationships
    fn validate_foreign_key_relationships(&self, result: &mut ValidationResult) {
        if let Some(schema_graph) = &self.schema_graph {
            for (entity_type, entities) in &self.data_store.entities {
                result.validated_entities.push(entity_type.clone());

                // Get foreign key mappings for this entity
                if let Some(fk_mappings) = schema_graph.foreign_keys.get(entity_type) {
                    for entity in entities {
                        self.validate_entity_foreign_keys(entity, fk_mappings, result);
                    }
                }
            }
        }
    }

    /// Validate foreign keys for a specific entity
    fn validate_entity_foreign_keys(
        &self,
        entity: &GeneratedEntity,
        fk_mappings: &[ForeignKeyMapping],
        result: &mut ValidationResult,
    ) {
        for mapping in fk_mappings {
            if let Some(fk_value) = entity.field_values.get(&mapping.field_name) {
                // Check if the referenced entity exists
                if !self.foreign_key_exists(&mapping.target_entity, fk_value) {
                    result.errors.push(ValidationError {
                        error_type: ValidationErrorType::ForeignKeyNotFound,
                        entity_name: entity.entity_type.clone(),
                        field_name: mapping.field_name.clone(),
                        message: format!(
                            "Foreign key '{}' references non-existent {} with value '{}'",
                            mapping.field_name, mapping.target_entity, fk_value
                        ),
                        invalid_value: fk_value.clone(),
                        suggested_fix: Some(format!(
                            "Create a {} entity with primary key '{}'",
                            mapping.target_entity, fk_value
                        )),
                    });
                }
            }
        }
    }

    /// Check if a foreign key value exists
    fn foreign_key_exists(&self, target_entity: &str, key_value: &str) -> bool {
        if let Some(type_index) = self.data_store.foreign_key_index.get(target_entity) {
            type_index.contains_key(key_value)
        } else {
            false
        }
    }

    /// Validate custom validation rules
    fn validate_custom_rules(&self, result: &mut ValidationResult) {
        for rule in &self.config.custom_rules {
            for entity_type in &rule.applies_to_entities {
                if let Some(entities) = self.data_store.entities.get(entity_type) {
                    for entity in entities {
                        self.validate_entity_against_rule(entity, rule, result);
                    }
                }
            }
        }
    }

    /// Validate an entity against a custom rule
    fn validate_entity_against_rule(
        &self,
        entity: &GeneratedEntity,
        rule: &CustomValidationRule,
        result: &mut ValidationResult,
    ) {
        match &rule.rule_type {
            ValidationRuleType::ForeignKeyExists => {
                self.validate_foreign_key_rule(entity, rule, result);
            }
            ValidationRuleType::FieldFormat => {
                self.validate_field_format(entity, rule, result);
            }
            ValidationRuleType::Range => {
                self.validate_field_range(entity, rule, result);
            }
            ValidationRuleType::Unique => {
                self.validate_field_uniqueness(entity, rule, result);
            }
            ValidationRuleType::BusinessLogic | ValidationRuleType::Custom => {
                // Evaluate simple expression-style rules if configured: field/operator/value.
                // Unknown operators/rules are surfaced as best-practice warnings.
                self.validate_business_logic_rule(entity, rule, result);
            }
        }
    }

    /// Validate explicit foreign key existence rule.
    /// Parameters:
    /// - target_entity: referenced entity type
    fn validate_foreign_key_rule(
        &self,
        entity: &GeneratedEntity,
        rule: &CustomValidationRule,
        result: &mut ValidationResult,
    ) {
        let Some(target_entity) = rule.parameters.get("target_entity") else {
            result.warnings.push(ValidationWarning {
                warning_type: ValidationWarningType::BestPracticeViolation,
                entity_name: entity.entity_type.clone(),
                field_name: None,
                message: format!(
                    "Rule '{}' is missing required parameter 'target_entity'",
                    rule.name
                ),
            });
            return;
        };

        for field_name in &rule.validates_fields {
            if let Some(fk_value) = entity.field_values.get(field_name) {
                if !self.foreign_key_exists(target_entity, fk_value) {
                    result.errors.push(ValidationError {
                        error_type: ValidationErrorType::ForeignKeyNotFound,
                        entity_name: entity.entity_type.clone(),
                        field_name: field_name.clone(),
                        message: rule.error_message.clone(),
                        invalid_value: fk_value.clone(),
                        suggested_fix: Some(format!(
                            "Create a {} entity with primary key '{}'",
                            target_entity, fk_value
                        )),
                    });
                }
            }
        }
    }

    /// Validate basic business logic/custom rule expressions.
    /// Supported parameters:
    /// - field: field name to validate
    /// - operator: eq|ne|contains|starts_with|ends_with
    /// - value: expected value
    fn validate_business_logic_rule(
        &self,
        entity: &GeneratedEntity,
        rule: &CustomValidationRule,
        result: &mut ValidationResult,
    ) {
        let Some(field) = rule.parameters.get("field") else {
            result.warnings.push(ValidationWarning {
                warning_type: ValidationWarningType::BestPracticeViolation,
                entity_name: entity.entity_type.clone(),
                field_name: None,
                message: format!(
                    "Rule '{}' skipped: missing 'field' parameter for {:?}",
                    rule.name, rule.rule_type
                ),
            });
            return;
        };
        let Some(operator) = rule.parameters.get("operator") else {
            result.warnings.push(ValidationWarning {
                warning_type: ValidationWarningType::BestPracticeViolation,
                entity_name: entity.entity_type.clone(),
                field_name: Some(field.clone()),
                message: format!(
                    "Rule '{}' skipped: missing 'operator' parameter for {:?}",
                    rule.name, rule.rule_type
                ),
            });
            return;
        };
        let Some(expected) = rule.parameters.get("value") else {
            result.warnings.push(ValidationWarning {
                warning_type: ValidationWarningType::BestPracticeViolation,
                entity_name: entity.entity_type.clone(),
                field_name: Some(field.clone()),
                message: format!(
                    "Rule '{}' skipped: missing 'value' parameter for {:?}",
                    rule.name, rule.rule_type
                ),
            });
            return;
        };

        if let Some(actual) = entity.field_values.get(field) {
            let passed = match operator.as_str() {
                "eq" => actual == expected,
                "ne" => actual != expected,
                "contains" => actual.contains(expected),
                "starts_with" => actual.starts_with(expected),
                "ends_with" => actual.ends_with(expected),
                _ => {
                    result.warnings.push(ValidationWarning {
                        warning_type: ValidationWarningType::BestPracticeViolation,
                        entity_name: entity.entity_type.clone(),
                        field_name: Some(field.clone()),
                        message: format!(
                            "Rule '{}' uses unsupported operator '{}'",
                            rule.name, operator
                        ),
                    });
                    return;
                }
            };

            if !passed {
                result.errors.push(ValidationError {
                    error_type: ValidationErrorType::BusinessRuleViolation,
                    entity_name: entity.entity_type.clone(),
                    field_name: field.clone(),
                    message: rule.error_message.clone(),
                    invalid_value: actual.clone(),
                    suggested_fix: Some(format!(
                        "Expected '{}' {} '{}'",
                        field, operator, expected
                    )),
                });
            }
        }
    }

    /// Validate field format
    fn validate_field_format(
        &self,
        entity: &GeneratedEntity,
        rule: &CustomValidationRule,
        result: &mut ValidationResult,
    ) {
        for field_name in &rule.validates_fields {
            if let Some(field_value) = entity.field_values.get(field_name) {
                if let Some(pattern) = rule.parameters.get("pattern") {
                    if let Ok(regex) = regex::Regex::new(pattern) {
                        if !regex.is_match(field_value) {
                            result.errors.push(ValidationError {
                                error_type: ValidationErrorType::InvalidFormat,
                                entity_name: entity.entity_type.clone(),
                                field_name: field_name.clone(),
                                message: rule.error_message.clone(),
                                invalid_value: field_value.clone(),
                                suggested_fix: Some(format!(
                                    "Value should match pattern: {}",
                                    pattern
                                )),
                            });
                        }
                    }
                }
            }
        }
    }

    /// Validate field range constraints
    fn validate_field_range(
        &self,
        entity: &GeneratedEntity,
        rule: &CustomValidationRule,
        result: &mut ValidationResult,
    ) {
        for field_name in &rule.validates_fields {
            if let Some(field_value) = entity.field_values.get(field_name) {
                if let Ok(value) = field_value.parse::<f64>() {
                    let min = rule.parameters.get("min").and_then(|s| s.parse::<f64>().ok());
                    let max = rule.parameters.get("max").and_then(|s| s.parse::<f64>().ok());

                    let out_of_range = (min.is_some() && value < min.unwrap())
                        || (max.is_some() && value > max.unwrap());

                    if out_of_range {
                        result.errors.push(ValidationError {
                            error_type: ValidationErrorType::OutOfRange,
                            entity_name: entity.entity_type.clone(),
                            field_name: field_name.clone(),
                            message: rule.error_message.clone(),
                            invalid_value: field_value.clone(),
                            suggested_fix: Some(format!(
                                "Value should be between {} and {}",
                                min.map_or("any".to_string(), |v| v.to_string()),
                                max.map_or("any".to_string(), |v| v.to_string())
                            )),
                        });
                    }
                }
            }
        }
    }

    /// Validate field uniqueness constraints
    fn validate_field_uniqueness(
        &self,
        entity: &GeneratedEntity,
        rule: &CustomValidationRule,
        result: &mut ValidationResult,
    ) {
        for field_name in &rule.validates_fields {
            if let Some(field_value) = entity.field_values.get(field_name) {
                // Check if this value appears in other entities
                let mut duplicate_count = 0;

                if let Some(entities) = self.data_store.entities.get(&entity.entity_type) {
                    for other_entity in entities {
                        if let Some(other_value) = other_entity.field_values.get(field_name) {
                            if other_value == field_value {
                                duplicate_count += 1;
                            }
                        }
                    }
                }

                if duplicate_count > 1 {
                    result.errors.push(ValidationError {
                        error_type: ValidationErrorType::DuplicateValue,
                        entity_name: entity.entity_type.clone(),
                        field_name: field_name.clone(),
                        message: rule.error_message.clone(),
                        invalid_value: field_value.clone(),
                        suggested_fix: Some("Generate unique values for this field".to_string()),
                    });
                }
            }
        }
    }

    /// Validate referential integrity across endpoints
    fn validate_referential_integrity(&self, result: &mut ValidationResult) {
        if let Some(schema_graph) = &self.schema_graph {
            for relationship in &schema_graph.relationships {
                self.validate_relationship_integrity(relationship, result);
            }
        }
    }

    /// Validate a specific relationship's integrity
    fn validate_relationship_integrity(
        &self,
        relationship: &Relationship,
        result: &mut ValidationResult,
    ) {
        if let (Some(from_entities), Some(to_entities)) = (
            self.data_store.entities.get(&relationship.from_entity),
            self.data_store.entities.get(&relationship.to_entity),
        ) {
            for from_entity in from_entities {
                if let Some(ref_value) = from_entity.field_values.get(&relationship.field_name) {
                    let target_exists = to_entities
                        .iter()
                        .any(|to_entity| to_entity.primary_key.as_ref() == Some(ref_value));

                    if !target_exists && relationship.is_required {
                        result.warnings.push(ValidationWarning {
                            warning_type: ValidationWarningType::DataInconsistency,
                            entity_name: from_entity.entity_type.clone(),
                            field_name: Some(relationship.field_name.clone()),
                            message: format!(
                                "Required relationship from {} to {} not satisfied - referenced {} '{}' does not exist",
                                relationship.from_entity, relationship.to_entity,
                                relationship.to_entity, ref_value
                            ),
                        });
                    }
                }
            }
        }
    }

    /// Check for general data consistency issues
    fn check_data_consistency(&self, result: &mut ValidationResult) {
        // Check for entities that are never referenced
        self.check_orphaned_entities(result);

        // Check for potential performance issues
        self.check_performance_concerns(result);
    }

    /// Check for entities that might be orphaned
    fn check_orphaned_entities(&self, result: &mut ValidationResult) {
        if let Some(schema_graph) = &self.schema_graph {
            for (entity_type, entity_node) in &schema_graph.entities {
                if entity_node.referenced_by.is_empty() && !entity_node.is_root {
                    if let Some(entities) = self.data_store.entities.get(entity_type) {
                        if !entities.is_empty() {
                            result.warnings.push(ValidationWarning {
                                warning_type: ValidationWarningType::DataInconsistency,
                                entity_name: entity_type.clone(),
                                field_name: None,
                                message: format!(
                                    "Entity type {} is not referenced by any other entities but {} instances were generated",
                                    entity_type, entities.len()
                                ),
                            });
                        }
                    }
                }
            }
        }
    }

    /// Check for potential performance concerns
    fn check_performance_concerns(&self, result: &mut ValidationResult) {
        for (entity_type, entities) in &self.data_store.entities {
            if entities.len() > 10000 {
                result.warnings.push(ValidationWarning {
                    warning_type: ValidationWarningType::PerformanceConcern,
                    entity_name: entity_type.clone(),
                    field_name: None,
                    message: format!(
                        "Large number of {} entities ({}) may impact performance",
                        entity_type,
                        entities.len()
                    ),
                });
            }
        }
    }

    /// Clear all validation data
    pub fn clear(&mut self) {
        self.data_store.entities.clear();
        self.data_store.foreign_key_index.clear();
        self.validation_cache.clear();
        info!("Validation framework data cleared");
    }

    /// Get validation statistics
    pub fn get_statistics(&self) -> ValidationStatistics {
        let total_entities: usize = self.data_store.entities.values().map(|v| v.len()).sum();
        let entity_type_count = self.data_store.entities.len();
        let indexed_keys: usize = self
            .data_store
            .foreign_key_index
            .values()
            .map(|type_index| type_index.len())
            .sum();

        ValidationStatistics {
            total_entities,
            entity_type_count,
            indexed_foreign_keys: indexed_keys,
            cache_size: self.validation_cache.len(),
        }
    }
}

/// Validation framework statistics
#[derive(Debug, Clone)]
pub struct ValidationStatistics {
    /// Total number of entities tracked
    pub total_entities: usize,
    /// Number of different entity types
    pub entity_type_count: usize,
    /// Number of indexed foreign key values
    pub indexed_foreign_keys: usize,
    /// Size of validation cache
    pub cache_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    #[test]
    fn test_validation_framework_creation() {
        let config = ValidationConfig::default();
        let framework = ValidationFramework::new(config);
        assert!(framework.config.enabled);
        assert!(!framework.config.strict_mode);
    }

    #[test]
    fn test_entity_registration() {
        let config = ValidationConfig::default();
        let mut framework = ValidationFramework::new(config);

        let entity = GeneratedEntity {
            entity_type: "User".to_string(),
            primary_key: Some("user_123".to_string()),
            field_values: HashMap::from([
                ("id".to_string(), "user_123".to_string()),
                ("name".to_string(), "John Doe".to_string()),
            ]),
            endpoint: "/users".to_string(),
            generated_at: SystemTime::now(),
        };

        framework.register_entity(entity).expect("Should register entity successfully");

        let stats = framework.get_statistics();
        assert_eq!(stats.total_entities, 1);
        assert_eq!(stats.entity_type_count, 1);
    }

    #[test]
    fn test_validation_with_no_schema() {
        let config = ValidationConfig::default();
        let mut framework = ValidationFramework::new(config);

        // Add some entities
        let entity1 = GeneratedEntity {
            entity_type: "User".to_string(),
            primary_key: Some("1".to_string()),
            field_values: HashMap::from([("id".to_string(), "1".to_string())]),
            endpoint: "/users".to_string(),
            generated_at: SystemTime::now(),
        };

        framework.register_entity(entity1).unwrap();

        let result = framework.validate_all_entities();
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_custom_validation_rule() {
        let mut config = ValidationConfig::default();
        config.custom_rules.push(CustomValidationRule {
            name: "email_format".to_string(),
            applies_to_entities: vec!["User".to_string()],
            validates_fields: vec!["email".to_string()],
            rule_type: ValidationRuleType::FieldFormat,
            parameters: HashMap::from([(
                "pattern".to_string(),
                r"^[^@]+@[^@]+\.[^@]+$".to_string(),
            )]),
            error_message: "Invalid email format".to_string(),
        });

        let mut framework = ValidationFramework::new(config);

        let entity = GeneratedEntity {
            entity_type: "User".to_string(),
            primary_key: Some("1".to_string()),
            field_values: HashMap::from([
                ("id".to_string(), "1".to_string()),
                ("email".to_string(), "invalid-email".to_string()),
            ]),
            endpoint: "/users".to_string(),
            generated_at: SystemTime::now(),
        };

        framework.register_entity(entity).unwrap();

        let result = framework.validate_all_entities();
        assert!(!result.errors.is_empty());
        assert_eq!(result.errors[0].error_type, ValidationErrorType::InvalidFormat);
    }
}
