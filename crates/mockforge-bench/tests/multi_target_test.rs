//! Integration tests for multi-target bench testing
//!
//! Tests the parallel execution of load tests against multiple targets,
//! including concurrency limiting, error handling, and results aggregation.

use mockforge_bench::command::BenchCommand;
use mockforge_bench::target_parser::{parse_targets_file, TargetConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a minimal OpenAPI spec for testing
fn create_minimal_spec_file() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let spec_path = temp_dir.path().join("test_spec.json");

    let spec = serde_json::json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Test API",
            "version": "1.0.0"
        },
        "paths": {
            "/test": {
                "get": {
                    "operationId": "test_get",
                    "responses": {
                        "200": {
                            "description": "Success"
                        }
                    }
                }
            }
        }
    });

    std::fs::write(&spec_path, serde_json::to_string_pretty(&spec).unwrap()).unwrap();
    (temp_dir, spec_path)
}

#[tokio::test]
async fn test_parse_targets_file_text_format() {
    let temp_dir = TempDir::new().unwrap();
    let targets_file = temp_dir.path().join("targets.txt");

    let content = r#"
https://api1.example.com
https://api2.example.com
192.168.1.100:8080
api3.example.com
# This is a comment
    "#;

    std::fs::write(&targets_file, content).unwrap();

    let targets = parse_targets_file(&targets_file).unwrap();
    assert_eq!(targets.len(), 4);
    assert_eq!(targets[0].url, "https://api1.example.com");
    assert_eq!(targets[1].url, "https://api2.example.com");
    assert_eq!(targets[2].url, "http://192.168.1.100:8080");
    assert_eq!(targets[3].url, "http://api3.example.com");
}

#[tokio::test]
async fn test_parse_targets_file_json_format() {
    let temp_dir = TempDir::new().unwrap();
    let targets_file = temp_dir.path().join("targets.json");

    let content = r#"
[
  {
    "url": "https://api1.example.com",
    "auth": "Bearer token1",
    "headers": {
      "X-Custom": "value1"
    }
  },
  {
    "url": "https://api2.example.com"
  }
]
    "#;

    std::fs::write(&targets_file, content).unwrap();

    let targets = parse_targets_file(&targets_file).unwrap();
    assert_eq!(targets.len(), 2);
    assert_eq!(targets[0].url, "https://api1.example.com");
    assert_eq!(targets[0].auth, Some("Bearer token1".to_string()));
    assert_eq!(
        targets[0].headers.as_ref().unwrap().get("X-Custom"),
        Some(&"value1".to_string())
    );
    assert_eq!(targets[1].url, "https://api2.example.com");
}

#[tokio::test]
async fn test_parse_targets_file_empty() {
    let temp_dir = TempDir::new().unwrap();
    let targets_file = temp_dir.path().join("empty.txt");

    std::fs::write(&targets_file, "").unwrap();

    let result = parse_targets_file(&targets_file);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No valid targets"));
}

#[tokio::test]
async fn test_parse_targets_file_invalid_json() {
    let temp_dir = TempDir::new().unwrap();
    let targets_file = temp_dir.path().join("invalid.json");

    std::fs::write(&targets_file, "{ invalid json }").unwrap();

    let result = parse_targets_file(&targets_file);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_bench_command_parse_headers() {
    let (_temp_dir, spec_path) = create_minimal_spec_file();

    let cmd = BenchCommand {
        spec: vec![spec_path],
        spec_dir: None,
        merge_conflicts: "error".to_string(),
        spec_mode: "merge".to_string(),
        dependency_config: None,
        target: "http://localhost".to_string(),
        base_path: None,
        duration: "1m".to_string(),
        vus: 10,
        scenario: "ramp-up".to_string(),
        operations: None,
        exclude_operations: None,
        auth: None,
        headers: Some("X-API-Key:test123,X-Client-ID:client456".to_string()),
        output: PathBuf::from("output"),
        generate_only: false,
        script_output: None,
        threshold_percentile: "p(95)".to_string(),
        threshold_ms: 500,
        max_error_rate: 0.05,
        verbose: false,
        skip_tls_verify: false,
        targets_file: None,
        max_concurrency: None,
        results_format: "both".to_string(),
        params_file: None,
        crud_flow: false,
        flow_config: None,
        extract_fields: None,
        parallel_create: None,
        data_file: None,
        data_distribution: "unique-per-vu".to_string(),
        data_mappings: None,
        per_uri_control: false,
        error_rate: None,
        error_types: None,
        security_test: false,
        security_payloads: None,
        security_categories: None,
        security_target_fields: None,
        wafbench_dir: None,
        wafbench_cycle_all: false,
        owasp_api_top10: false,
        owasp_categories: None,
        owasp_auth_header: "Authorization".to_string(),
        owasp_auth_token: None,
        owasp_admin_paths: None,
        owasp_id_fields: None,
        owasp_report: None,
        owasp_report_format: "json".to_string(),
        owasp_iterations: 1,
        conformance: false,
        conformance_api_key: None,
        conformance_basic_auth: None,
        conformance_report: PathBuf::from("conformance-report.json"),
        conformance_categories: None,
        conformance_report_format: "json".to_string(),
    };

    let headers = cmd.parse_headers().unwrap();
    assert_eq!(headers.get("X-API-Key"), Some(&"test123".to_string()));
    assert_eq!(headers.get("X-Client-ID"), Some(&"client456".to_string()));
}

#[tokio::test]
async fn test_bench_command_parse_headers_invalid_format() {
    let (_temp_dir, spec_path) = create_minimal_spec_file();

    let cmd = BenchCommand {
        spec: vec![spec_path],
        spec_dir: None,
        merge_conflicts: "error".to_string(),
        spec_mode: "merge".to_string(),
        dependency_config: None,
        target: "http://localhost".to_string(),
        base_path: None,
        duration: "1m".to_string(),
        vus: 10,
        scenario: "ramp-up".to_string(),
        operations: None,
        exclude_operations: None,
        auth: None,
        headers: Some("InvalidFormat".to_string()),
        output: PathBuf::from("output"),
        generate_only: false,
        script_output: None,
        threshold_percentile: "p(95)".to_string(),
        threshold_ms: 500,
        max_error_rate: 0.05,
        verbose: false,
        skip_tls_verify: false,
        targets_file: None,
        max_concurrency: None,
        results_format: "both".to_string(),
        params_file: None,
        crud_flow: false,
        flow_config: None,
        extract_fields: None,
        parallel_create: None,
        data_file: None,
        data_distribution: "unique-per-vu".to_string(),
        data_mappings: None,
        per_uri_control: false,
        error_rate: None,
        error_types: None,
        security_test: false,
        security_payloads: None,
        security_categories: None,
        security_target_fields: None,
        wafbench_dir: None,
        wafbench_cycle_all: false,
        owasp_api_top10: false,
        owasp_categories: None,
        owasp_auth_header: "Authorization".to_string(),
        owasp_auth_token: None,
        owasp_admin_paths: None,
        owasp_id_fields: None,
        owasp_report: None,
        owasp_report_format: "json".to_string(),
        owasp_iterations: 1,
        conformance: false,
        conformance_api_key: None,
        conformance_basic_auth: None,
        conformance_report: PathBuf::from("conformance-report.json"),
        conformance_categories: None,
        conformance_report_format: "json".to_string(),
    };

    let result = cmd.parse_headers();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid header format"));
}

#[tokio::test]
async fn test_bench_command_parse_duration() {
    assert_eq!(BenchCommand::parse_duration("30s").unwrap(), 30);
    assert_eq!(BenchCommand::parse_duration("5m").unwrap(), 300);
    assert_eq!(BenchCommand::parse_duration("1h").unwrap(), 3600);
    assert_eq!(BenchCommand::parse_duration("60").unwrap(), 60);
}

#[tokio::test]
async fn test_bench_command_parse_duration_invalid() {
    assert!(BenchCommand::parse_duration("invalid").is_err());
    assert!(BenchCommand::parse_duration("30x").is_err());
}

#[tokio::test]
async fn test_target_config_normalize_url() {
    let mut target = TargetConfig::from_url("api.example.com".to_string());
    target.normalize_url();
    assert_eq!(target.url, "http://api.example.com");

    let mut target2 = TargetConfig::from_url("192.168.1.1:8080".to_string());
    target2.normalize_url();
    assert_eq!(target2.url, "http://192.168.1.1:8080");

    let mut target3 = TargetConfig::from_url("https://api.example.com".to_string());
    target3.normalize_url();
    assert_eq!(target3.url, "https://api.example.com");
}

#[tokio::test]
async fn test_target_config_with_auth_and_headers() {
    let mut headers = HashMap::new();
    headers.insert("X-Custom".to_string(), "value".to_string());

    let target = TargetConfig {
        url: "https://api.example.com".to_string(),
        auth: Some("Bearer token123".to_string()),
        headers: Some(headers),
        spec: None,
    };

    assert_eq!(target.url, "https://api.example.com");
    assert_eq!(target.auth, Some("Bearer token123".to_string()));
    assert_eq!(target.headers.as_ref().unwrap().get("X-Custom"), Some(&"value".to_string()));
}

// Note: Full integration tests that actually execute k6 would require k6 to be installed
// and would take significant time. These are better suited for manual testing or CI/CD pipelines.
// The unit tests above cover the core logic and parsing functionality.
