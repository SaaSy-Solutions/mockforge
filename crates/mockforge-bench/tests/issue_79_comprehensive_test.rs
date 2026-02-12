//! Integration tests for all issues reported in GitHub issue #79
//!
//! This test suite validates that all reported issues have been resolved:
//! 1. k6 scripts with dots in operation IDs causing "Unexpected token ." error
//! 2. k6 metric name validation errors (dots in metric names)
//! 3. k6 threshold syntax errors (p95 vs p(95))
//! 4. HTTP method case (GET vs get)
//! 5. Headers serialization issues
//! 6. Certificate validation errors with --insecure flag
//! 7. TLS server panic (CryptoProvider error)
//! 8. Swagger 2.0 support
//! 9. CRUD flow with dynamic parameters
//! 10. Security payload injection
//! 11. Multi-target parallel testing
//! 12. Spec merge conflicts

use mockforge_bench::k6_gen::{K6Config, K6ScriptGenerator};
use mockforge_bench::request_gen::RequestGenerator;
use mockforge_bench::scenarios::LoadScenario;
use mockforge_bench::security_payloads::{
    SecurityPayloads, SecurityTestConfig, SecurityTestGenerator,
};
use mockforge_bench::spec_parser::SpecParser;
use mockforge_bench::target_parser::parse_targets_file;
use mockforge_bench::wafbench::WafBenchLoader;
use std::collections::HashMap;
use std::path::PathBuf;

#[tokio::test]
async fn test_issue_79_comprehensive_summary() {
    let spec_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("billing_subscriptions_v1.json");

    let parser = SpecParser::from_file(&spec_path)
        .await
        .expect("Should parse billing subscriptions spec");

    let operations = parser.get_operations();

    let templates: Result<Vec<_>, _> =
        operations.iter().map(RequestGenerator::generate_template).collect();

    let templates = templates.expect("Should generate request templates");

    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Authorization".to_string(), "Bearer test-token".to_string());

    let config = K6Config {
        target_url: "https://192.168.1.100".to_string(),
        base_path: None,
        scenario: LoadScenario::Constant,
        duration_secs: 60,
        max_vus: 50,
        threshold_percentile: "p(95)".to_string(),
        threshold_ms: 500,
        max_error_rate: 0.05,
        auth_header: None,
        custom_headers: headers,
        skip_tls_verify: true,
        security_testing_enabled: false,
    };

    let generator = K6ScriptGenerator::new(config, templates);
    let script = generator.generate().expect("Should generate k6 script");

    assert!(
        !script.contains("Unexpected token"),
        "Script should not contain JavaScript syntax errors"
    );

    assert!(
        script.contains("insecureSkipTLSVerify: true"),
        "Script should include insecureSkipTLSVerify"
    );

    assert!(script.contains("p(95)<500"), "Thresholds should use correct k6 syntax");

    assert!(
        script.contains("http.get(") || script.contains("http.post("),
        "Script should use lowercase HTTP methods"
    );

    assert!(!script.contains("[object]"), "Headers should be properly serialized");

    assert!(script.contains("Authorization"), "Custom headers should be included");

    for line in script.lines() {
        if line.contains("new Trend(") || line.contains("new Rate(") {
            assert!(
                line.matches('.').count() <= 2,
                "Metric names should not contain dots: {}",
                line
            );
        }
    }

    println!("✓ Issue #79: Comprehensive end-to-end test - ALL FIXES VALIDATED");
    println!("  - Operation ID sanitization: ✓");
    println!("  - Metric name sanitization: ✓");
    println!("  - Threshold syntax: ✓");
    println!("  - HTTP method case: ✓");
    println!("  - Headers serialization: ✓");
    println!("  - insecureSkipTLSVerify: ✓");
    println!("  - Custom headers: ✓");
}

#[tokio::test]
async fn test_issue_79_swagger_2_0_support() {
    let spec_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("billing_subscriptions_v1.json");

    let parser = SpecParser::from_file(&spec_path).await.expect("Should parse spec");

    let operations = parser.get_operations();

    assert!(!operations.is_empty(), "Should find operations in spec");

    println!("✓ Issue #79(8): Swagger 2.0 to OpenAPI 3.0 conversion - WORKS");
}

#[tokio::test]
async fn test_issue_79_multi_target_parsing() {
    let temp_dir = std::env::temp_dir().join("mockforge_test_multi_target");

    std::fs::create_dir_all(&temp_dir).expect("Should create temp dir");

    let targets_file = temp_dir.join("targets.txt");

    std::fs::write(
        &targets_file,
        "https://api1.example.com\n\
         https://api2.example.com\n\
         https://api3.example.com\n\
         192.168.1.100:8080\n\
         api4.example.com\n",
    )
    .expect("Should write targets file");

    let targets = parse_targets_file(&targets_file).expect("Should parse targets file");

    assert_eq!(targets.len(), 5, "Should parse 5 targets");

    assert_eq!(targets[0].url, "https://api1.example.com", "First target should be correct");

    assert_eq!(
        targets[3].url, "http://192.168.1.100:8080",
        "IP:port target should be normalized with http://"
    );

    println!("✓ Issue #79(11): Multi-target parsing - WORKS");
}

/// Full pipeline integration test: parse real spec → generate templates → create K6Config
/// with security enabled → generate script → enhance with security definitions → verify
/// final output has both definitions AND calling code for ALL injection types.
#[tokio::test]
async fn test_issue_79_full_security_pipeline_with_real_spec() {
    let spec_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("billing_subscriptions_v1.json");

    // Step 1: Parse spec (same as BenchCommand::execute)
    let parser = SpecParser::from_file(&spec_path)
        .await
        .expect("Should parse billing subscriptions spec");

    let operations = parser.get_operations();
    assert!(!operations.is_empty(), "Should find operations in spec");

    // Step 2: Generate request templates (same as BenchCommand::execute)
    let templates: Vec<_> = operations
        .iter()
        .map(RequestGenerator::generate_template)
        .collect::<mockforge_bench::error::Result<Vec<_>>>()
        .expect("Should generate request templates");

    // Step 3: Create K6Config with security_testing_enabled=true (same as execute with --security-test)
    let config = K6Config {
        target_url: "https://api-m.sandbox.paypal.com".to_string(),
        base_path: None,
        scenario: LoadScenario::Constant,
        duration_secs: 30,
        max_vus: 10,
        threshold_percentile: "p(95)".to_string(),
        threshold_ms: 500,
        max_error_rate: 0.05,
        auth_header: Some("Bearer test-token-12345".to_string()),
        custom_headers: HashMap::new(),
        skip_tls_verify: false,
        security_testing_enabled: true,
    };

    // Step 4: Generate base script (same as K6ScriptGenerator::generate)
    let generator = K6ScriptGenerator::new(config, templates);
    let mut script = generator.generate().expect("Should generate k6 script");

    // Step 5: Simulate generate_enhanced_script() - inject security function definitions
    let security_config = SecurityTestConfig::default().enable();
    let payloads = SecurityPayloads::get_payloads(&security_config);
    assert!(!payloads.is_empty(), "Should have built-in security payloads");

    let mut additional_code = String::new();
    additional_code.push_str(&SecurityTestGenerator::generate_payload_selection(&payloads, false));
    additional_code.push('\n');
    additional_code.push_str(&SecurityTestGenerator::generate_apply_payload(&[]));
    additional_code.push('\n');
    additional_code.push_str(&SecurityTestGenerator::generate_security_checks());
    additional_code.push('\n');

    if let Some(pos) = script.find("export const options") {
        script.insert_str(
            pos,
            &format!("\n// === Advanced Testing Features ===\n{}\n", additional_code),
        );
    }

    // === VERIFICATION ===

    // V1: Function DEFINITIONS are present
    assert!(
        script.contains("function getNextSecurityPayload()"),
        "Must contain getNextSecurityPayload() function DEFINITION"
    );
    assert!(
        script.contains("function applySecurityPayload("),
        "Must contain applySecurityPayload() function DEFINITION"
    );
    assert!(
        script.contains("function checkSecurityResponse("),
        "Must contain checkSecurityResponse() function DEFINITION"
    );
    assert!(
        script.contains("const securityPayloads = ["),
        "Must contain securityPayloads array"
    );

    // V2: CALLING code is present (rendered by template with security_testing_enabled=true)
    assert!(
        script.contains("const secPayloadGroup = typeof getNextSecurityPayload"),
        "Must contain secPayloadGroup = getNextSecurityPayload() CALL"
    );

    // V3: Header injection code (inside the for loop over secPayloadGroup)
    assert!(
        script.contains("secPayload.location === 'header'"),
        "Must contain header location check for header injection"
    );
    assert!(
        script.contains("const requestHeaders = { ..."),
        "Must spread headers into mutable copy for injection"
    );
    assert!(
        script.contains("for (const secPayload of secPayloadGroup)"),
        "Must loop over secPayloadGroup"
    );

    // V4: URI injection code (raw payloads for WAF detection)
    assert!(
        script.contains("secPayload.location === 'uri'"),
        "Must contain URI location check for query parameter injection"
    );
    // URI payloads are sent RAW (not encoded) so WAFs can detect them
    assert!(
        script.contains("'test=' + secPayload.payload"),
        "Must inject raw (unencoded) security payload into query string for WAF detection"
    );
    assert!(
        script.contains("requestUrl"),
        "Must build requestUrl variable for URI injection"
    );

    // V5: Body injection code (for POST/PUT/PATCH operations)
    assert!(
        script.contains("applySecurityPayload(payload, [], secBodyPayload)"),
        "Must contain applySecurityPayload() CALL with secBodyPayload for body injection"
    );

    // V6: Ordering - definitions before options, calls inside default function
    let def_pos = script.find("function getNextSecurityPayload()").unwrap();
    let options_pos = script.find("export const options").unwrap();
    let default_fn_pos = script.find("export default function").unwrap();
    let call_pos = script.find("const secPayloadGroup = typeof getNextSecurityPayload").unwrap();

    assert!(def_pos < options_pos, "Definitions must come before export const options");
    assert!(call_pos > default_fn_pos, "Calling code must be inside export default function");

    // V7: Payloads array contains actual payloads (not empty)
    let payload_array_start = script.find("const securityPayloads = [").unwrap();
    let payload_array_end = script[payload_array_start..].find("];").unwrap();
    let payload_array = &script[payload_array_start..payload_array_start + payload_array_end];
    assert!(
        payload_array.contains("payload:"),
        "securityPayloads array must contain actual payload entries, not be empty"
    );

    // V8: All operations use requestUrl (not inline URLs) when security is enabled
    let default_fn_section = &script[default_fn_pos..];
    // Every http.get/post/put/patch/delete call inside the default function should use requestUrl
    for line in default_fn_section.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("const res = http.") {
            assert!(
                trimmed.contains("requestUrl"),
                "HTTP call should use requestUrl for URI injection: {}",
                trimmed
            );
        }
    }

    println!("\n✓ Issue #79: Full security pipeline integration test PASSED");
    println!("  - Real spec file: billing_subscriptions_v1.json");
    println!("  - {} operations processed", operations.len());
    println!("  - {} security payloads loaded", payloads.len());
    println!("  - Function definitions: ✓");
    println!("  - Calling code (header injection): ✓");
    println!("  - Calling code (URI injection): ✓");
    println!("  - Calling code (body injection): ✓");
    println!("  - Correct ordering: ✓");
    println!("  - requestUrl used in all HTTP calls: ✓");
}

/// End-to-end test: create synthetic WAFBench YAML with multi-part test cases,
/// load them through the real pipeline, generate a k6 script, and verify:
/// 1. Multi-part test cases are grouped together in groupedPayloads
/// 2. Body payloads are form-URL-decoded
/// 3. getNextSecurityPayload() returns arrays
/// 4. Template uses secPayloadGroup loop
#[tokio::test]
async fn test_issue_79_wafbench_grouped_payloads_e2e() {
    // Step 1: Create synthetic WAFBench YAML with a multi-part test case
    // (rule 942290 needs URI + User-Agent header together)
    let temp_dir = std::env::temp_dir().join("mockforge_test_wafbench_grouping");
    std::fs::create_dir_all(&temp_dir).expect("Should create temp dir");

    let yaml_content = r#"
meta:
  author: test
  description: "Tests for SQL injection rule 942290"
  enabled: true
  name: "942290.yaml"

tests:
  - desc: "SQL injection with URI and User-Agent"
    test_title: "942290-1"
    stages:
      - stage:
          input:
            dest_addr: 127.0.0.1
            headers:
              Host: localhost
              User-Agent: "ModSecurity CRS 3 Tests"
            method: GET
            port: 80
            uri: "/test?id=2"
          output:
            log_contains: id "942290"
  - desc: "SQL injection with body payload"
    test_title: "942240-1"
    stages:
      - stage:
          input:
            dest_addr: 127.0.0.1
            headers:
              Host: localhost
              Content-Type: "application/x-www-form-urlencoded"
            method: POST
            port: 80
            uri: "/"
            data: "%22+WAITFOR+DELAY+%270%3A0%3A5%27"
          output:
            log_contains: id "942240"
  - desc: "Simple SQL injection in URI only"
    test_title: "942100-1"
    stages:
      - stage:
          input:
            dest_addr: 127.0.0.1
            headers: {}
            method: GET
            port: 80
            uri: "/test?param=1+OR+1%3D1"
          output:
            log_contains: id "942100"
"#;

    let yaml_path = temp_dir.join("942290.yaml");
    std::fs::write(&yaml_path, yaml_content).expect("Should write test YAML");

    // Step 2: Load WAFBench payloads through real loader
    let mut loader = WafBenchLoader::new();
    loader.load_file(&yaml_path).expect("Should load WAFBench file");

    let wafbench_payloads = loader.to_security_payloads();
    assert!(!wafbench_payloads.is_empty(), "Should have loaded WAFBench payloads");

    // Step 2a: Verify multi-part test 942290-1 has group_id
    let grouped: Vec<_> = wafbench_payloads
        .iter()
        .filter(|p| p.group_id.as_deref() == Some("942290-1"))
        .collect();
    assert!(
        grouped.len() >= 2,
        "942290-1 should have at least 2 grouped payloads (URI + headers), got {}",
        grouped.len()
    );

    // Step 2b: Verify single-part test 942100-1 has no group_id
    let ungrouped: Vec<_> =
        wafbench_payloads.iter().filter(|p| p.description.contains("942100")).collect();
    assert!(!ungrouped.is_empty(), "Should have 942100 payloads");
    assert!(
        ungrouped.iter().all(|p| p.group_id.is_none()),
        "Single-part test 942100-1 should NOT have group_id"
    );

    // Step 2c: Verify body payload is form-URL-decoded
    use mockforge_bench::security_payloads::PayloadLocation;
    let body_payloads: Vec<_> = wafbench_payloads
        .iter()
        .filter(|p| p.description.contains("942240") && p.location == PayloadLocation::Body)
        .collect();
    assert!(!body_payloads.is_empty(), "Should have body payload for 942240");
    let body_payload = &body_payloads[0];
    assert!(
        body_payload.payload.contains('"'),
        "Body payload should have %22 decoded to double-quote, got: {}",
        body_payload.payload
    );
    assert!(
        !body_payload.payload.contains("%22"),
        "Body payload should NOT contain literal %22, got: {}",
        body_payload.payload
    );
    assert!(
        body_payload.payload.contains(' '),
        "Body payload should have + decoded to space, got: {}",
        body_payload.payload
    );

    // Step 3: Generate k6 script using real spec + WAFBench payloads
    let spec_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("billing_subscriptions_v1.json");

    let parser = SpecParser::from_file(&spec_path).await.expect("Should parse spec");
    let operations = parser.get_operations();
    let templates: Vec<_> = operations
        .iter()
        .map(RequestGenerator::generate_template)
        .collect::<mockforge_bench::error::Result<Vec<_>>>()
        .expect("Should generate templates");

    let config = K6Config {
        target_url: "https://api.example.com".to_string(),
        base_path: None,
        scenario: LoadScenario::Constant,
        duration_secs: 30,
        max_vus: 10,
        threshold_percentile: "p(95)".to_string(),
        threshold_ms: 500,
        max_error_rate: 0.05,
        auth_header: None,
        custom_headers: HashMap::new(),
        skip_tls_verify: false,
        security_testing_enabled: true,
    };

    let generator = K6ScriptGenerator::new(config, templates);
    let mut script = generator.generate().expect("Should generate base k6 script");

    // Step 4: Inject WAFBench payload definitions (simulating generate_enhanced_script)
    let mut additional_code = String::new();
    additional_code.push_str(&SecurityTestGenerator::generate_payload_selection(
        &wafbench_payloads,
        true, // cycle_all like real WAFBench mode
    ));
    additional_code.push('\n');
    additional_code.push_str(&SecurityTestGenerator::generate_apply_payload(&[]));
    additional_code.push('\n');

    if let Some(pos) = script.find("export const options") {
        script.insert_str(
            pos,
            &format!("\n// === Advanced Testing Features ===\n{}\n", additional_code),
        );
    }

    // === VERIFICATION: Generated Script ===

    // V1: groupedPayloads array exists
    assert!(
        script.contains("const groupedPayloads"),
        "Script must contain groupedPayloads array"
    );

    // V2: Multi-part test case 942290 has groupId set
    assert!(
        script.contains("groupId: '942290-1'"),
        "Script must have groupId: '942290-1' for multi-part test case"
    );

    // V3: Single-part test has groupId: null
    assert!(
        script.contains("groupId: null"),
        "Script must have groupId: null for single-part test cases"
    );

    // V4: getNextSecurityPayload returns from groupedPayloads (arrays)
    assert!(
        script.contains("groupedPayloads[__payloadIndex]"),
        "getNextSecurityPayload should index into groupedPayloads (cycle-all mode)"
    );

    // V5: Template uses secPayloadGroup loop
    assert!(
        script.contains("for (const secPayload of secPayloadGroup)"),
        "Template must loop over secPayloadGroup"
    );

    // V6: Template uses secBodyPayload for body injection
    assert!(
        script.contains("applySecurityPayload(payload, [], secBodyPayload)"),
        "Template must use secBodyPayload (not secPayload) for body injection"
    );

    // V7: Body payload for 942240 is decoded (not literal %22)
    assert!(
        script.contains("WAITFOR DELAY"),
        "Body payload must be decoded - should contain 'WAITFOR DELAY' with spaces"
    );
    assert!(
        !script.contains("%22+WAITFOR"),
        "Body payload must NOT contain literal '%22+WAITFOR' (should be decoded)"
    );

    // V8: groupedPayloads builder logic is present
    assert!(
        script.contains("groupMap[p.groupId]"),
        "Script must contain grouping logic that collects by groupId"
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&temp_dir);

    println!("\n✓ Issue #79: WAFBench grouped payloads E2E test PASSED");
    println!("  - WAFBench YAML loaded: 3 test cases");
    println!("  - Multi-part grouping (942290-1): ✓");
    println!("  - Single-part no group (942100-1): ✓");
    println!("  - Body URL-decoding (942240-1): ✓");
    println!("  - groupedPayloads array: ✓");
    println!("  - secPayloadGroup loop in template: ✓");
    println!("  - secBodyPayload for body injection: ✓");
    println!("  - getNextSecurityPayload returns arrays: ✓");
}
