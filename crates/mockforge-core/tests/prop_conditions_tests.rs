//! Property-based tests for condition evaluation system.
//!
//! These tests use property-based testing to verify correctness of condition
//! evaluation logic across a wide range of inputs, including JSONPath queries,
//! XPath queries, and logical operators.

use mockforge_core::conditions::{evaluate_condition, ConditionContext};
use proptest::prelude::*;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Property test: Condition evaluation should never panic
#[cfg(test)]
mod condition_evaluation_tests {
    use super::*;

    proptest! {
        #[test]
        fn evaluate_condition_never_panics(
            condition in ".*",
            has_body in any::<bool>(),
            body_val in prop::option::of(prop::num::i64::ANY)
        ) {
            // Create a context with optional body
            let mut context = ConditionContext::new();
            if has_body {
                if let Some(val) = body_val {
                    context = context.with_request_body(json!({"value": val}));
                }
            }

            // Should never panic, even with invalid conditions
            let _ = evaluate_condition(&condition, &context);
        }

        #[test]
        fn evaluate_empty_condition_always_true(
            headers in prop::collection::hash_map(".*", ".*", 0..10),
            query_params in prop::collection::hash_map(".*", ".*", 0..10)
        ) {
            let context = ConditionContext::new()
                .with_headers(headers)
                .with_query_params(query_params);

            // Empty condition should always evaluate to true
            let result = evaluate_condition("", &context);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), true);
        }

        #[test]
        fn evaluate_condition_with_headers(
            header_key in "[a-zA-Z0-9_-]+",
            header_val in ".*"
        ) {
            let mut headers = HashMap::new();
            headers.insert(header_key.clone(), header_val.clone());

            let context = ConditionContext::new()
                .with_headers(headers);

            // Test header-based conditions
            let condition1 = format!("headers.{} == {}", header_key, header_val);
            let condition2 = format!("headers.{} != different", header_key);

            // Should handle header-based conditions without panicking
            let _ = evaluate_condition(&condition1, &context);
            let _ = evaluate_condition(&condition2, &context);
        }

        #[test]
        fn evaluate_condition_with_query_params(
            param_key in "[a-zA-Z0-9_-]+",
            param_val in ".*"
        ) {
            let mut query_params = HashMap::new();
            query_params.insert(param_key.clone(), param_val.clone());

            let context = ConditionContext::new()
                .with_query_params(query_params);

            // Test query parameter conditions
            let condition1 = format!("query.{} == {}", param_key, param_val);
            let condition2 = format!("query.{} != different", param_key);

            // Should handle query parameter conditions without panicking
            let _ = evaluate_condition(&condition1, &context);
            let _ = evaluate_condition(&condition2, &context);
        }

        #[test]
        fn evaluate_condition_with_path(
            path in "/[a-zA-Z0-9/_-]*",
            method in prop::sample::select(vec!["GET", "POST", "PUT", "DELETE", "PATCH"])
        ) {
            let context = ConditionContext::new()
                .with_path(path.clone())
                .with_method(method.to_string());

            // Test path-based conditions
            let conditions = vec![
                format!("path == {}", path),
                format!("path != /different"),
                format!("method == {}", method),
            ];

            for condition in conditions {
                let _ = evaluate_condition(&condition, &context);
            }
        }
    }
}

/// Property test: JSONPath condition evaluation
#[cfg(test)]
mod jsonpath_tests {
    use super::*;

    proptest! {
        #[test]
        fn evaluate_jsonpath_with_arbitrary_body(
            jsonpath in prop::sample::select(vec![
                "$.id",
                "$.name",
                "$.value",
                "$.data.id",
                "$.items[0]",
                "$.items[*].id",
            ]),
            body_key in "[a-zA-Z_][a-zA-Z0-9_]*",
            body_val in prop::num::i64::ANY
        ) {
            let body = json!({
                body_key: body_val
            });

            let context = ConditionContext::new()
                .with_request_body(body);

            // JSONPath evaluation should not panic
            let _ = evaluate_condition(&jsonpath, &context);
        }

        #[test]
        fn evaluate_jsonpath_with_nested_objects(
            depth in 0usize..5,
            key in "[a-zA-Z_][a-zA-Z0-9_]*",
            value in prop::num::i64::ANY
        ) {
            // Build nested object
            let mut obj = json!(value);
            for _ in 0..depth {
                let mut wrapper = serde_json::Map::new();
                wrapper.insert(key.clone(), obj);
                obj = Value::Object(wrapper);
            }

            let context = ConditionContext::new()
                .with_request_body(obj);

            // Test various JSONPath queries
            let jsonpaths = vec![
                "$.".to_string() + &key,
                "$.*".to_string(),
                "$..*".to_string(),
            ];

            for jsonpath in jsonpaths {
                let _ = evaluate_condition(&jsonpath, &context);
            }
        }

        #[test]
        fn evaluate_jsonpath_with_arrays(
            array_len in 0usize..20,
            item_val in prop::num::i64::ANY
        ) {
            let array: Vec<Value> = (0..array_len)
                .map(|_| json!(item_val))
                .collect();

            let body = json!({
                "items": array
            });

            let context = ConditionContext::new()
                .with_request_body(body);

            // Test array JSONPath queries
            let jsonpaths = vec![
                "$.items[0]".to_string(),
                "$.items[*]".to_string(),
                "$.items.length".to_string(),
            ];

            for jsonpath in jsonpaths {
                let _ = evaluate_condition(&jsonpath, &context);
            }
        }
    }
}

/// Property test: Logical operator conditions
#[cfg(test)]
mod logical_operator_tests {
    use super::*;

    proptest! {
        #[test]
        fn evaluate_and_condition(
            condition1 in ".*",
            condition2 in ".*"
        ) {
            let context = ConditionContext::new();
            let and_condition = format!("AND({},{})", condition1, condition2);

            // AND conditions should not panic
            let _ = evaluate_condition(&and_condition, &context);
        }

        #[test]
        fn evaluate_or_condition(
            condition1 in ".*",
            condition2 in ".*"
        ) {
            let context = ConditionContext::new();
            let or_condition = format!("OR({},{})", condition1, condition2);

            // OR conditions should not panic
            let _ = evaluate_condition(&or_condition, &context);
        }

        #[test]
        fn evaluate_not_condition(
            condition in ".*"
        ) {
            let context = ConditionContext::new();
            let not_condition = format!("NOT({})", condition);

            // NOT conditions should not panic
            let _ = evaluate_condition(&not_condition, &context);
        }

        #[test]
        fn evaluate_nested_logical_operators(
            inner_condition in ".*"
        ) {
            let context = ConditionContext::new();

            // Test nested logical operators
            let nested = format!("AND(OR({},false),NOT(false))", inner_condition);
            let _ = evaluate_condition(&nested, &context);
        }

        #[test]
        fn evaluate_multiple_conditions(
            count in 2usize..10
        ) {
            let context = ConditionContext::new();

            // Generate multiple conditions
            let conditions: Vec<String> = (0..count)
                .map(|i| format!("condition{}", i))
                .collect();

            let and_condition = format!("AND({})", conditions.join(","));
            let _ = evaluate_condition(&and_condition, &context);
        }
    }
}

/// Property test: Simple comparison conditions
#[cfg(test)]
mod comparison_tests {
    use super::*;

    proptest! {
        #[test]
        fn evaluate_equality_condition(
            key in "[a-zA-Z0-9_-]+",
            value in ".*"
        ) {
            let mut headers = HashMap::new();
            headers.insert(key.clone(), value.clone());

            let context = ConditionContext::new()
                .with_headers(headers);

            let condition = format!("headers.{} == {}", key, value);
            let _ = evaluate_condition(&condition, &context);
        }

        #[test]
        fn evaluate_inequality_condition(
            key in "[a-zA-Z0-9_-]+",
            value1 in ".*",
            value2 in ".*"
        ) {
            let mut headers = HashMap::new();
            headers.insert(key.clone(), value1.clone());

            let context = ConditionContext::new()
                .with_headers(headers);

            // Test inequality with different value
            if value1 != value2 {
                let condition = format!("headers.{} != {}", key, value2);
                let _ = evaluate_condition(&condition, &context);
            }
        }

        #[test]
        fn evaluate_numeric_comparison(
            num_val in prop::num::i64::ANY
        ) {
            let body = json!({
                "value": num_val
            });

            let context = ConditionContext::new()
                .with_request_body(body);

            // Test numeric comparisons
            let conditions = vec![
                format!("$.value > {}", num_val - 1),
                format!("$.value < {}", num_val + 1),
                format!("$.value >= {}", num_val),
                format!("$.value <= {}", num_val),
            ];

            for condition in conditions {
                let _ = evaluate_condition(&condition, &context);
            }
        }
    }
}

/// Property test: Edge cases and boundary conditions
#[cfg(test)]
mod edge_cases {
    use super::*;

    proptest! {
        #[test]
        fn evaluate_with_empty_context(
            condition in ".*"
        ) {
            let context = ConditionContext::new();
            // Should handle empty context gracefully
            let _ = evaluate_condition(&condition, &context);
        }

        #[test]
        fn evaluate_with_very_long_condition(
            len in 0usize..10000
        ) {
            let condition = "a".repeat(len);
            let context = ConditionContext::new();
            let _ = evaluate_condition(&condition, &context);
        }

        #[test]
        fn evaluate_with_special_characters(
            special_chars in "[\\s\\S]{0,100}"
        ) {
            let condition = format!("headers.test == {}", special_chars);
            let mut headers = HashMap::new();
            headers.insert("test".to_string(), special_chars.clone());

            let context = ConditionContext::new()
                .with_headers(headers);

            let _ = evaluate_condition(&condition, &context);
        }

        #[test]
        fn evaluate_with_unicode(
            unicode_str in "\\PC*"
        ) {
            let mut headers = HashMap::new();
            headers.insert("test".to_string(), unicode_str.clone());

            let context = ConditionContext::new()
                .with_headers(headers);

            let condition = format!("headers.test == {}", unicode_str);
            let _ = evaluate_condition(&condition, &context);
        }

        #[test]
        fn evaluate_with_deeply_nested_json(
            depth in 0usize..10
        ) {
            // Build deeply nested JSON
            let mut value = json!("leaf");
            for i in 0..depth {
                let mut obj = serde_json::Map::new();
                obj.insert(format!("level{}", i), value);
                value = Value::Object(obj);
            }

            let context = ConditionContext::new()
                .with_request_body(value);

            // Test JSONPath on deeply nested structure
            let jsonpath = "$.level0";
            let _ = evaluate_condition(jsonpath, &context);
        }
    }
}
