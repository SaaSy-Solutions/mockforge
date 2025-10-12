use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

/// Test CLI binary exists and can be executed
#[test]
fn test_cli_binary_exists() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("--help");
    cmd.assert().success().stdout(predicate::str::contains("MockForge"));
}

/// Test version flag
#[test]
fn test_version_flag() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("--version");
    cmd.assert().success().stdout(predicate::str::contains("mockforge"));
}

/// Test help shows available commands
#[test]
fn test_help_shows_commands() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("serve"))
        .stdout(predicate::str::contains("data"))
        .stdout(predicate::str::contains("admin"));
}

/// Test data subcommand help
#[test]
fn test_data_help() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["data", "--help"]);
    cmd.assert().success().stdout(predicate::str::contains("template"));
}

/// Test serve subcommand help
#[test]
fn test_serve_help() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["serve", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("http-port"))
        .stdout(predicate::str::contains("ws-port"))
        .stdout(predicate::str::contains("grpc-port"));
}

/// Test admin subcommand help
#[test]
fn test_admin_help() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["admin", "--help"]);
    cmd.assert().success().stdout(predicate::str::contains("port"));
}

/// Test invalid command shows error
#[test]
fn test_invalid_command() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("invalid-command");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

/// Test data template with invalid template name
#[test]
fn test_data_template_invalid() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["data", "template", "nonexistent"]);
    cmd.assert().failure();
}

/// Test data template with valid template
#[test]
fn test_data_template_user() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["data", "template", "user", "--rows", "1"]);
    cmd.assert().success();
}

/// Test serve help contains expected options
#[test]
fn test_serve_options() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["serve", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("config"))
        .stdout(predicate::str::contains("http-port"));
}

/// Test admin with custom port
#[test]
fn test_admin_custom_port() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["admin", "--port", "9999", "--help"]);
    cmd.assert().success();
}

/// Test sync help
#[test]
fn test_sync_help() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["sync", "--help"]);
    cmd.assert().success().stdout(predicate::str::contains("workspace-dir"));
}

/// Test data template with different formats
#[test]
fn test_data_template_formats() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["data", "template", "user", "--rows", "1", "--format", "csv"]);
    cmd.assert().success();
}

/// Test serve with invalid port argument validation
#[test]
fn test_serve_port_validation() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["serve", "--http-port", "abc"]);
    cmd.assert().failure();
}

/// Test data template help
#[test]
fn test_data_template_help() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["data", "template", "--help"]);
    cmd.assert().success().stdout(predicate::str::contains("template"));
}

/// Test admin command starts successfully
#[test]
fn test_admin_startup() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.args(["admin", "--help"]);
    cmd.assert().success().stdout(predicate::str::contains("Admin UI"));
}

/// Test that all main commands can show help
#[test]
fn test_all_commands_help() {
    let commands = ["serve", "data", "admin", "sync"];
    for command in commands {
        let mut cmd = Command::cargo_bin("mockforge").unwrap();
        cmd.args([command, "--help"]);
        cmd.assert().success();
    }
}

/// Test that CLI shows version
#[test]
fn test_version_output_format() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("--version");
    cmd.assert().success().stdout(predicate::str::contains("mockforge"));
}

/// Test that invalid commands show proper error
#[test]
fn test_proper_error_message() {
    let mut cmd = Command::cargo_bin("mockforge").unwrap();
    cmd.arg("nonexistent-command");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("unrecognized subcommand"));
}
