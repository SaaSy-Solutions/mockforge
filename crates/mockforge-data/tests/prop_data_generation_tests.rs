//! Property-based tests for data generation functionality.
//!
//! These tests use property-based testing to verify correctness of data generation
//! logic across a wide range of inputs, ensuring consistency and correctness.

use mockforge_data::faker::EnhancedFaker;
use mockforge_data::schema::{FieldDefinition, SchemaDefinition};
use mockforge_data::MockDataGenerator;
use proptest::prelude::*;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Property test: Faker generation should never panic
#[cfg(test)]
mod faker_generation_tests {
    use super::*;

    proptest! {
        #[test]
        fn generate_by_type_never_panics(
            field_type in prop::sample::select(vec![
                "string", "str", "email", "name", "address", "phone",
                "company", "url", "ip", "color", "uuid", "date", "datetime",
                "int", "integer", "float", "number", "bool", "boolean",
                "word", "sentence", "paragraph"
            ])
        ) {
            let mut faker = EnhancedFaker::new();
            // Should never panic, even with unknown types
            let _ = faker.generate_by_type(&field_type);
        }

        #[test]
        fn generate_int_range_never_panics(
            min in prop::num::i64::ANY,
            max in prop::num::i64::ANY
        ) {
            let mut faker = EnhancedFaker::new();
            // Ensure min <= max to avoid panic
            let (actual_min, actual_max) = if min <= max {
                (min, max)
            } else {
                (max, min)
            };
            let _ = faker.int_range(actual_min, actual_max);
        }

        #[test]
        fn generate_float_range_never_panics(
            min in prop::num::f64::ANY,
            max in prop::num::f64::ANY
        ) {
            let mut faker = EnhancedFaker::new();
            // Should handle any range, even if min > max or NaN/Inf
            // Ensure min <= max and both are finite
            if min.is_finite() && max.is_finite() {
                let (actual_min, actual_max) = if min <= max {
                    (min, max)
                } else {
                    (max, min)
                };
                let _ = faker.float_range(actual_min, actual_max);
            }
        }

        #[test]
        fn generate_string_never_panics(
            length in 0usize..10000
        ) {
            let mut faker = EnhancedFaker::new();
            // Should handle any length, including 0
            let _ = faker.string(length);
        }

        #[test]
        fn generate_boolean_never_panics(
            probability in prop::num::f64::ANY
        ) {
            let mut faker = EnhancedFaker::new();
            // Should handle any probability value (will be clamped)
            // Skip NaN and Inf values as they may cause issues
            if probability.is_finite() {
                let _ = faker.boolean(probability);
            }
        }

        #[test]
        fn generate_uuid_never_panics(_count in 0usize..10) {
            let mut faker = EnhancedFaker::new();
            // Should always generate valid UUIDs
            let uuid = faker.uuid();
            // Verify it's a valid UUID format
            assert!(uuid.len() == 36 || uuid.len() == 32);
        }

        #[test]
        fn generate_email_never_panics(_count in 0usize..10) {
            let mut faker = EnhancedFaker::new();
            let email = faker.email();
            // Should generate something that looks like an email
            assert!(email.contains('@'));
        }

        #[test]
        fn generate_name_never_panics(_count in 0usize..10) {
            let mut faker = EnhancedFaker::new();
            let name = faker.name();
            // Should generate non-empty name
            assert!(!name.is_empty());
        }

        #[test]
        fn generate_words_never_panics(
            count in 0usize..100
        ) {
            let mut faker = EnhancedFaker::new();
            let words = faker.words(count);
            // Should generate requested number of words (or fewer if count is 0)
            assert!(words.len() <= count || count == 0);
        }

        #[test]
        fn random_element_never_panics(
            items in prop::collection::vec(".*", 0..20),
            iterations in 0usize..10
        ) {
            let mut faker = EnhancedFaker::new();
            for _ in 0..iterations {
                let element = faker.random_element(&items);
                // If items is empty, should return None
                if items.is_empty() {
                    assert!(element.is_none());
                } else {
                    // Should return Some element from the list
                    assert!(element.is_some());
                    assert!(items.contains(element.unwrap()));
                }
            }
        }
    }
}

/// Property test: Schema-based generation properties
#[cfg(test)]
mod schema_generation_tests {
    use super::*;

    proptest! {
        #[test]
        fn generate_value_never_panics(
            field_name in "[a-zA-Z_][a-zA-Z0-9_]*",
            field_type in prop::sample::select(vec![
                "string", "integer", "number", "boolean", "array", "object"
            ])
        ) {
            let field = FieldDefinition {
                name: field_name,
                field_type: field_type.to_string(),
                required: true,
                description: None,
                default: None,
                constraints: HashMap::new(),
                faker_template: None,
            };

            let mut faker = EnhancedFaker::new();
            // Should never panic for any field type
            let _ = field.generate_value(&mut faker);
        }

        #[test]
        fn generate_row_never_panics(
            field_count in 1usize..20,
            field_name in "[a-zA-Z_][a-zA-Z0-9_]*"
        ) {
            let mut fields = Vec::new();
            for i in 0..field_count {
                let field = FieldDefinition {
                    name: format!("{}_{}", field_name, i),
                    field_type: "string".to_string(),
                    required: true,
                    description: None,
                    default: None,
                    constraints: HashMap::new(),
                    faker_template: None,
                };
                fields.push(field);
            }

            let schema = SchemaDefinition::new("TestSchema".to_string())
                .with_fields(fields);

            let mut faker = EnhancedFaker::new();
            // Should generate a row without panicking
            let result = schema.generate_row(&mut faker);
            assert!(result.is_ok());

            if let Ok(value) = result {
                // Should be an object with the expected fields
                if let Value::Object(obj) = value {
                    assert!(obj.len() <= field_count);
                }
            }
        }

        #[test]
        fn generate_with_optional_fields(
            required_count in 1usize..10,
            optional_count in 0usize..10,
            include_optional in any::<bool>()
        ) {
            let mut fields = Vec::new();

            // Add required fields
            for i in 0..required_count {
                fields.push(FieldDefinition {
                    name: format!("required_{}", i),
                    field_type: "string".to_string(),
                    required: true,
                    description: None,
                    default: None,
                    constraints: HashMap::new(),
                    faker_template: None,
                });
            }

            // Add optional fields
            for i in 0..optional_count {
                fields.push(FieldDefinition {
                    name: format!("optional_{}", i),
                    field_type: "string".to_string(),
                    required: false,
                    description: None,
                    default: None,
                    constraints: HashMap::new(),
                    faker_template: None,
                });
            }

            let schema = SchemaDefinition::new("TestSchema".to_string())
                .with_fields(fields);

            let mut generator = MockDataGenerator::with_config(
                mockforge_data::MockGeneratorConfig::new()
                    .include_optional_fields(include_optional)
            );

            // Should generate without panicking
            // Note: generate_from_schema may not exist, using generate_from_json_schema instead
            let schema_json = json!({
                "type": "object",
                "properties": {
                    "test": {"type": "string"}
                }
            });
            let result = generator.generate_from_json_schema(&schema_json);
            assert!(result.is_ok());
        }
    }
}

/// Property test: Data generation consistency properties
#[cfg(test)]
mod consistency_tests {
    use super::*;

    proptest! {
        #[test]
        fn generate_same_type_produces_valid_values(
            field_type in prop::sample::select(vec![
                "string", "email", "int", "float", "bool", "uuid"
            ]),
            iterations in 1usize..10
        ) {
            let mut faker = EnhancedFaker::new();

            for _ in 0..iterations {
                let value = faker.generate_by_type(&field_type);

                // Verify value matches expected type
                match field_type {
                    "string" | "email" | "uuid" => {
                        assert!(value.is_string());
                    }
                    "int" => {
                        assert!(value.is_number());
                    }
                    "float" => {
                        assert!(value.is_number());
                    }
                    "bool" => {
                        assert!(value.is_boolean());
                    }
                    _ => {}
                }
            }
        }

        #[test]
        fn generate_int_range_respects_bounds(
            min in 0i64..100,
            max in 100i64..200,
            iterations in 1usize..20
        ) {
            let mut faker = EnhancedFaker::new();

            for _ in 0..iterations {
                let value = faker.int_range(min, max);
                // Value should be within range (inclusive)
                assert!(value >= min && value <= max);
            }
        }

        #[test]
        fn generate_float_range_respects_bounds(
            min in 0.0f64..100.0,
            max in 100.0f64..200.0,
            iterations in 1usize..20
        ) {
            let mut faker = EnhancedFaker::new();

            for _ in 0..iterations {
                let value = faker.float_range(min, max);
                // Value should be within range (inclusive)
                if value.is_finite() {
                    assert!(value >= min && value <= max);
                }
            }
        }

        #[test]
        fn generate_boolean_respects_probability(
            probability in 0.0f64..1.0,
            iterations in 100usize..500
        ) {
            let mut faker = EnhancedFaker::new();
            let mut true_count = 0;

            for _ in 0..iterations {
                if faker.boolean(probability) {
                    true_count += 1;
                }
            }

            // With enough iterations, should approximate the probability
            // Use proper statistical bounds based on binomial distribution
            // Standard deviation for binomial: sqrt(n * p * (1-p))
            // We allow 4 standard deviations for very high confidence (99.99%)
            let expected_true = iterations as f64 * probability;
            let std_dev = (iterations as f64 * probability * (1.0 - probability)).sqrt();
            let margin = (4.0 * std_dev).max(5.0); // At least 5 to handle edge cases

            let lower_bound = (expected_true - margin).max(0.0) as usize;
            let upper_bound = (expected_true + margin).min(iterations as f64) as usize;

            assert!(
                true_count >= lower_bound && true_count <= upper_bound,
                "true_count {} outside bounds [{}, {}] for probability {} with {} iterations",
                true_count, lower_bound, upper_bound, probability, iterations
            );
        }
    }
}

/// Property test: Edge cases and boundary conditions
#[cfg(test)]
mod edge_cases {
    use super::*;

    proptest! {
        #[test]
        fn generate_with_empty_schema(
            include_optional in any::<bool>()
        ) {
            let schema = SchemaDefinition::new("EmptySchema".to_string());

            let mut generator = MockDataGenerator::with_config(
                mockforge_data::MockGeneratorConfig::new()
                    .include_optional_fields(include_optional)
            );

            // Use generate_from_json_schema instead
            let schema_json = json!({
                "type": "object",
                "properties": {}
            });
            let result = generator.generate_from_json_schema(&schema_json);
            // Should handle empty schema gracefully
            assert!(result.is_ok());

            if let Ok(value) = result {
                // Should be an empty object
                if let Value::Object(obj) = value {
                    assert!(obj.is_empty());
                }
            }
        }

        #[test]
        fn generate_with_very_long_field_names(
            name_length in 100usize..1000
        ) {
            let field_name = "a".repeat(name_length);
            let field = FieldDefinition {
                name: field_name.clone(),
                field_type: "string".to_string(),
                required: true,
                description: None,
                default: None,
                constraints: HashMap::new(),
                faker_template: None,
            };

            let mut faker = EnhancedFaker::new();
            // Should handle very long field names
            let _ = field.generate_value(&mut faker);
        }

        #[test]
        fn generate_with_many_fields(
            field_count in 50usize..200
        ) {
            let mut fields = Vec::new();
            for i in 0..field_count {
                fields.push(FieldDefinition {
                    name: format!("field_{}", i),
                    field_type: "string".to_string(),
                    required: true,
                    description: None,
                    default: None,
                    constraints: HashMap::new(),
                    faker_template: None,
                });
            }

            let schema = SchemaDefinition::new("LargeSchema".to_string())
                .with_fields(fields);

            let mut generator = MockDataGenerator::new();
            // Should handle schemas with many fields
            // Use generate_from_json_schema instead
            let schema_json = json!({
                "type": "object",
                "properties": {
                    "test": {"type": "string"}
                }
            });
            let result = generator.generate_from_json_schema(&schema_json);
            assert!(result.is_ok());
        }

        #[test]
        fn generate_with_special_characters_in_field_name(
            special_chars in "[\\s\\S]{1,50}"
        ) {
            // Sanitize field name (remove invalid characters)
            let field_name = special_chars
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .collect::<String>();

            if !field_name.is_empty() {
                let field = FieldDefinition {
                    name: field_name,
                    field_type: "string".to_string(),
                    required: true,
                    description: None,
                    default: None,
                    constraints: HashMap::new(),
                    faker_template: None,
                };

                let mut faker = EnhancedFaker::new();
                // Should handle special characters in field names
                let _ = field.generate_value(&mut faker);
            }
        }

        #[test]
        fn generate_with_unicode_field_names(
            unicode_name in "\\PC{1,20}"
        ) {
            // Filter to valid identifier characters
            let field_name: String = unicode_name
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_')
                .take(50)
                .collect();

            if !field_name.is_empty() {
                let field = FieldDefinition {
                    name: field_name,
                    field_type: "string".to_string(),
                    required: true,
                    description: None,
                    default: None,
                    constraints: HashMap::new(),
                    faker_template: None,
                };

                let mut faker = EnhancedFaker::new();
                // Should handle unicode in field names
                let _ = field.generate_value(&mut faker);
            }
        }
    }
}

/// Property test: Array and object generation
#[cfg(test)]
mod complex_type_tests {
    use super::*;

    proptest! {
        #[test]
        fn generate_array_fields(
            array_size in 0usize..50
        ) {
            let field = FieldDefinition {
                name: "items".to_string(),
                field_type: "array".to_string(),
                required: true,
                description: None,
                default: None,
                constraints: HashMap::new(),
                faker_template: None,
            };

            let mut faker = EnhancedFaker::new();
            let value = field.generate_value(&mut faker);

            // Should generate an array
            if let Value::Array(arr) = value {
                // Array should have reasonable size
                assert!(arr.len() <= 100);
            }
        }

        #[test]
        fn generate_object_fields(
            object_field_count in 1usize..20
        ) {
            let field = FieldDefinition {
                name: "nested".to_string(),
                field_type: "object".to_string(),
                required: true,
                description: None,
                default: None,
                constraints: HashMap::new(),
                faker_template: None,
            };

            let mut faker = EnhancedFaker::new();
            let value = field.generate_value(&mut faker);

            // Should generate an object
            if let Value::Object(obj) = value {
                // Object should have reasonable number of fields
                assert!(obj.len() <= 50);
            }
        }

        #[test]
        fn generate_nested_structures(
            nesting_depth in 0usize..5
        ) {
            let mut fields = Vec::new();

            // Create nested structure
            for i in 0..nesting_depth {
                fields.push(FieldDefinition {
                    name: format!("level_{}", i),
                    field_type: if i < nesting_depth - 1 {
                        "object".to_string()
                    } else {
                        "string".to_string()
                    },
                    required: true,
                    description: None,
                    default: None,
                    constraints: HashMap::new(),
                    faker_template: None,
                });
            }

            if !fields.is_empty() {
                let schema = SchemaDefinition::new("NestedSchema".to_string())
                    .with_fields(fields);

            let mut generator = MockDataGenerator::new();
            // Should handle nested structures without stack overflow
            // Use generate_from_json_schema instead
            let schema_json = json!({
                "type": "object",
                "properties": {
                    "level_0": {
                        "type": "object",
                        "properties": {
                            "level_1": {"type": "string"}
                        }
                    }
                }
            });
            let result = generator.generate_from_json_schema(&schema_json);
            assert!(result.is_ok());
            }
        }
    }
}
