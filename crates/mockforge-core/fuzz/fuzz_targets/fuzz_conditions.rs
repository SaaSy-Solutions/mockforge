#![no_main]

use libfuzzer_sys::fuzz_target;
use mockforge_core::conditions::{evaluate_condition, ConditionContext};
use serde_json::json;
use std::collections::HashMap;

fuzz_target!(|data: &[u8]| {
    // Try to use the fuzz input as a condition string
    if let Ok(condition_str) = std::str::from_utf8(data) {
        // Create a context with various data types
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Authorization".to_string(), "Bearer token123".to_string());

        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), "1".to_string());
        query_params.insert("limit".to_string(), "10".to_string());

        let request_body = json!({
            "id": 123,
            "name": "test",
            "active": true,
            "items": [1, 2, 3],
            "nested": {
                "field": "value"
            }
        });

        let response_body = json!({
            "status": "success",
            "data": {
                "result": "ok"
            }
        });

        let context = ConditionContext::new()
            .with_headers(headers)
            .with_query_params(query_params)
            .with_request_body(request_body)
            .with_response_body(response_body)
            .with_path("/api/test".to_string())
            .with_method("GET".to_string());

        // Attempt to evaluate the condition
        // Should never panic, even with malformed conditions
        let _ = evaluate_condition(condition_str, &context);
    }
});
