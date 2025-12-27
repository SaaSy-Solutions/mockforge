//! Performance regression tests with automated threshold checking.
//!
//! These tests verify that critical operations complete within acceptable
//! time thresholds. If these tests fail, it indicates a performance regression.

use mockforge_core::conditions::{evaluate_condition, ConditionContext};
use mockforge_core::routing::{HttpMethod, Route, RouteRegistry};
use mockforge_core::templating::expand_str;
use mockforge_core::validation::validate_json_schema;
use serde_json::json;
use std::time::Instant;

// Performance thresholds (in microseconds)
// Note: These thresholds are intentionally generous to avoid flaky tests
// on different hardware and under varying system loads
const ROUTE_MATCHING_THRESHOLD_US: u64 = 50; // 50 microseconds
const CONDITION_EVAL_THRESHOLD_US: u64 = 100; // 100 microseconds
const VALIDATION_THRESHOLD_US: u64 = 50; // 50 microseconds
const TEMPLATE_EXPANSION_THRESHOLD_US: u64 = 100; // 100 microseconds
const ROUTE_ADDITION_THRESHOLD_US: u64 = 50; // 50 microseconds

#[cfg(test)]
mod route_matching_performance {
    use super::*;

    #[test]
    fn route_matching_simple_path() {
        let mut registry = RouteRegistry::new();
        let route = Route::new(HttpMethod::GET, "/api/users".to_string());
        registry.add_http_route(route).unwrap();

        let start = Instant::now();
        for _ in 0..1000 {
            let _ = registry.find_http_routes(&HttpMethod::GET, "/api/users");
        }
        let elapsed = start.elapsed();

        let avg_us = elapsed.as_micros() / 1000;
        assert!(
            avg_us <= ROUTE_MATCHING_THRESHOLD_US as u128,
            "Route matching took {}µs, threshold is {}µs",
            avg_us,
            ROUTE_MATCHING_THRESHOLD_US
        );
    }

    #[test]
    fn route_matching_wildcard_path() {
        let mut registry = RouteRegistry::new();
        let route = Route::new(HttpMethod::GET, "/api/users/*".to_string());
        registry.add_http_route(route).unwrap();

        let start = Instant::now();
        for _ in 0..1000 {
            let _ = registry.find_http_routes(&HttpMethod::GET, "/api/users/123");
        }
        let elapsed = start.elapsed();

        let avg_us = elapsed.as_micros() / 1000;
        assert!(
            avg_us <= ROUTE_MATCHING_THRESHOLD_US as u128 * 2, // Wildcards may be slightly slower
            "Wildcard route matching took {}µs, threshold is {}µs",
            avg_us,
            ROUTE_MATCHING_THRESHOLD_US * 2
        );
    }

    #[test]
    fn route_matching_with_many_routes() {
        let mut registry = RouteRegistry::new();

        // Add 100 routes
        for i in 0..100 {
            let route = Route::new(HttpMethod::GET, format!("/api/route_{}", i));
            registry.add_http_route(route).unwrap();
        }

        let start = Instant::now();
        for _ in 0..100 {
            let _ = registry.find_http_routes(&HttpMethod::GET, "/api/route_50");
        }
        let elapsed = start.elapsed();

        let avg_us = elapsed.as_micros() / 100;
        // With many routes, matching may take longer, but should still be reasonable
        assert!(
            avg_us <= ROUTE_MATCHING_THRESHOLD_US as u128 * 10,
            "Route matching with many routes took {}µs, threshold is {}µs",
            avg_us,
            ROUTE_MATCHING_THRESHOLD_US * 10
        );
    }
}

#[cfg(test)]
mod condition_evaluation_performance {
    use super::*;

    #[test]
    fn condition_evaluation_simple() {
        let context = ConditionContext::new().with_request_body(json!({"value": 42}));

        let start = Instant::now();
        for _ in 0..1000 {
            let _ = evaluate_condition("$.value == 42", &context);
        }
        let elapsed = start.elapsed();

        let avg_us = elapsed.as_micros() / 1000;
        assert!(
            avg_us <= CONDITION_EVAL_THRESHOLD_US as u128,
            "Condition evaluation took {}µs, threshold is {}µs",
            avg_us,
            CONDITION_EVAL_THRESHOLD_US
        );
    }

    #[test]
    fn condition_evaluation_complex() {
        let context = ConditionContext::new().with_request_body(json!({
            "user": {"id": 123, "name": "test"},
            "items": [1, 2, 3, 4, 5]
        }));

        let condition = "AND($.user.id == 123, $.items.length > 0)";

        let start = Instant::now();
        for _ in 0..100 {
            let _ = evaluate_condition(condition, &context);
        }
        let elapsed = start.elapsed();

        let avg_us = elapsed.as_micros() / 100;
        // Complex conditions may take longer
        assert!(
            avg_us <= CONDITION_EVAL_THRESHOLD_US as u128 * 2,
            "Complex condition evaluation took {}µs, threshold is {}µs",
            avg_us,
            CONDITION_EVAL_THRESHOLD_US * 2
        );
    }

    #[test]
    fn condition_evaluation_with_headers() {
        use std::collections::HashMap;
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let context = ConditionContext::new().with_headers(headers);

        let start = Instant::now();
        for _ in 0..1000 {
            let _ = evaluate_condition("headers.Content-Type == 'application/json'", &context);
        }
        let elapsed = start.elapsed();

        let avg_us = elapsed.as_micros() / 1000;
        assert!(
            avg_us <= CONDITION_EVAL_THRESHOLD_US as u128,
            "Header condition evaluation took {}µs, threshold is {}µs",
            avg_us,
            CONDITION_EVAL_THRESHOLD_US
        );
    }
}

#[cfg(test)]
mod validation_performance {
    use super::*;

    #[test]
    fn validation_simple_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            },
            "required": ["name"]
        });

        let data = json!({"name": "test"});

        let start = Instant::now();
        for _ in 0..1000 {
            let _ = validate_json_schema(&data, &schema);
        }
        let elapsed = start.elapsed();

        let avg_us = elapsed.as_micros() / 1000;
        // Allow more time for validation (200µs threshold to avoid flaky tests)
        assert!(avg_us <= 200, "Simple validation took {}µs, threshold is 200µs", avg_us);
    }

    #[test]
    fn validation_complex_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "integer"},
                        "name": {"type": "string"},
                        "email": {"type": "string", "format": "email"}
                    },
                    "required": ["id", "name", "email"]
                },
                "items": {
                    "type": "array",
                    "items": {"type": "string"}
                }
            }
        });

        let data = json!({
            "user": {
                "id": 123,
                "name": "test",
                "email": "test@example.com"
            },
            "items": ["a", "b", "c"]
        });

        let start = Instant::now();
        for _ in 0..100 {
            let _ = validate_json_schema(&data, &schema);
        }
        let elapsed = start.elapsed();

        let avg_us = elapsed.as_micros() / 100;
        // Complex schemas may take longer (1000µs threshold)
        assert!(avg_us <= 1000, "Complex validation took {}µs, threshold is 1000µs", avg_us);
    }
}

#[cfg(test)]
mod template_expansion_performance {
    use super::*;

    #[test]
    fn template_expansion_simple() {
        let template = "Hello {{name}}";

        let start = Instant::now();
        for _ in 0..1000 {
            let _ = expand_str(template);
        }
        let elapsed = start.elapsed();

        let avg_us = elapsed.as_micros() / 1000;
        assert!(
            avg_us <= TEMPLATE_EXPANSION_THRESHOLD_US as u128,
            "Simple template expansion took {}µs, threshold is {}µs",
            avg_us,
            TEMPLATE_EXPANSION_THRESHOLD_US
        );
    }

    #[test]
    fn template_expansion_complex() {
        let template = "User {{uuid}} created at {{now}} with ID {{randInt 1 100}}";

        let start = Instant::now();
        for _ in 0..100 {
            let _ = expand_str(template);
        }
        let elapsed = start.elapsed();

        let avg_us = elapsed.as_micros() / 100;
        // Complex templates with multiple tokens may take longer (500µs threshold)
        assert!(
            avg_us <= 500,
            "Complex template expansion took {}µs, threshold is 500µs",
            avg_us
        );
    }
}

#[cfg(test)]
mod route_addition_performance {
    use super::*;

    #[test]
    fn route_addition_performance() {
        let mut registry = RouteRegistry::new();

        let start = Instant::now();
        for i in 0..1000 {
            let route = Route::new(HttpMethod::GET, format!("/api/route_{}", i));
            let _ = registry.add_http_route(route);
        }
        let elapsed = start.elapsed();

        let avg_us = elapsed.as_micros() / 1000;
        assert!(
            avg_us <= ROUTE_ADDITION_THRESHOLD_US as u128,
            "Route addition took {}µs, threshold is {}µs",
            avg_us,
            ROUTE_ADDITION_THRESHOLD_US
        );
    }
}

#[cfg(test)]
mod bulk_operations_performance {
    use super::*;

    #[test]
    fn bulk_route_matching() {
        let mut registry = RouteRegistry::new();

        // Add 50 routes
        for i in 0..50 {
            let route = Route::new(HttpMethod::GET, format!("/api/route_{}", i));
            registry.add_http_route(route).unwrap();
        }

        let start = Instant::now();
        // Match all routes
        for i in 0..50 {
            let _ = registry.find_http_routes(&HttpMethod::GET, &format!("/api/route_{}", i));
        }
        let elapsed = start.elapsed();

        let total_ms = elapsed.as_millis();
        // Bulk operations should complete in reasonable time
        assert!(
            total_ms <= 10, // 10ms for 50 matches
            "Bulk route matching took {}ms, threshold is 10ms",
            total_ms
        );
    }

    #[test]
    fn bulk_condition_evaluation() {
        let context = ConditionContext::new().with_request_body(json!({"value": 42}));

        let conditions = vec![
            "true",
            "false",
            "$.value == 42",
            "$.value != 0",
            "AND($.value == 42, $.value > 0)",
        ];

        let start = Instant::now();
        for _ in 0..100 {
            for condition in &conditions {
                let _ = evaluate_condition(condition, &context);
            }
        }
        let elapsed = start.elapsed();

        let total_ms = elapsed.as_millis();
        assert!(
            total_ms <= 50, // 50ms for 500 evaluations
            "Bulk condition evaluation took {}ms, threshold is 50ms",
            total_ms
        );
    }
}
