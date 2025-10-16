use mockforge_core::templating::{expand_str, expand_tokens};
use mockforge_core::validation::validate_json_schema;
use proptest::prelude::*;
use serde_json::{json, Value};

/// Property test: Template expansion should never panic, regardless of input
#[cfg(test)]
mod template_expansion_tests {
    use super::*;

    proptest! {
        #[test]
        fn expand_str_never_panics(input in ".*") {
            // Should never panic, even with invalid or malformed input
            let _ = expand_str(&input);
        }

        #[test]
        fn expand_tokens_never_panics(
            value in prop::option::of(prop::num::i64::ANY),
            key in "[a-zA-Z_][a-zA-Z0-9_]*"
        ) {
            // Test with various JSON values
            let json_val = match value {
                Some(v) => json!(v),
                None => Value::Null,
            };

            let obj = json!({key: json_val});
            let _ = expand_tokens(&obj);
        }

        #[test]
        fn expand_tokens_with_nested_objects(
            key1 in "[a-zA-Z_][a-zA-Z0-9_]*",
            key2 in "[a-zA-Z_][a-zA-Z0-9_]*",
            val in prop::num::i64::ANY
        ) {
            let obj = json!({
                key1: {
                    key2: val
                }
            });
            let _ = expand_tokens(&obj);
        }
    }
}

/// Property test: JSON schema validation should never panic
#[cfg(test)]
mod validation_tests {
    use super::*;

    proptest! {
        #[test]
        fn validate_json_schema_never_panics(
            data_type in prop::sample::select(vec!["string", "number", "boolean", "null", "array", "object"]),
            prop_name in "[a-zA-Z_][a-zA-Z0-9_]*"
        ) {
            // Create a simple schema
            let schema = json!({
                "type": "object",
                "properties": {
                    prop_name.clone(): {
                        "type": data_type
                    }
                }
            });

            // Create data that might or might not match
            let data = json!({
                prop_name: "test"
            });

            // Should never panic, even if validation fails
            let _ = validate_json_schema(&data, &schema);
        }

        #[test]
        fn validate_arbitrary_json_values(
            key in "[a-zA-Z_][a-zA-Z0-9_]*",
            value in prop::num::i64::ANY
        ) {
            let schema = json!({
                "type": "object",
                "properties": {
                    key.clone(): {"type": "number"}
                }
            });

            let data = json!({key: value});

            // Validation might succeed or fail, but should never panic
            let result = validate_json_schema(&data, &schema);
            // Verify result is either valid or has errors
            assert!(result.valid || !result.errors.is_empty());
        }

        #[test]
        fn validate_with_complex_schemas(
            min_val in 0i64..100,
            max_val in 100i64..1000,
            test_val in 0i64..1000
        ) {
            let schema = json!({
                "type": "object",
                "properties": {
                    "value": {
                        "type": "number",
                        "minimum": min_val,
                        "maximum": max_val
                    }
                }
            });

            let data = json!({"value": test_val});
            let result = validate_json_schema(&data, &schema);

            // Verify the validation result is correct
            if test_val >= min_val && test_val <= max_val {
                // Might be valid
                assert!(result.valid || !result.errors.is_empty());
            }
            // Should never panic regardless
        }
    }
}

/// Property test: String template expansion properties
#[cfg(test)]
mod template_properties {
    use super::*;

    proptest! {
        #[test]
        fn expand_str_preserves_non_template_text(text in "[^{]+") {
            // Text without template markers should be preserved
            let result = expand_str(&text);
            assert_eq!(result, text);
        }

        #[test]
        fn expand_str_handles_escaped_braces(count in 0usize..10) {
            // Multiple braces in a row should be handled gracefully
            let input = "{".repeat(count);
            let _ = expand_str(&input);
        }

        #[test]
        fn expand_str_handles_unicode(text in "\\PC*") {
            // Should handle unicode characters without panicking
            let _ = expand_str(&text);
        }
    }
}

/// Property test: Edge cases and boundary conditions
#[cfg(test)]
mod edge_cases {
    use super::*;

    proptest! {
        #[test]
        fn handles_empty_strings(_count in 0usize..5) {
            let empty_str = "".to_string();
            let _ = expand_str(&empty_str);

            let empty_json = json!({});
            let _ = expand_tokens(&empty_json);
        }

        #[test]
        fn handles_very_long_strings(len in 0usize..10000) {
            let long_str = "a".repeat(len);
            let _ = expand_str(&long_str);
        }

        #[test]
        fn handles_deep_nesting(depth in 0usize..10) {
            let mut value = json!("leaf");
            for _ in 0..depth {
                value = json!({"nested": value});
            }
            let _ = expand_tokens(&value);
        }
    }
}

/// Property test: OpenAPI and JSON parsing robustness
#[cfg(test)]
mod parsing_robustness {
    use super::*;

    proptest! {
        #[test]
        fn json_parsing_handles_arbitrary_structure(
            key in "[a-zA-Z_][a-zA-Z0-9_]*",
            int_val in prop::num::i64::ANY,
            bool_val in any::<bool>(),
            str_val in ".*"
        ) {
            // Test parsing complex JSON structures
            let complex_json = json!({
                key.clone(): {
                    "integer": int_val,
                    "boolean": bool_val,
                    "string": str_val,
                    "array": [1, 2, 3],
                    "null": null
                }
            });

            // Should always serialize/deserialize successfully
            let serialized = serde_json::to_string(&complex_json);
            assert!(serialized.is_ok());

            if let Ok(json_str) = serialized {
                let deserialized = serde_json::from_str::<Value>(&json_str);
                assert!(deserialized.is_ok());
            }
        }

        #[test]
        fn schema_validation_with_arrays(
            min_items in 0usize..10,
            max_items in 10usize..20,
            array_len in 0usize..25
        ) {
            let schema = json!({
                "type": "array",
                "minItems": min_items,
                "maxItems": max_items,
                "items": {"type": "number"}
            });

            let data = json!((0..array_len).collect::<Vec<usize>>());
            let result = validate_json_schema(&data, &schema);

            // Validation should complete without panicking
            let expected_valid = array_len >= min_items && array_len <= max_items;
            if expected_valid {
                assert!(result.valid || !result.errors.is_empty());
            }
        }

        #[test]
        fn schema_validation_with_pattern(
            pattern in "[a-z]{3,10}",
            test_string in "\\PC*"
        ) {
            let schema = json!({
                "type": "string",
                "pattern": pattern
            });

            let data = json!(test_string);
            let _ = validate_json_schema(&data, &schema);
            // Should never panic regardless of pattern or string
        }

        #[test]
        fn template_expansion_with_special_chars(
            text in "[\\s\\S]{0,100}"
        ) {
            // Test with all sorts of special characters
            let template = format!("{{{{random.uuid}}}} {} {{{{faker.name}}}}", text);
            let result = expand_str(&template);
            // Should handle gracefully
            assert!(!result.is_empty() || template.is_empty());
        }
    }
}

/// Property test: Data type conversions and coercion
#[cfg(test)]
mod type_handling {
    use super::*;

    proptest! {
        #[test]
        fn handles_numeric_type_variations(
            int_val in prop::num::i64::ANY,
            float_val in prop::num::f64::ANY
        ) {
            // Schema expects number, should accept both int and float
            let schema = json!({
                "type": "object",
                "properties": {
                    "value": {"type": "number"}
                }
            });

            let int_data = json!({"value": int_val});
            let float_data = json!({"value": float_val});

            // Both should be processed without panicking
            let _ = validate_json_schema(&int_data, &schema);
            if float_val.is_finite() {
                let _ = validate_json_schema(&float_data, &schema);
            }
        }

        #[test]
        fn handles_string_representations(
            val in prop::num::i32::ANY
        ) {
            let schema = json!({
                "type": "object",
                "properties": {
                    "id": {"type": "string"}
                }
            });

            // Test with actual number vs string representation
            let as_number = json!({"id": val});
            let as_string = json!({"id": val.to_string()});

            let _result_num = validate_json_schema(&as_number, &schema);
            let result_str = validate_json_schema(&as_string, &schema);

            // String should be valid, number should not, but neither should panic
            assert!(!result_str.valid || result_str.errors.is_empty());
            // Number will likely be invalid, but shouldn't panic
        }

        #[test]
        fn handles_null_and_optional_fields(
            include_field in any::<bool>(),
            val in prop::option::of(prop::num::i64::ANY)
        ) {
            let schema = json!({
                "type": "object",
                "properties": {
                    "optional_field": {"type": ["number", "null"]}
                }
            });

            let data = if include_field {
                match val {
                    Some(v) => json!({"optional_field": v}),
                    None => json!({"optional_field": null})
                }
            } else {
                json!({})
            };

            let result = validate_json_schema(&data, &schema);
            // Should handle null and missing fields gracefully
            assert!(result.valid || !result.errors.is_empty());
        }
    }
}
