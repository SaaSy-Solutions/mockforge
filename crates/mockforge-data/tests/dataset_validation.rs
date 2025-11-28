//! Tests for dataset validation functionality.
//!
//! These tests verify that datasets are correctly validated against their
//! schema definitions and that field constraints are properly enforced.

use mockforge_data::{Dataset, FieldDefinition, SchemaDefinition};
use serde_json::json;

#[cfg(test)]
mod dataset_validation_tests {
    use super::*;

    #[test]
    fn test_validate_dataset_valid_data() {
        let schema = SchemaDefinition::new("User".to_string())
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()))
            .with_field(FieldDefinition::new("age".to_string(), "integer".to_string()));

        let data = vec![
            json!({"name": "Alice", "age": 30}),
            json!({"name": "Bob", "age": 25}),
        ];

        let dataset = Dataset::new(Default::default(), data);

        let errors = dataset.validate_against_schema(&schema);
        assert!(errors.is_ok());
        assert!(errors.unwrap().is_empty());
    }

    #[test]
    fn test_validate_dataset_missing_required_field() {
        let schema = SchemaDefinition::new("User".to_string())
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()))
            .with_field(FieldDefinition::new("age".to_string(), "integer".to_string()));

        let data = vec![
            json!({"name": "Alice"}), // Missing age field
            json!({"name": "Bob", "age": 25}),
        ];

        let dataset = Dataset::new(Default::default(), data);

        let errors = dataset.validate_against_schema(&schema);
        assert!(errors.is_ok());
        let error_list = errors.unwrap();
        assert!(!error_list.is_empty());
        assert!(error_list.iter().any(|e| e.contains("Required field 'age' is missing")));
    }

    #[test]
    fn test_validate_dataset_unexpected_field() {
        let schema = SchemaDefinition::new("User".to_string())
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()))
            .with_field(FieldDefinition::new("age".to_string(), "integer".to_string()));

        let data = vec![
            json!({"name": "Alice", "age": 30, "email": "alice@example.com"}), // Unexpected email field
        ];

        let dataset = Dataset::new(Default::default(), data);

        let errors = dataset.validate_against_schema(&schema);
        assert!(errors.is_ok());
        let error_list = errors.unwrap();
        assert!(!error_list.is_empty());
        assert!(error_list.iter().any(|e| e.contains("Unexpected field 'email'")));
    }

    #[test]
    fn test_validate_dataset_type_mismatch() {
        let schema = SchemaDefinition::new("User".to_string())
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()))
            .with_field(FieldDefinition::new("age".to_string(), "integer".to_string()));

        let data = vec![
            json!({"name": "Alice", "age": "thirty"}), // Age should be integer, not string
        ];

        let dataset = Dataset::new(Default::default(), data);

        let errors = dataset.validate_against_schema(&schema);
        assert!(errors.is_ok());
        let error_list = errors.unwrap();
        assert!(!error_list.is_empty());
        assert!(error_list
            .iter()
            .any(|e| e.contains("type mismatch") || e.contains("expected number")));
    }

    #[test]
    fn test_validate_dataset_optional_fields() {
        let schema = SchemaDefinition::new("User".to_string())
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()))
            .with_field(FieldDefinition::new("email".to_string(), "string".to_string()).optional());

        let data = vec![
            json!({"name": "Alice"}), // Email is optional, so this should be valid
            json!({"name": "Bob", "email": "bob@example.com"}),
        ];

        let dataset = Dataset::new(Default::default(), data);

        let errors = dataset.validate_against_schema(&schema);
        assert!(errors.is_ok());
        assert!(errors.unwrap().is_empty());
    }

    #[test]
    fn test_validate_dataset_with_details() {
        let schema = SchemaDefinition::new("User".to_string())
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()))
            .with_field(FieldDefinition::new("age".to_string(), "integer".to_string()));

        let data = vec![
            json!({"name": "Alice", "age": 30}),
            json!({"name": "Bob"}), // Missing age
        ];

        let dataset = Dataset::new(Default::default(), data);

        let result = dataset.validate_with_details(&schema);

        assert!(!result.valid);
        assert!(!result.errors.is_empty());
        assert_eq!(result.total_rows_validated, 2);
        assert!(result.errors.iter().any(|e| e.contains("Required field 'age' is missing")));
    }

    #[test]
    fn test_validate_dataset_size_constraints() {
        let mut schema = SchemaDefinition::new("User".to_string())
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()));

        // Add size constraints to the schema metadata
        schema.metadata.insert("min_rows".to_string(), json!(2));
        schema.metadata.insert("max_rows".to_string(), json!(5));

        let data = vec![
            json!({"name": "Alice"}), // Only 1 row, should fail min_rows constraint
        ];

        let dataset = Dataset::new(Default::default(), data);

        let errors = dataset.validate_against_schema(&schema);
        assert!(errors.is_ok());
        let error_list = errors.unwrap();
        assert!(!error_list.is_empty());
        assert!(error_list.iter().any(|e| e.contains("at least") || e.contains("min_rows")));
    }

    #[test]
    fn test_validate_dataset_complex_schema() {
        let schema = SchemaDefinition::new("Product".to_string())
            .with_field(FieldDefinition::new("id".to_string(), "integer".to_string()))
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()))
            .with_field(FieldDefinition::new("price".to_string(), "number".to_string()))
            .with_field(FieldDefinition::new("tags".to_string(), "array".to_string()).optional());

        let data = vec![
            json!({"id": 1, "name": "Product A", "price": 29.99, "tags": ["electronics", "gadgets"]}),
            json!({"id": 2, "name": "Product B", "price": 49.99}),
            json!({"id": 3, "name": "Product C", "price": 19.99, "tags": "not_an_array"}), // Wrong type for tags
        ];

        let dataset = Dataset::new(Default::default(), data);

        let errors = dataset.validate_against_schema(&schema);
        assert!(errors.is_ok());
        let error_list = errors.unwrap();
        // Should have error for the tags field being wrong type
        assert!(!error_list.is_empty());
    }

    #[test]
    fn test_validate_dataset_empty() {
        let schema = SchemaDefinition::new("User".to_string())
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()));

        let data = vec![];
        let dataset = Dataset::new(Default::default(), data);

        let errors = dataset.validate_against_schema(&schema);
        assert!(errors.is_ok());
        assert!(errors.unwrap().is_empty());
    }

    #[test]
    fn test_validate_dataset_invalid_json_structure() {
        let schema = SchemaDefinition::new("User".to_string())
            .with_field(FieldDefinition::new("name".to_string(), "string".to_string()));

        let data = vec![
            json!("not_an_object"), // Should be an object
        ];

        let dataset = Dataset::new(Default::default(), data);

        let errors = dataset.validate_against_schema(&schema);
        assert!(errors.is_ok());
        let error_list = errors.unwrap();
        assert!(!error_list.is_empty());
        assert!(error_list.iter().any(|e| e.contains("Expected object")));
    }
}
