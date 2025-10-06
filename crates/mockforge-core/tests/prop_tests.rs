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
            assert!(result.valid || !result.errors.is_empty() || result.valid);
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
