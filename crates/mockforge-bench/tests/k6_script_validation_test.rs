//! k6 script structural validation tests
//!
//! These tests generate k6 scripts from the billing spec fixture and verify
//! that the output is structurally valid JavaScript: balanced delimiters,
//! required k6 imports, no unresolved Handlebars expressions, etc.

use mockforge_bench::k6_gen::{K6Config, K6ScriptGenerator};
use mockforge_bench::request_gen::RequestGenerator;
use mockforge_bench::scenarios::LoadScenario;
use mockforge_bench::spec_parser::SpecParser;
use std::collections::HashMap;
use std::path::PathBuf;

/// Path to the billing spec fixture shared across tests.
fn billing_spec_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("billing_subscriptions_v1.json")
}

/// Helper: parse spec and build templates.
async fn templates_from_billing_spec() -> Vec<mockforge_bench::request_gen::RequestTemplate> {
    let parser = SpecParser::from_file(&billing_spec_path()).await.expect("parse billing spec");
    let ops = parser.get_operations();
    ops.iter()
        .map(RequestGenerator::generate_template)
        .collect::<mockforge_bench::error::Result<Vec<_>>>()
        .expect("generate templates")
}

/// Helper: generate a k6 script with the given configuration overrides.
async fn generate_script(
    scenario: LoadScenario,
    security_enabled: bool,
    base_path: Option<String>,
    custom_headers: HashMap<String, String>,
) -> String {
    let templates = templates_from_billing_spec().await;
    let config = K6Config {
        target_url: "https://api-m.sandbox.paypal.com".to_string(),
        base_path,
        scenario,
        duration_secs: 30,
        max_vus: 50,
        threshold_percentile: "p(95)".to_string(),
        threshold_ms: 500,
        max_error_rate: 0.05,
        auth_header: None,
        custom_headers,
        skip_tls_verify: false,
        security_testing_enabled: security_enabled,
    };
    let generator = K6ScriptGenerator::new(config, templates);
    generator.generate().expect("generate k6 script")
}

// ---------------------------------------------------------------------------
// Delimiter balance
// ---------------------------------------------------------------------------

/// Verify that braces, brackets, and parentheses are balanced.
fn assert_balanced_delimiters(script: &str) {
    let mut braces = 0i64;
    let mut brackets = 0i64;
    let mut parens = 0i64;

    for ch in script.chars() {
        match ch {
            '{' => braces += 1,
            '}' => braces -= 1,
            '[' => brackets += 1,
            ']' => brackets -= 1,
            '(' => parens += 1,
            ')' => parens -= 1,
            _ => {}
        }
        assert!(braces >= 0, "unmatched closing brace");
        assert!(brackets >= 0, "unmatched closing bracket");
        assert!(parens >= 0, "unmatched closing paren");
    }
    assert_eq!(braces, 0, "unbalanced braces: excess {braces}");
    assert_eq!(brackets, 0, "unbalanced brackets: excess {brackets}");
    assert_eq!(parens, 0, "unbalanced parens: excess {parens}");
}

#[tokio::test]
async fn balanced_delimiters_constant() {
    let script = generate_script(LoadScenario::Constant, false, None, HashMap::new()).await;
    assert_balanced_delimiters(&script);
}

#[tokio::test]
async fn balanced_delimiters_ramp_up() {
    let script = generate_script(LoadScenario::RampUp, false, None, HashMap::new()).await;
    assert_balanced_delimiters(&script);
}

#[tokio::test]
async fn balanced_delimiters_spike() {
    let script = generate_script(LoadScenario::Spike, false, None, HashMap::new()).await;
    assert_balanced_delimiters(&script);
}

#[tokio::test]
async fn balanced_delimiters_security_enabled() {
    let script = generate_script(LoadScenario::Constant, true, None, HashMap::new()).await;
    assert_balanced_delimiters(&script);
}

#[tokio::test]
async fn balanced_delimiters_with_base_path() {
    let script = generate_script(
        LoadScenario::Constant,
        false,
        Some("/v2/billing".to_string()),
        HashMap::new(),
    )
    .await;
    assert_balanced_delimiters(&script);
}

#[tokio::test]
async fn balanced_delimiters_custom_headers() {
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer test-token".to_string());
    headers.insert("X-Custom".to_string(), "value".to_string());
    let script = generate_script(LoadScenario::Constant, false, None, headers).await;
    assert_balanced_delimiters(&script);
}

// ---------------------------------------------------------------------------
// Required k6 constructs
// ---------------------------------------------------------------------------

#[tokio::test]
async fn required_k6_imports() {
    let script = generate_script(LoadScenario::Constant, false, None, HashMap::new()).await;
    assert!(script.contains("import http from"), "script must import k6/http");
}

#[tokio::test]
async fn export_options_present() {
    let script = generate_script(LoadScenario::Constant, false, None, HashMap::new()).await;
    assert!(script.contains("export const options"), "script must export const options");
}

#[tokio::test]
async fn export_default_function_present() {
    let script = generate_script(LoadScenario::Constant, false, None, HashMap::new()).await;
    assert!(
        script.contains("export default function"),
        "script must export default function"
    );
}

// ---------------------------------------------------------------------------
// No unresolved Handlebars
// ---------------------------------------------------------------------------

#[tokio::test]
async fn no_unresolved_handlebars_constant() {
    let script = generate_script(LoadScenario::Constant, false, None, HashMap::new()).await;
    assert!(
        !script.contains("{{"),
        "no unresolved Handlebars expressions should remain in the output"
    );
}

#[tokio::test]
async fn no_unresolved_handlebars_security() {
    let script = generate_script(LoadScenario::Constant, true, None, HashMap::new()).await;
    assert!(
        !script.contains("{{"),
        "no unresolved Handlebars expressions should remain when security is enabled"
    );
}

#[tokio::test]
async fn no_unresolved_handlebars_base_path() {
    let script =
        generate_script(LoadScenario::RampUp, false, Some("/api".to_string()), HashMap::new())
            .await;
    assert!(!script.contains("{{"), "no unresolved Handlebars expressions with base path");
}

// ---------------------------------------------------------------------------
// Base path injection
// ---------------------------------------------------------------------------

#[tokio::test]
async fn base_path_appears_in_script() {
    let script = generate_script(
        LoadScenario::Constant,
        false,
        Some("/v2/billing".to_string()),
        HashMap::new(),
    )
    .await;
    assert!(
        script.contains("/v2/billing"),
        "base path should appear in the generated script"
    );
}

// ---------------------------------------------------------------------------
// Custom headers
// ---------------------------------------------------------------------------

#[tokio::test]
async fn custom_headers_appear_in_script() {
    let mut headers = HashMap::new();
    headers.insert("X-Correlation-ID".to_string(), "test-123".to_string());
    let script = generate_script(LoadScenario::Constant, false, None, headers).await;
    assert!(
        script.contains("X-Correlation-ID"),
        "custom headers should appear in the generated script"
    );
}
