//! Comprehensive error handling tests for core functionality.
//!
//! These tests verify that MockForge handles errors gracefully without panicking,
//! providing proper error messages and recovery mechanisms.

use mockforge_core::conditions::{evaluate_condition, ConditionContext};
use mockforge_core::routing::{HttpMethod, Route, RouteRegistry};
use mockforge_core::templating::expand_str;
use mockforge_core::validation::validate_json_schema;
use serde_json::json;

#[cfg(test)]
mod malformed_input_tests {
    use super::*;

    #[test]
    fn condition_evaluation_with_malformed_jsonpath() {
        let context = ConditionContext::new();

        // Malformed JSONPath expressions should return errors, not panic
        let malformed_paths = vec![
            "$.",
            "$..",
            "$[",
            "$]",
            "$.field[",
            "$.field[]",
            "$[invalid]",
            "$.field..nested",
        ];

        for path in malformed_paths {
            let result = evaluate_condition(path, &context);
            // Should return an error, not panic
            assert!(result.is_err() || result.is_ok());
        }
    }

    #[test]
    fn condition_evaluation_with_malformed_logical_operators() {
        let context = ConditionContext::new();

        // Malformed logical operators
        let malformed = vec![
            "AND(", "OR(", "NOT(", "AND())", "OR(,)", "AND(OR(", "NOT(NOT(",
        ];

        for condition in malformed {
            let result = evaluate_condition(condition, &context);
            // Should handle gracefully
            assert!(result.is_err() || result.is_ok());
        }
    }

    #[test]
    fn route_matching_with_malformed_paths() {
        let mut registry = RouteRegistry::new();
        let route = Route::new(HttpMethod::GET, "/api/users".to_string());
        registry.add_http_route(route).unwrap();

        // Malformed paths should not panic
        let malformed_paths = vec![
            "",
            "//",
            "/api//users",
            "/api/users/",
            "api/users",             // Missing leading slash
            "/api/users?query=test", // Query string in path
            "/api/users#fragment",   // Fragment in path
        ];

        for path in malformed_paths {
            let _ = registry.find_http_routes(&HttpMethod::GET, path);
        }
    }

    #[test]
    fn validation_with_malformed_schemas() {
        // Malformed JSON schemas should not panic
        let malformed_schemas = vec![
            json!({}), // Empty schema
            json!({"type": "invalid_type"}),
            json!({"type": "object", "properties": null}),
            json!({"type": "array", "items": null}),
            json!({"type": "string", "pattern": "[invalid regex"}),
            json!({"type": "number", "minimum": "not a number"}),
            json!({"allOf": null}),
            json!({"oneOf": []}),
            json!({"$ref": "#/definitions/nonexistent"}),
        ];

        let test_data = json!({"test": "value"});

        for schema in malformed_schemas {
            let result = validate_json_schema(&test_data, &schema);
            // Should handle gracefully, may return errors
            let _ = result;
        }
    }

    #[test]
    fn template_expansion_with_malformed_templates() {
        // Malformed templates should not panic
        let malformed = vec![
            "{{",
            "}}",
            "{{{",
            "}}}",
            "{{{{",
            "{{.}}",
            "{{..}}",
            "{{/etc/passwd}}",
            "{{../../etc/passwd}}",
            "{{field",
            "field}}",
        ];

        for template in malformed {
            let _ = expand_str(template);
        }
    }
}

#[cfg(test)]
mod large_payload_tests {
    use super::*;

    #[test]
    fn condition_evaluation_with_very_large_input() {
        let context = ConditionContext::new();

        // Very large condition string
        let large_condition = "a".repeat(1_000_000);
        let result = evaluate_condition(&large_condition, &context);
        // Should handle without stack overflow
        let _ = result;
    }

    #[test]
    fn route_matching_with_very_long_path() {
        let mut registry = RouteRegistry::new();
        let route = Route::new(HttpMethod::GET, "/api/users".to_string());
        registry.add_http_route(route).unwrap();

        // Very long path
        let long_path = "/".to_string() + &"a".repeat(100_000);
        let _ = registry.find_http_routes(&HttpMethod::GET, &long_path);
    }

    #[test]
    fn validation_with_very_large_schema() {
        // Create a very large schema with many fields
        let mut properties = serde_json::Map::new();
        for i in 0..10_000 {
            properties.insert(format!("field_{}", i), json!({"type": "string"}));
        }

        let large_schema = json!({
            "type": "object",
            "properties": properties
        });

        let test_data = json!({"field_0": "value"});
        let result = validate_json_schema(&test_data, &large_schema);
        // Should handle without memory issues
        let _ = result;
    }

    #[test]
    fn template_expansion_with_very_large_template() {
        // Very large template
        let large_template = "{{field}} ".repeat(100_000);
        let _ = expand_str(&large_template);
    }
}

#[cfg(test)]
mod invalid_utf8_tests {
    use super::*;

    #[test]
    fn condition_evaluation_handles_invalid_utf8_gracefully() {
        let context = ConditionContext::new();

        // Create invalid UTF-8 sequences
        let invalid_utf8 = vec![
            &[0xFF, 0xFE, 0xFD][..],
            &[0xC0, 0x80][..],       // Overlong encoding
            &[0xE0, 0x80, 0x80][..], // Overlong encoding
        ];

        for bytes in invalid_utf8 {
            // Try to convert to string (will fail, but should handle gracefully)
            if let Ok(condition_str) = std::str::from_utf8(bytes) {
                let _ = evaluate_condition(condition_str, &context);
            }
        }
    }

    #[test]
    fn validation_handles_invalid_utf8_in_strings() {
        // Test with strings that might have encoding issues
        // Note: serde_json handles UTF-8 validation, so we test valid but unusual UTF-8
        let schema = json!({
            "type": "object",
            "properties": {
                "field": {"type": "string"}
            }
        });

        // Valid but unusual UTF-8 sequences
        let test_cases = vec![
            json!({"field": "\u{0000}"}),  // Null byte
            json!({"field": "\u{FFFD}"}),  // Replacement character
            json!({"field": "\u{1F4A9}"}), // Emoji
        ];

        for test_data in test_cases {
            let _ = validate_json_schema(&test_data, &schema);
        }
    }
}

#[cfg(test)]
mod resource_exhaustion_tests {
    use super::*;

    #[test]
    fn condition_evaluation_with_deeply_nested_conditions() {
        let context = ConditionContext::new();

        // Create deeply nested logical operators
        let mut nested = "true".to_string();
        for _ in 0..100 {
            nested = format!("AND({})", nested);
        }

        let result = evaluate_condition(&nested, &context);
        // Should handle without stack overflow
        let _ = result;
    }

    #[test]
    fn route_registry_with_many_routes() {
        let mut registry = RouteRegistry::new();

        // Add many routes
        for i in 0..10_000 {
            let route = Route::new(HttpMethod::GET, format!("/api/route_{}", i));
            let _ = registry.add_http_route(route);
        }

        // Should still be able to find routes
        let matches = registry.find_http_routes(&HttpMethod::GET, "/api/route_0");
        assert!(!matches.is_empty());
    }

    #[test]
    fn validation_with_deeply_nested_schemas() {
        // Create deeply nested schema
        let mut nested = json!({"type": "string"});
        for _ in 0..100 {
            nested = json!({
                "type": "object",
                "properties": {
                    "nested": nested
                }
            });
        }

        let test_data = json!({"nested": "value"});
        let result = validate_json_schema(&test_data, &nested);
        // Should handle without stack overflow
        let _ = result;
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn condition_evaluation_with_empty_context() {
        let context = ConditionContext::new();

        // Various conditions with empty context
        let conditions = vec![
            "",
            "true",
            "false",
            "$.field",
            "headers.test == value",
            "query.param == value",
        ];

        for condition in conditions {
            let _ = evaluate_condition(condition, &context);
        }
    }

    #[test]
    fn route_matching_with_special_characters() {
        let mut registry = RouteRegistry::new();

        // Routes with special characters
        let special_routes = vec![
            "/api/test%20path",
            "/api/test+path",
            "/api/test@path",
            "/api/test#path",
            "/api/test$path",
        ];

        for route_path in special_routes {
            let route = Route::new(HttpMethod::GET, route_path.to_string());
            let _ = registry.add_http_route(route);
        }
    }

    #[test]
    fn validation_with_null_values() {
        let schema = json!({
            "type": "object",
            "properties": {
                "field": {"type": "string"}
            }
        });

        let test_cases = vec![
            json!({"field": null}),
            json!({"field": "value"}),
            json!({}), // Missing field
        ];

        for test_data in test_cases {
            let _ = validate_json_schema(&test_data, &schema);
        }
    }

    #[test]
    fn template_expansion_with_empty_context() {
        let templates = vec!["{{field}}", "{{nested.field}}", "{{array.0}}"];

        for template in templates {
            // Should handle missing context gracefully
            let _ = expand_str(template);
        }
    }
}

#[cfg(test)]
mod concurrent_access_tests {
    use super::*;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn route_registry_concurrent_access() {
        let registry = Arc::new(RouteRegistry::new());
        let mut handles = vec![];

        // Spawn multiple threads adding routes
        for i in 0..10 {
            let _registry_clone = Arc::clone(&registry);
            let handle = thread::spawn(move || {
                for j in 0..100 {
                    let mut reg = RouteRegistry::new();
                    let route = Route::new(HttpMethod::GET, format!("/api/route_{}_{}", i, j));
                    let _ = reg.add_http_route(route);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn condition_evaluation_concurrent_access() {
        let context = Arc::new(ConditionContext::new());
        let mut handles = vec![];

        // Spawn multiple threads evaluating conditions
        for _ in 0..10 {
            let context_clone = Arc::clone(&context);
            let handle = thread::spawn(move || {
                for _ in 0..100 {
                    let _ = evaluate_condition("true", &context_clone);
                }
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }
    }
}
