//! Integration test using the actual billing_subscriptions_v1.json spec from issue #79
//!
//! This test verifies that k6 script generation works correctly with operation IDs
//! that contain dots (e.g., "plans.create", "subscriptions.create"), which was
//! the root cause of the "Unexpected token ." error.

use mockforge_bench::error::Result;
use mockforge_bench::k6_gen::{K6Config, K6ScriptGenerator};
use mockforge_bench::request_gen::RequestGenerator;
use mockforge_bench::scenarios::LoadScenario;
use mockforge_bench::spec_parser::SpecParser;
use std::collections::HashMap;
use std::path::PathBuf;

#[tokio::test]
async fn test_billing_subscriptions_spec_generation() {
    // Load the actual spec file from the issue
    let spec_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("billing_subscriptions_v1.json");

    assert!(spec_path.exists(), "Test fixture file should exist at: {}", spec_path.display());

    // Parse the OpenAPI spec
    let parser = SpecParser::from_file(&spec_path)
        .await
        .expect("Should parse the billing subscriptions spec");

    // Get all operations from the spec
    let operations = parser.get_operations();
    assert!(!operations.is_empty(), "Should find operations in the spec");

    // Verify we have operations with dots in their IDs (the problematic case)
    let operations_with_dots: Vec<_> = operations
        .iter()
        .filter(|op| op.operation_id.as_ref().map(|id| id.contains('.')).unwrap_or(false))
        .collect();

    assert!(
        !operations_with_dots.is_empty(),
        "Spec should contain operations with dots in operation IDs (e.g., 'plans.create')"
    );

    // Generate request templates
    let templates: Vec<_> = operations
        .iter()
        .map(RequestGenerator::generate_template)
        .collect::<Result<Vec<_>>>()
        .expect("Should generate request templates");

    // Verify that ALL operations have corresponding templates (no filtering)
    assert_eq!(
        templates.len(),
        operations.len(),
        "All operations should have corresponding templates - no filtering should occur"
    );

    // Create k6 config
    let config = K6Config {
        target_url: "https://api-m.sandbox.paypal.com".to_string(),
        base_path: None,
        scenario: LoadScenario::Constant,
        duration_secs: 30,
        max_vus: 50,
        threshold_percentile: "p(95)".to_string(),
        threshold_ms: 500,
        max_error_rate: 0.05,
        auth_header: None,
        custom_headers: HashMap::new(),
        skip_tls_verify: false,
        security_testing_enabled: false,
    };

    // Generate the k6 script
    let generator = K6ScriptGenerator::new(config, templates);
    let script = generator.generate().expect("Should generate k6 script without errors");

    // Verify the script doesn't contain invalid JavaScript identifiers with dots
    // Check for variable declarations that would cause "Unexpected token ." errors
    let lines: Vec<&str> = script.lines().collect();
    let mut invalid_variables = Vec::new();

    for (line_num, line) in lines.iter().enumerate() {
        // Look for const/let declarations that might have dots in variable names
        if line.trim().starts_with("const ") || line.trim().starts_with("let ") {
            // Extract the variable name (everything between const/let and =)
            if let Some(equals_pos) = line.find('=') {
                let var_decl = &line[..equals_pos];
                // Check if the variable name contains a dot (which would be invalid)
                if var_decl.contains('.') && !var_decl.contains("'") && !var_decl.contains("\"") {
                    // This is likely an invalid variable name (not a string literal)
                    invalid_variables.push((line_num + 1, line.to_string()));
                }
            }
        }

        // Also check for variable usage (e.g., "operation_name_latency.add")
        // Variable names with dots would cause syntax errors
        if line.contains("_latency.add") || line.contains("_errors.add") {
            // Extract the variable name before the method call
            if let Some(method_pos) = line.find(".add") {
                let var_usage = &line[..method_pos];
                // Check if it contains a dot (invalid identifier)
                if var_usage.contains('.') && !var_usage.contains("'") && !var_usage.contains("\"")
                {
                    invalid_variables.push((line_num + 1, line.to_string()));
                }
            }
        }
    }

    if !invalid_variables.is_empty() {
        panic!(
            "Found {} invalid variable declarations/usage with dots in variable names:\n{}",
            invalid_variables.len(),
            invalid_variables
                .iter()
                .map(|(num, line)| format!("  Line {}: {}", num, line))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    // Verify that ALL operations with special chars (dots/hyphens) appear in the script
    // with properly sanitized variable names
    let operations_with_special_chars: Vec<_> = operations
        .iter()
        .filter(|op| {
            op.operation_id
                .as_ref()
                .map(|id| id.contains('.') || id.contains('-'))
                .unwrap_or(false)
        })
        .collect();

    assert!(
        !operations_with_special_chars.is_empty(),
        "Should have operations with dots or hyphens in operation IDs"
    );

    // Verify that ALL operations with special chars appear in the script with sanitized names
    for op in &operations_with_special_chars {
        if let Some(op_id) = &op.operation_id {
            // Sanitize the operation ID (replace dots and hyphens with underscores)
            let sanitized = op_id
                .chars()
                .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
                .collect::<String>()
                .replace("__", "_") // Remove consecutive underscores
                .trim_matches('_')
                .to_string();

            // ALL operations should appear in the script with sanitized variable names
            assert!(
                script.contains(&format!("const {}_latency", sanitized))
                    || script.contains(&format!("const {}_errors", sanitized)),
                "Operation '{}' should appear in script with sanitized name '{}'",
                op_id,
                sanitized
            );
        }
    }

    // Verify that ALL original operation IDs still appear in comments/strings (for readability)
    // ALL operations should be included in the script, so ALL operation IDs should appear
    for op in &operations_with_special_chars {
        if let Some(op_id) = &op.operation_id {
            assert!(
                script.contains(op_id),
                "Original operation ID '{}' should appear in comments or strings",
                op_id
            );
        }
    }

    // Verify the script doesn't have the specific error pattern from issue #79:
    // variable names with dots like "const plans.create_latency" which would cause
    // "Unexpected token ." error when k6 tries to parse the JavaScript.
    // Note: String literals can contain dots (e.g., 'plans.create_latency'), but
    // variable identifiers cannot.

    // Check for invalid variable declarations (const/let with dots in identifier)
    let lines: Vec<&str> = script.lines().collect();
    let mut invalid_declarations = Vec::new();

    for (line_num, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        // Look for const/let declarations
        if trimmed.starts_with("const ") || trimmed.starts_with("let ") {
            // Extract the variable name part (before the = sign)
            if let Some(equals_pos) = trimmed.find('=') {
                let var_part = &trimmed[..equals_pos];
                // Check if it contains a dot and is NOT a string literal
                // Variable names with dots would look like "const plans.create_latency"
                if var_part.contains('.') && !var_part.contains("'") && !var_part.contains("\"") {
                    invalid_declarations.push((line_num + 1, line.to_string()));
                }
            }
        }

        // Also check for variable usage with dots (e.g., "plans.create_latency.add")
        if trimmed.contains(".add(") || trimmed.contains(".add ") {
            // Extract the part before .add
            if let Some(add_pos) = trimmed.find(".add") {
                let var_usage = &trimmed[..add_pos];
                // Check if it's a variable name with dots (not a string)
                if var_usage.contains('.')
                    && !var_usage.contains("'")
                    && !var_usage.contains("\"")
                    && !var_usage.trim().starts_with("//")
                {
                    invalid_declarations.push((line_num + 1, line.to_string()));
                }
            }
        }
    }

    if !invalid_declarations.is_empty() {
        panic!(
            "Found {} invalid variable declarations/usage with dots in identifiers:\n{}\n\nThis would cause 'Unexpected token .' error in k6.\n\nGenerated script:\n{}",
            invalid_declarations.len(),
            invalid_declarations
                .iter()
                .map(|(num, line)| format!("  Line {}: {}", num, line))
                .collect::<Vec<_>>()
                .join("\n"),
            script
        );
    }

    // Verify sanitized versions exist (these should be valid JavaScript identifiers)
    assert!(
        script.contains("plans_create") || script.contains("subscriptions_create"),
        "Script should contain sanitized operation names (plans_create or subscriptions_create)"
    );

    // Print a summary for debugging
    println!("\n✓ Successfully generated k6 script from billing_subscriptions_v1.json");
    println!("  - Found {} operations", operations.len());
    println!("  - Operations with dots in IDs: {}", operations_with_dots.len());
    println!("  - Script length: {} characters", script.len());
    println!("  - All variable names are properly sanitized (no dots in identifiers)");
}

/// Test that verifies insecureSkipTLSVerify is placed in global options, not per-request
/// This is critical because k6 only supports this as a global option.
/// See: https://grafana.com/docs/k6/latest/using-k6/k6-options/reference/#insecure-skip-tls-verify
#[tokio::test]
async fn test_insecure_skip_tls_verify_in_global_options() {
    let spec_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("billing_subscriptions_v1.json");

    let parser = SpecParser::from_file(&spec_path).await.expect("Should parse the spec");

    let operations = parser.get_operations();
    let templates: Vec<_> = operations
        .iter()
        .map(RequestGenerator::generate_template)
        .collect::<Result<Vec<_>>>()
        .expect("Should generate request templates");

    // Test with skip_tls_verify = true
    let config = K6Config {
        target_url: "https://192.168.1.100".to_string(),
        base_path: None,
        scenario: LoadScenario::Constant,
        duration_secs: 30,
        max_vus: 5,
        threshold_percentile: "p(95)".to_string(),
        threshold_ms: 500,
        max_error_rate: 0.05,
        auth_header: None,
        custom_headers: HashMap::new(),
        skip_tls_verify: true, // Enable insecure mode
        security_testing_enabled: false,
    };

    let generator = K6ScriptGenerator::new(config, templates);
    let script = generator.generate().expect("Should generate k6 script");

    // Verify insecureSkipTLSVerify is in the global options block
    // It should appear BEFORE "scenarios:" in the options object
    let options_start = script
        .find("export const options = {")
        .expect("Script should have options export");
    let scenarios_start = script.find("scenarios:").expect("Script should have scenarios");

    let options_prefix = &script[options_start..scenarios_start];
    assert!(
        options_prefix.contains("insecureSkipTLSVerify: true"),
        "insecureSkipTLSVerify should be in global options block BEFORE scenarios.\n\
         Options prefix:\n{}\n\nFull options block should look like:\n\
         export const options = {{\n  insecureSkipTLSVerify: true,\n  scenarios: ...",
        options_prefix
    );

    // Verify insecureSkipTLSVerify is NOT in individual request params
    // (It won't work there - k6 only supports it globally)
    let request_sections: Vec<&str> = script
        .split("const res = http.")
        .skip(1) // Skip first split (before first request)
        .collect();

    for (i, section) in request_sections.iter().enumerate() {
        // Get just the request line (up to the semicolon)
        if let Some(end) = section.find(';') {
            let request_line = &section[..end];
            assert!(
                !request_line.contains("insecureSkipTLSVerify"),
                "Request {} should NOT have insecureSkipTLSVerify in params (it's ignored there).\n\
                 Request: {}",
                i + 1,
                request_line
            );
        }
    }

    println!("\n✓ insecureSkipTLSVerify correctly placed in global options");
    println!("  - Verified it appears before 'scenarios:' in options block");
    println!("  - Verified it does NOT appear in individual request params");
}
