//! Comprehensive tests for MockForge CLI functionality
//!
//! This module tests the enhanced CLI features including progress bars,
//! error handling, watch mode, and user feedback.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::TempDir;

/// Test the basic CLI help output
#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("MockForge - Comprehensive API Mocking Framework"))
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("serve"))
        .stdout(predicate::str::contains("generate"));
}

/// Test the generate command help
#[test]
fn test_generate_help() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("generate").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Generate mock servers from OpenAPI specifications"))
        .stdout(predicate::str::contains("--watch"))
        .stdout(predicate::str::contains("--progress"))
        .stdout(predicate::str::contains("--verbose"));
}

/// Test the serve command help
#[test]
fn test_serve_help() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("serve").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Start mock servers (HTTP, WebSocket, gRPC)"))
        .stdout(predicate::str::contains("--progress"))
        .stdout(predicate::str::contains("--verbose"));
}

/// Test generate command with dry run
#[test]
fn test_generate_dry_run() {
    let temp_dir = TempDir::new().unwrap();
    let spec_file = temp_dir.path().join("test-spec.yaml");

    // Create a minimal OpenAPI spec
    std::fs::write(
        &spec_file,
        r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /test:
    get:
      responses:
        '200':
          description: OK
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("generate").arg("--spec").arg(&spec_file).arg("--dry-run");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Generating mocks from configuration"));
}

/// Test generate command with progress bar
#[test]
fn test_generate_with_progress() {
    let temp_dir = TempDir::new().unwrap();
    let spec_file = temp_dir.path().join("test-spec.yaml");
    let output_dir = temp_dir.path().join("output");

    // Create a minimal OpenAPI spec
    std::fs::write(
        &spec_file,
        r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /test:
    get:
      responses:
        '200':
          description: OK
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("generate")
        .arg("--spec")
        .arg(&spec_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--progress");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Generating mocks from configuration"));
}

/// Test generate command with verbose output
#[test]
fn test_generate_verbose() {
    let temp_dir = TempDir::new().unwrap();
    let spec_file = temp_dir.path().join("test-spec.yaml");
    let output_dir = temp_dir.path().join("output");

    // Create a minimal OpenAPI spec
    std::fs::write(
        &spec_file,
        r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /test:
    get:
      responses:
        '200':
          description: OK
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("generate")
        .arg("--spec")
        .arg(&spec_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--verbose");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Generating mocks from configuration"));
}

/// Test error handling for missing spec file
#[test]
fn test_generate_missing_spec() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("generate").arg("--spec").arg("nonexistent.yaml");

    cmd.assert()
        .failure()
        .stdout(predicate::str::contains("Specification file not found"));
}

/// Test error handling for invalid output directory
#[test]
fn test_generate_invalid_output() {
    let temp_dir = TempDir::new().unwrap();
    let spec_file = temp_dir.path().join("test-spec.yaml");
    let invalid_output = temp_dir.path().join("invalid-file.txt");

    // Create a minimal OpenAPI spec
    std::fs::write(
        &spec_file,
        r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /test:
    get:
      responses:
        '200':
          description: OK
"#,
    )
    .unwrap();

    // Create a file instead of directory
    std::fs::write(&invalid_output, "test").unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("generate")
        .arg("--spec")
        .arg(&spec_file)
        .arg("--output")
        .arg(&invalid_output);

    cmd.assert().failure().stdout(predicate::str::contains("Error"));
}

/// Test serve command dry run
#[test]
fn test_serve_dry_run() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("serve").arg("--dry-run");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Configuration validation passed"));
}

/// Test serve command with progress indicator
#[test]
fn test_serve_with_progress() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("serve").arg("--dry-run").arg("--progress");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Configuration validation passed"));
}

/// Test serve command with verbose output
#[test]
fn test_serve_verbose() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("serve").arg("--dry-run").arg("--verbose");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Configuration validation passed"));
}

/// Test error handling for invalid port
#[test]
fn test_serve_invalid_port() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("serve").arg("--http-port").arg("99999").arg("--dry-run");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("99999 is not in 0..=65535"));
}

/// Test watch mode (basic validation)
#[test]
fn test_generate_watch_mode_help() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("generate").arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--watch"))
        .stdout(predicate::str::contains("--watch-debounce"));
}

/// Test configuration file discovery
#[test]
fn test_config_discovery() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("mockforge.yaml");

    // Create a minimal spec file
    let spec_file = temp_dir.path().join("test-spec.yaml");
    std::fs::write(
        &spec_file,
        r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths:
  /test:
    get:
      responses:
        '200':
          description: OK
"#,
    )
    .unwrap();

    // Create a minimal config file
    std::fs::write(
        &config_file,
        r#"
http:
  port: 3000
input:
  spec: "./test-spec.yaml"
output:
  path: "./generated"
  clean: true
"#,
    )
    .unwrap();

    // Change to the temp directory
    std::env::set_current_dir(&temp_dir).unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("generate").arg("--dry-run");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Generating mocks from configuration"));
}

/// Test exit codes
#[test]
fn test_exit_codes() {
    // Test success exit code
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("--help");
    cmd.assert().success();

    // Test error exit code
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("generate").arg("--spec").arg("nonexistent.yaml");
    cmd.assert().failure();
}

/// Test progress bar formatting
#[test]
fn test_progress_formatting() {
    use crate::progress::utils;

    assert_eq!(utils::format_file_size(1024), "1.0 KB");
    assert_eq!(utils::format_file_size(1048576), "1.0 MB");
    assert_eq!(utils::format_file_size(512), "512.0 B");

    use std::time::Duration;
    assert_eq!(utils::format_duration(Duration::from_secs(65)), "1m 5s");
    assert_eq!(utils::format_duration(Duration::from_secs(3665)), "1h 1m 5s");
    assert_eq!(utils::format_duration(Duration::from_secs(30)), "30s");
}

/// Test error handling with suggestions
#[test]
fn test_error_suggestions() {
    use crate::progress::{CliError, ExitCode};

    let error = CliError::new("Test error".to_string(), ExitCode::GeneralError)
        .with_suggestion("Test suggestion".to_string());

    assert_eq!(error.message, "Test error");
    assert_eq!(error.exit_code, ExitCode::GeneralError);
    assert_eq!(error.suggestion, Some("Test suggestion".to_string()));
}

/// Test CLI version output
#[test]
fn test_version_output() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("--version");

    cmd.assert()
        .success()
        .stdout(predicate::str::is_match(r"mockforge \d+\.\d+\.\d+").unwrap());
}

/// Test completion generation
#[test]
fn test_completion_generation() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("completions").arg("bash");

    cmd.assert().success().stdout(
        predicate::str::contains("complete -F _mockforge")
            .and(predicate::str::contains("mockforge")),
    );
}

/// Test invalid command handling
#[test]
fn test_invalid_command() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("invalid-command");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

/// Test configuration validation
#[test]
fn test_config_validation() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("invalid-config.yaml");

    // Create an invalid config file
    std::fs::write(
        &config_file,
        r#"
invalid:
  config:
    structure: true
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("serve").arg("--config").arg(&config_file).arg("--dry-run");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Configuration validation passed"));
}

/// Integration test for full generate workflow
#[test]
fn test_full_generate_workflow() {
    let temp_dir = TempDir::new().unwrap();
    let spec_file = temp_dir.path().join("api.yaml");
    let output_dir = temp_dir.path().join("generated");

    // Create a comprehensive OpenAPI spec
    std::fs::write(
        &spec_file,
        r#"
openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
  description: A comprehensive test API
paths:
  /users:
    get:
      summary: Get all users
      responses:
        '200':
          description: List of users
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/User'
    post:
      summary: Create a new user
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/User'
      responses:
        '201':
          description: User created
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/User'
  /users/{id}:
    get:
      summary: Get user by ID
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: integer
      responses:
        '200':
          description: User details
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/User'
components:
  schemas:
    User:
      type: object
      properties:
        id:
          type: integer
        name:
          type: string
        email:
          type: string
      required:
        - id
        - name
        - email
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("generate")
        .arg("--spec")
        .arg(&spec_file)
        .arg("--output")
        .arg(&output_dir)
        .arg("--progress")
        .arg("--verbose");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Generating mocks from configuration"));
}
