//! Integration test for CRUD flow params file integration
//!
//! This test verifies that the --params-file option is properly loaded and applied
//! when using --crud-flow mode. This was the root cause of the "ReferenceError: object
//! is not defined" error reported in issue #79.

use mockforge_bench::crud_flow::{CrudFlow, CrudFlowDetector, FlowStep};
use mockforge_bench::k6_gen::K6ScriptGenerator;
use mockforge_bench::param_overrides::{OperationOverrides, ParameterOverrides};
use mockforge_bench::spec_parser::SpecParser;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;

/// Helper function to generate CRUD flow script with params file
fn generate_crud_flow_script_with_params(
    flows: &[CrudFlow],
    param_overrides: Option<&ParameterOverrides>,
    skip_tls_verify: bool,
) -> String {
    let handlebars = handlebars::Handlebars::new();
    let template = include_str!("../src/templates/k6_crud_flow.hbs");

    let stages = vec![
        json!({"duration": "30s", "target": 5}),
        json!({"duration": "30s", "target": 5}),
    ];

    let headers_json = serde_json::to_string(&HashMap::<String, String>::new())
        .unwrap_or_else(|_| "{}".to_string());

    let data = json!({
        "base_url": "https://test.example.com",
        "flows": flows.iter().map(|f| {
            let sanitized_name = K6ScriptGenerator::sanitize_js_identifier(&f.name);
            json!({
                "name": sanitized_name.clone(),
                "display_name": f.name,
                "base_path": f.base_path,
                "steps": f.steps.iter().enumerate().map(|(idx, s)| {
                    let parts: Vec<&str> = s.operation.splitn(2, ' ').collect();
                    let method_raw = if !parts.is_empty() {
                        parts[0].to_uppercase()
                    } else {
                        "GET".to_string()
                    };
                    let method = if !parts.is_empty() {
                        let m = parts[0].to_lowercase();
                        if m == "delete" { "del".to_string() } else { m }
                    } else {
                        "get".to_string()
                    };
                    let path = if parts.len() >= 2 { parts[1] } else { "/" };
                    let is_get_or_head = method == "get" || method == "head";
                    let has_body = matches!(method.as_str(), "post" | "put" | "patch");

                    // Look up body from params file if available
                    let body_value = if has_body {
                        param_overrides
                            .map(|po| po.get_for_operation(None, &method_raw, path))
                            .and_then(|oo| oo.body)
                            .unwrap_or_else(|| json!({}))
                    } else {
                        json!({})
                    };

                    // Serialize body as JSON string for the template
                    let body_json_str = serde_json::to_string(&body_value)
                        .unwrap_or_else(|_| "{}".to_string());

                    json!({
                        "operation": s.operation,
                        "method": method,
                        "path": path,
                        "extract": s.extract,
                        "use_values": s.use_values,
                        "description": s.description,
                        "display_name": s.description.clone().unwrap_or_else(|| format!("Step {}", idx)),
                        "is_get_or_head": is_get_or_head,
                        "has_body": has_body,
                        "body": body_json_str,
                        "body_is_dynamic": false,
                    })
                }).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
        "extract_fields": vec!["id", "uuid"],
        "duration_secs": 60,
        "max_vus": 5,
        "skip_tls_verify": skip_tls_verify,
        "stages": stages,
        "threshold_percentile": "p(95)",
        "threshold_ms": 500,
        "max_error_rate": 0.05,
        "headers": headers_json,
        "dynamic_imports": Vec::<String>::new(),
        "dynamic_globals": Vec::<String>::new(),
    });

    handlebars
        .render_template(template, &data)
        .expect("Should render CRUD flow template")
}

#[test]
fn test_crud_flow_without_params_file_has_empty_body() {
    // Create a simple CRUD flow
    let flows = vec![CrudFlow {
        name: "virtualservice".to_string(),
        base_path: Some("/virtualservice".to_string()),
        steps: vec![
            FlowStep::new("POST /virtualservice".to_string())
                .with_description("Create virtualservice".to_string())
                .with_extract(vec!["uuid".to_string()]),
            FlowStep::new("GET /virtualservice/{uuid}".to_string())
                .with_description("Read virtualservice".to_string())
                .with_values([("uuid".to_string(), "uuid".to_string())].into()),
        ],
    }];

    // Generate script WITHOUT params file
    let script = generate_crud_flow_script_with_params(&flows, None, false);

    // Verify the POST request has an empty body (the default)
    // Note: Uses 'let' instead of 'const' to allow potential reassignment for security testing
    assert!(
        script.contains("let payload = {};"),
        "POST request should have empty body when no params file is provided.\nScript:\n{}",
        script
    );

    // Verify the script is valid JavaScript (no "[object Object]" rendering)
    assert!(
        !script.contains("[object Object]"),
        "Script should not contain '[object Object]' (improper serialization).\nScript:\n{}",
        script
    );

    println!("✓ CRUD flow without params file correctly uses empty body");
}

#[test]
fn test_crud_flow_with_params_file_applies_body() {
    // Create a simple CRUD flow
    let flows = vec![CrudFlow {
        name: "virtualservice".to_string(),
        base_path: Some("/virtualservice".to_string()),
        steps: vec![
            FlowStep::new("POST /virtualservice".to_string())
                .with_description("Create virtualservice".to_string())
                .with_extract(vec!["uuid".to_string()]),
            FlowStep::new("PUT /virtualservice/{uuid}".to_string())
                .with_description("Update virtualservice".to_string())
                .with_values([("uuid".to_string(), "uuid".to_string())].into()),
            FlowStep::new("GET /virtualservice/{uuid}".to_string())
                .with_description("Read virtualservice".to_string())
                .with_values([("uuid".to_string(), "uuid".to_string())].into()),
        ],
    }];

    // Create params file with body configurations
    let mut operations = HashMap::new();
    operations.insert(
        "POST /virtualservice".to_string(),
        OperationOverrides {
            body: Some(json!({
                "name": "test-vs",
                "pool_ref": "/api/pool/pool-uuid",
                "services": [{"port": 80}]
            })),
            ..Default::default()
        },
    );
    operations.insert(
        "PUT /virtualservice/{uuid}".to_string(),
        OperationOverrides {
            body: Some(json!({
                "name": "updated-vs",
                "enabled": true
            })),
            ..Default::default()
        },
    );

    let param_overrides = ParameterOverrides {
        defaults: OperationOverrides::default(),
        operations,
    };

    // Generate script WITH params file
    let script = generate_crud_flow_script_with_params(&flows, Some(&param_overrides), false);

    // Verify the POST request has the configured body
    assert!(
        script.contains("\"name\":\"test-vs\""),
        "POST request should have the body from params file.\nScript:\n{}",
        script
    );
    assert!(
        script.contains("\"pool_ref\":\"/api/pool/pool-uuid\""),
        "POST request should have pool_ref from params file.\nScript:\n{}",
        script
    );
    assert!(
        script.contains("\"services\":[{\"port\":80}]"),
        "POST request should have services array from params file.\nScript:\n{}",
        script
    );

    // Verify the PUT request has its configured body
    assert!(
        script.contains("\"name\":\"updated-vs\""),
        "PUT request should have the body from params file.\nScript:\n{}",
        script
    );
    assert!(
        script.contains("\"enabled\":true"),
        "PUT request should have enabled flag from params file.\nScript:\n{}",
        script
    );

    // Verify the script is valid JavaScript (no "[object Object]" rendering)
    assert!(
        !script.contains("[object Object]"),
        "Script should not contain '[object Object]' (improper serialization).\nScript:\n{}",
        script
    );

    // Verify the script doesn't have "object is not defined" issue
    // This was the original bug - the body was rendered as just "object"
    assert!(
        !script.contains("= object"),
        "Script should not contain '= object' (improper variable reference).\nScript:\n{}",
        script
    );

    println!("✓ CRUD flow with params file correctly applies body configurations");
    println!("  - POST body: name=test-vs, pool_ref, services");
    println!("  - PUT body: name=updated-vs, enabled=true");
}

#[test]
fn test_crud_flow_body_serialization_is_valid_javascript() {
    let flows = vec![CrudFlow {
        name: "test".to_string(),
        base_path: Some("/test".to_string()),
        steps: vec![
            FlowStep::new("POST /test".to_string()).with_description("Create test".to_string())
        ],
    }];

    // Create params with complex nested body
    let mut operations = HashMap::new();
    operations.insert(
        "POST /test".to_string(),
        OperationOverrides {
            body: Some(json!({
                "string_field": "hello world",
                "number_field": 42,
                "boolean_field": true,
                "null_field": null,
                "array_field": [1, 2, 3],
                "nested_object": {
                    "inner_string": "nested value",
                    "inner_array": ["a", "b", "c"]
                }
            })),
            ..Default::default()
        },
    );

    let param_overrides = ParameterOverrides {
        defaults: OperationOverrides::default(),
        operations,
    };

    let script = generate_crud_flow_script_with_params(&flows, Some(&param_overrides), false);

    // Verify all field types are properly serialized
    assert!(script.contains("\"string_field\":\"hello world\""));
    assert!(script.contains("\"number_field\":42"));
    assert!(script.contains("\"boolean_field\":true"));
    assert!(script.contains("\"null_field\":null"));
    assert!(script.contains("\"array_field\":[1,2,3]"));
    assert!(script.contains("\"nested_object\":{"));
    assert!(script.contains("\"inner_string\":\"nested value\""));

    // Verify the body is assigned to a variable properly (JSON field order may vary)
    // Note: Uses 'let' instead of 'const' to allow potential reassignment for security testing
    assert!(
        script.contains("let payload = {") && script.contains("\"string_field\":\"hello world\""),
        "Body should be assigned to let payload as valid JSON.\nScript:\n{}",
        script
    );

    println!("✓ Complex nested body is correctly serialized as valid JavaScript");
}

#[test]
fn test_crud_flow_get_request_has_no_body() {
    let flows = vec![CrudFlow {
        name: "test".to_string(),
        base_path: Some("/test".to_string()),
        steps: vec![
            FlowStep::new("GET /test".to_string()).with_description("List tests".to_string()),
            FlowStep::new("GET /test/{id}".to_string())
                .with_description("Get test".to_string())
                .with_values([("id".to_string(), "id".to_string())].into()),
        ],
    }];

    // Even with params file, GET requests shouldn't have body
    let mut operations = HashMap::new();
    operations.insert(
        "GET /test".to_string(),
        OperationOverrides {
            body: Some(json!({"should": "be ignored"})),
            ..Default::default()
        },
    );

    let param_overrides = ParameterOverrides {
        defaults: OperationOverrides::default(),
        operations,
    };

    let script = generate_crud_flow_script_with_params(&flows, Some(&param_overrides), false);

    // GET requests should use http.get with just headers (and jar: null for cookie jar disable), no body
    // Count occurrences of the GET pattern without body
    let get_without_body =
        script.matches("http.get(`${BASE_URL}${path}`, { headers, jar: null })").count();

    assert_eq!(
        get_without_body, 2,
        "Both GET requests should use http.get without body.\nScript:\n{}",
        script
    );

    // Verify GET body config is NOT in the script
    assert!(
        !script.contains("should"),
        "GET request body from params should be ignored.\nScript:\n{}",
        script
    );

    println!("✓ GET requests correctly have no body (params file body ignored)");
}

#[tokio::test]
async fn test_crud_flow_detection_and_params_integration() {
    // Load a real spec file
    let spec_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("billing_subscriptions_v1.json");

    if !spec_path.exists() {
        println!("Skipping test - fixture file not found");
        return;
    }

    let parser = SpecParser::from_file(&spec_path).await.expect("Should parse spec");

    let operations = parser.get_operations();
    let flows = CrudFlowDetector::detect_flows(&operations);

    if flows.is_empty() {
        println!("Skipping test - no CRUD flows detected in fixture");
        return;
    }

    // Create params for detected flows
    let mut op_overrides = HashMap::new();
    for flow in &flows {
        for step in &flow.steps {
            let parts: Vec<&str> = step.operation.splitn(2, ' ').collect();
            if parts.len() >= 2 {
                let method = parts[0].to_uppercase();
                if method == "POST" || method == "PUT" || method == "PATCH" {
                    op_overrides.insert(
                        step.operation.clone(),
                        OperationOverrides {
                            body: Some(json!({"test_field": "test_value"})),
                            ..Default::default()
                        },
                    );
                }
            }
        }
    }

    let param_overrides = ParameterOverrides {
        defaults: OperationOverrides::default(),
        operations: op_overrides,
    };

    let script = generate_crud_flow_script_with_params(&flows, Some(&param_overrides), true);

    // Verify the script is valid
    assert!(!script.contains("[object Object]"));
    assert!(!script.contains("= object"));
    assert!(script.contains("insecureSkipTLSVerify: true"));

    // If there were any POST/PUT/PATCH operations, verify body was applied
    if script.contains("http.post") || script.contains("http.put") || script.contains("http.patch")
    {
        assert!(
            script.contains("test_field"),
            "Params file body should be applied to POST/PUT/PATCH requests.\nScript:\n{}",
            script
        );
    }

    println!("✓ CRUD flow detection with params integration works correctly");
    println!("  - Detected {} flows", flows.len());
}
