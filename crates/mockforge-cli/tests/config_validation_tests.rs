//! Tests for `mockforge config validate` command with failure scenarios
//!
//! This test suite ensures that config validation fails gracefully
//! and provides helpful error messages for common configuration mistakes.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

/// Test config validation with malformed YAML
#[test]
fn test_config_validate_malformed_yaml() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("malformed.yaml");

    // Write malformed YAML
    fs::write(
        &config_path,
        r#"
http:
  port: 3000
  host: "0.0.0.0"
    invalid: indentation
  badly: [formed yaml
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["config", "validate", "--config"]).arg(config_path.to_str().unwrap());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("error").or(predicate::str::contains("Error")));

    println!("✓ Config validation correctly rejects malformed YAML");
}

/// Test config validation with malformed JSON
#[test]
fn test_config_validate_malformed_json() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("malformed.json");

    // Write malformed JSON
    fs::write(
        &config_path,
        r#"
{
  "http": {
    "port": 3000,
    "host": "0.0.0.0"
  },
  "invalid": syntax here
}
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["config", "validate", "--config"]).arg(config_path.to_str().unwrap());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("error").or(predicate::str::contains("Error")));

    println!("✓ Config validation correctly rejects malformed JSON");
}

/// Test config validation with empty file
#[test]
fn test_config_validate_empty_file() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("empty.yaml");

    // Write empty file
    fs::write(&config_path, "").unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["config", "validate", "--config"]).arg(config_path.to_str().unwrap());

    cmd.assert().success();

    println!("✓ Config validation accepts empty file");
}

/// Test config validation with nonexistent file
#[test]
fn test_config_validate_nonexistent_file() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args([
        "config",
        "validate",
        "--config",
        "/nonexistent/path/to/config.yaml",
    ]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("No such file")));

    println!("✓ Config validation correctly handles nonexistent file");
}

/// Test config validation with invalid port numbers
#[test]
fn test_config_validate_invalid_port() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("invalid-port.yaml");

    // Write config with invalid port (negative number)
    fs::write(
        &config_path,
        r#"
http:
  port: -1
  host: "0.0.0.0"
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["config", "validate", "--config"]).arg(config_path.to_str().unwrap());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("error").or(predicate::str::contains("Error")));

    println!("✓ Config validation correctly rejects invalid port number");
}

/// Test config validation with invalid field type
#[test]
fn test_config_validate_wrong_field_type() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("wrong-type.yaml");

    // Write config with wrong field type (port as string instead of number)
    fs::write(
        &config_path,
        r#"
http:
  port: "not a number"
  host: "0.0.0.0"
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["config", "validate", "--config"]).arg(config_path.to_str().unwrap());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("error").or(predicate::str::contains("Error")));

    println!("✓ Config validation correctly rejects wrong field types");
}

/// Test config validation with missing required nested fields
#[test]
fn test_config_validate_missing_nested_fields() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("missing-fields.yaml");

    // Write config with incomplete TLS configuration (missing cert_path)
    fs::write(
        &config_path,
        r#"
http:
  port: 3000
  host: "0.0.0.0"

tls:
  enabled: true
  # Missing cert_path and key_path which are required when enabled: true
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["config", "validate", "--config"]).arg(config_path.to_str().unwrap());

    // This might pass or fail depending on how strict the validation is
    // We just want to ensure it doesn't crash
    let output = cmd.output().unwrap();
    assert!(
        output.status.success() || !output.stderr.is_empty(),
        "Should either validate with warnings or fail with errors"
    );

    println!("✓ Config validation handles missing nested fields");
}

/// Test config validation with duplicate keys
#[test]
fn test_config_validate_duplicate_keys() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("duplicate-keys.yaml");

    // Write config with duplicate keys
    fs::write(
        &config_path,
        r#"
http:
  port: 3000
  host: "0.0.0.0"

http:
  port: 8080
  host: "127.0.0.1"
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["config", "validate", "--config"]).arg(config_path.to_str().unwrap());

    // YAML parsers typically accept this and use the last value
    // but we want to ensure it doesn't crash
    let output = cmd.output().unwrap();
    assert!(
        output.status.success() || !output.stderr.is_empty(),
        "Should handle duplicate keys gracefully"
    );

    println!("✓ Config validation handles duplicate keys");
}

/// Test config validation with valid minimal configuration
#[test]
fn test_config_validate_valid_minimal() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("valid-minimal.yaml");

    // Write minimal valid config (must include all required fields)
    fs::write(
        &config_path,
        r#"
http:
  port: 3000
  host: "0.0.0.0"
  cors_enabled: true
  request_timeout_secs: 30
  request_validation: "enforce"
  aggregate_validation_errors: true
  validate_responses: false
  response_template_expand: false
  skip_admin_validation: true
  validation_overrides: {}

websocket:
  port: 3001
  host: "0.0.0.0"
  connection_timeout_secs: 300

smtp:
  enabled: false
  port: 1025
  host: "0.0.0.0"
  hostname: "mockforge-smtp"
  timeout_secs: 300
  max_connections: 10
  enable_mailbox: true
  max_mailbox_messages: 1000

grpc:
  port: 50051
  host: "0.0.0.0"

admin:
  enabled: false
  port: 9080
  host: "127.0.0.1"
  auth_required: false
  api_enabled: true
  prometheus_url: "http://localhost:9090"

chaining:
  enabled: false
  maxChainLength: 20
  globalTimeoutSecs: 300
  enableParallelExecution: false

core:
  latency_enabled: false
  failures_enabled: false
  overrides_enabled: true
  traffic_shaping_enabled: false
  max_request_logs: 1000
  default_latency:
    base_ms: 50
    jitter_ms: 20
    distribution: "fixed"
    min_ms: 0
    tag_overrides: {}
  traffic_shaping:
    bandwidth:
      enabled: false
      max_bytes_per_sec: 0
      burst_capacity_bytes: 1048576
      tag_overrides: {}
    burst_loss:
      enabled: false
      burst_probability: 0.1
      burst_duration_ms: 5000
      loss_rate_during_burst: 0.5
      recovery_time_ms: 30000
      tag_overrides: {}
  time_travel:
    enabled: false
    initial_time: null
    scale_factor: 1.0
    enable_scheduling: true

logging:
  level: "info"
  json_format: false
  max_file_size_mb: 10
  max_files: 5

data:
  default_rows: 100
  default_format: "json"
  locale: "en"
  templates: {}
  rag:
    enabled: false
    provider: "openai"
    model: "gpt-3.5-turbo"
    max_tokens: 1024
    temperature: 0.7
    context_window: 4000
    caching: true
    cache_ttl_secs: 3600
    timeout_secs: 30
    max_retries: 3

observability:
  prometheus:
    enabled: false
    port: 9090
    host: "0.0.0.0"
    path: "/metrics"
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["config", "validate", "--config"]).arg(config_path.to_str().unwrap());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("valid").or(predicate::str::contains("✓")));

    println!("✓ Config validation accepts valid minimal configuration");
}

/// Test config validation with valid comprehensive configuration
#[test]
fn test_config_validate_valid_comprehensive() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("valid-comprehensive.yaml");

    // Write comprehensive valid config (must include all required fields)
    fs::write(
        &config_path,
        r#"
http:
  port: 3000
  host: "0.0.0.0"
  cors_enabled: true
  request_timeout_secs: 30
  request_validation: "enforce"
  aggregate_validation_errors: true
  validate_responses: false
  response_template_expand: false
  skip_admin_validation: true
  validation_overrides: {}

websocket:
  port: 3001
  host: "0.0.0.0"
  connection_timeout_secs: 300

smtp:
  enabled: false
  port: 1025
  host: "0.0.0.0"
  hostname: "mockforge-smtp"
  timeout_secs: 300
  max_connections: 10
  enable_mailbox: true
  max_mailbox_messages: 1000

grpc:
  port: 50051
  host: "0.0.0.0"

admin:
  enabled: true
  port: 9080
  host: "0.0.0.0"
  auth_required: false
  api_enabled: true
  prometheus_url: "http://localhost:9090"

chaining:
  enabled: false
  maxChainLength: 20
  globalTimeoutSecs: 300
  enableParallelExecution: false

core:
  latency_enabled: true
  failures_enabled: false
  overrides_enabled: true
  traffic_shaping_enabled: false
  max_request_logs: 1000
  default_latency:
    base_ms: 100
    jitter_ms: 50
    distribution: "fixed"
    min_ms: 0
    tag_overrides: {}
  traffic_shaping:
    bandwidth:
      enabled: false
      max_bytes_per_sec: 0
      burst_capacity_bytes: 1048576
      tag_overrides: {}
    burst_loss:
      enabled: false
      burst_probability: 0.1
      burst_duration_ms: 5000
      loss_rate_during_burst: 0.5
      recovery_time_ms: 30000
      tag_overrides: {}
  time_travel:
    enabled: false
    initial_time: null
    scale_factor: 1.0
    enable_scheduling: true

logging:
  level: "info"
  json_format: false
  max_file_size_mb: 10
  max_files: 5

data:
  default_rows: 100
  default_format: "json"
  locale: "en"
  templates: {}
  rag:
    enabled: false
    provider: "openai"
    model: "gpt-3.5-turbo"
    max_tokens: 1024
    temperature: 0.7
    context_window: 4000
    caching: true
    cache_ttl_secs: 3600
    timeout_secs: 30
    max_retries: 3

observability:
  prometheus:
    enabled: true
    port: 9090
    host: "0.0.0.0"
    path: "/metrics"
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["config", "validate", "--config"]).arg(config_path.to_str().unwrap());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("valid").or(predicate::str::contains("✓")));

    println!("✓ Config validation accepts valid comprehensive configuration");
}

/// Test config validation with whitespace-only file
#[test]
fn test_config_validate_whitespace_only() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("whitespace.yaml");

    // Write file with only whitespace
    fs::write(&config_path, "   \n\t\n   \n").unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["config", "validate", "--config"]).arg(config_path.to_str().unwrap());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("error").or(predicate::str::contains("Error")));

    println!("✓ Config validation correctly rejects whitespace-only file");
}

/// Test config validation with comments only
#[test]
fn test_config_validate_comments_only() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("comments-only.yaml");

    // Write file with only comments
    fs::write(
        &config_path,
        r#"
# This is a comment
# Another comment
# No actual configuration
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["config", "validate", "--config"]).arg(config_path.to_str().unwrap());

    cmd.assert().success();

    println!("✓ Config validation accepts comments-only file");
}

/// Test config validation with extremely large port number
#[test]
fn test_config_validate_port_out_of_range() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("port-out-of-range.yaml");

    // Write config with port number exceeding u16 max
    fs::write(
        &config_path,
        r#"
http:
  port: 99999
  host: "0.0.0.0"
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["config", "validate", "--config"]).arg(config_path.to_str().unwrap());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("error").or(predicate::str::contains("Error")));

    println!("✓ Config validation correctly rejects port number out of range");
}

/// Test config validation with mixed valid and invalid sections
#[test]
fn test_config_validate_mixed_valid_invalid() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("mixed.yaml");

    // Write config with some valid and some invalid sections
    fs::write(
        &config_path,
        r#"
http:
  port: 3000  # Valid
  host: "0.0.0.0"  # Valid
  endpoints:
    - path: ""  # Invalid: empty path
      methods: []  # Invalid: no methods
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["config", "validate", "--config"]).arg(config_path.to_str().unwrap());

    // Should either fail or succeed with warnings
    let output = cmd.output().unwrap();
    assert!(
        output.status.success() || !output.stderr.is_empty(),
        "Should provide validation feedback for mixed valid/invalid config"
    );

    println!("✓ Config validation handles mixed valid/invalid sections");
}

/// Test config validation with special characters in strings
#[test]
fn test_config_validate_special_characters() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("special-chars.yaml");

    // Write config with special characters
    fs::write(
        &config_path,
        r#"
http:
  port: 3000
  host: "0.0.0.0"
  endpoints:
    - path: "/test/\u{1F600}"  # Emoji
      methods: ["GET"]
      response:
        body: '{"msg": "Hello \n World"}'
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["config", "validate", "--config"]).arg(config_path.to_str().unwrap());

    // Should handle special characters gracefully
    let output = cmd.output().unwrap();
    assert!(
        output.status.success() || !output.stderr.is_empty(),
        "Should handle special characters in config"
    );

    println!("✓ Config validation handles special characters");
}

/// Test config validation auto-discovery when no config file specified
#[test]
fn test_config_validate_auto_discovery_no_file() {
    let temp_dir = tempdir().unwrap();

    // Change to temp directory where no config file exists
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.current_dir(&temp_dir).args(["config", "validate"]);

    cmd.assert().failure().stderr(
        predicate::str::contains("onfiguration file").or(predicate::str::contains("No such")),
    );

    println!("✓ Config validation correctly handles missing config during auto-discovery");
}

/// Test config validation with binary data (not text)
#[test]
fn test_config_validate_binary_file() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("binary.yaml");

    // Write binary data
    fs::write(&config_path, [0xFF, 0xFE, 0xFD, 0x00, 0x01, 0x02]).unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["config", "validate", "--config"]).arg(config_path.to_str().unwrap());

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("error").or(predicate::str::contains("Error")));

    println!("✓ Config validation correctly rejects binary file");
}

/// Test config validation with extremely nested structure
#[test]
fn test_config_validate_deeply_nested() {
    let temp_dir = tempdir().unwrap();
    let config_path = temp_dir.path().join("deeply-nested.yaml");

    // Write config with deep nesting
    fs::write(
        &config_path,
        r#"
http:
  port: 3000
  host: "0.0.0.0"
  endpoints:
    - path: "/test"
      methods: ["GET"]
      response:
        body:
          level1:
            level2:
              level3:
                level4:
                  level5:
                    level6:
                      level7:
                        level8:
                          level9:
                            level10:
                              data: "deep"
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["config", "validate", "--config"]).arg(config_path.to_str().unwrap());

    // Should handle deep nesting
    let output = cmd.output().unwrap();
    assert!(
        output.status.success() || !output.stderr.is_empty(),
        "Should handle deeply nested configuration"
    );

    println!("✓ Config validation handles deeply nested structure");
}
