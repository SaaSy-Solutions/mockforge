//! HTTP Fixture Validation Tool
//!
//! Standalone tool for validating HTTP fixtures in a directory.
//! Can validate individual files or entire directories.

use anyhow::{Context, Result};
use mockforge_core::{CustomFixture, CustomFixtureLoader};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Validation result for a single fixture
#[derive(Debug)]
pub struct ValidationResult {
    pub file: PathBuf,
    pub valid: bool,
    pub error: Option<String>,
    pub format: FixtureFormat,
}

/// Format of the fixture
#[derive(Debug, Clone, Copy)]
pub enum FixtureFormat {
    Flat,
    Nested,
    Invalid,
}

/// Validate a single fixture file
pub async fn validate_file(file_path: &Path) -> Result<ValidationResult> {
    let content = fs::read_to_string(file_path)
        .await
        .with_context(|| format!("Failed to read fixture file: {}", file_path.display()))?;

    // Check if it's a template file
    if should_skip_file(&content) {
        return Ok(ValidationResult {
            file: file_path.to_path_buf(),
            valid: false,
            error: Some("Template file (contains _comment, _usage, or scenario field)".to_string()),
            format: FixtureFormat::Invalid,
        });
    }

    // Try to parse as flat format
    match serde_json::from_str::<CustomFixture>(&content) {
        Ok(mut fixture) => {
            // Normalize path
            fixture.path = normalize_path(&fixture.path);

            // Validate
            match validate_fixture(&fixture, file_path) {
                Ok(_) => Ok(ValidationResult {
                    file: file_path.to_path_buf(),
                    valid: true,
                    error: None,
                    format: FixtureFormat::Flat,
                }),
                Err(e) => Ok(ValidationResult {
                    file: file_path.to_path_buf(),
                    valid: false,
                    error: Some(e.to_string()),
                    format: FixtureFormat::Flat,
                }),
            }
        }
        Err(_) => {
            // Try nested format
            match serde_json::from_str::<NestedFixture>(&content) {
                Ok(nested) => match convert_nested_to_flat(nested) {
                    Ok(fixture) => match validate_fixture(&fixture, file_path) {
                        Ok(_) => Ok(ValidationResult {
                            file: file_path.to_path_buf(),
                            valid: true,
                            error: None,
                            format: FixtureFormat::Nested,
                        }),
                        Err(e) => Ok(ValidationResult {
                            file: file_path.to_path_buf(),
                            valid: false,
                            error: Some(e.to_string()),
                            format: FixtureFormat::Nested,
                        }),
                    },
                    Err(e) => Ok(ValidationResult {
                        file: file_path.to_path_buf(),
                        valid: false,
                        error: Some(e.to_string()),
                        format: FixtureFormat::Nested,
                    }),
                },
                Err(e) => Ok(ValidationResult {
                    file: file_path.to_path_buf(),
                    valid: false,
                    error: Some(format!("Invalid JSON or fixture format: {}", e)),
                    format: FixtureFormat::Invalid,
                }),
            }
        }
    }
}

/// Validate all fixtures in a directory
pub async fn validate_directory(dir_path: &Path) -> Result<Vec<ValidationResult>> {
    let mut results = Vec::new();

    if !dir_path.exists() {
        anyhow::bail!("Directory does not exist: {}", dir_path.display());
    }

    if !dir_path.is_dir() {
        anyhow::bail!("Path is not a directory: {}", dir_path.display());
    }

    let mut entries = fs::read_dir(dir_path)
        .await
        .with_context(|| format!("Failed to read directory: {}", dir_path.display()))?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            let result = validate_file(&path).await?;
            results.push(result);
        }
    }

    Ok(results)
}

// Helper functions (duplicated from CustomFixtureLoader for standalone use)
fn should_skip_file(content: &str) -> bool {
    // Check for template indicators
    if content.contains("\"_comment\"") || content.contains("\"_usage\"") {
        return true;
    }

    // Check if it's a scenario/config file (not a fixture)
    if content.contains("\"scenario\"") || content.contains("\"presentation_mode\"") {
        return true;
    }

    false
}

fn normalize_path(path: &str) -> String {
    let mut normalized = path.trim().to_string();

    // Strip query string if present (query strings are handled separately)
    if let Some(query_start) = normalized.find('?') {
        normalized = normalized[..query_start].to_string();
    }

    // Collapse multiple slashes into one
    while normalized.contains("//") {
        normalized = normalized.replace("//", "/");
    }

    // Remove trailing slash (except for root path)
    if normalized.len() > 1 && normalized.ends_with('/') {
        normalized.pop();
    }

    // Ensure path starts with /
    if !normalized.starts_with('/') {
        normalized = format!("/{}", normalized);
    }

    normalized
}

fn validate_fixture(fixture: &CustomFixture, file_path: &Path) -> Result<()> {
    use anyhow::bail;

    // Check required fields
    if fixture.method.is_empty() {
        bail!("method is required and cannot be empty");
    }

    if fixture.path.is_empty() {
        bail!("path is required and cannot be empty");
    }

    // Validate HTTP method
    let method_upper = fixture.method.to_uppercase();
    let valid_methods = [
        "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS", "TRACE",
    ];
    if !valid_methods.contains(&method_upper.as_str()) {
        // Warn but don't fail
    }

    // Validate status code
    if fixture.status < 100 || fixture.status >= 600 {
        bail!("status code {} is not a valid HTTP status code (100-599)", fixture.status);
    }

    Ok(())
}

fn convert_nested_to_flat(nested: NestedFixture) -> Result<CustomFixture> {
    use anyhow::bail;

    let request = nested
        .request
        .ok_or_else(|| anyhow::anyhow!("Nested fixture missing 'request' object"))?;

    let response = nested
        .response
        .ok_or_else(|| anyhow::anyhow!("Nested fixture missing 'response' object"))?;

    Ok(CustomFixture {
        method: request.method,
        path: normalize_path(&request.path),
        status: response.status,
        response: response.body,
        headers: response.headers,
        delay_ms: 0,
    })
}

// Nested fixture types for parsing
#[derive(Debug, Deserialize)]
struct NestedFixture {
    request: Option<NestedRequest>,
    response: Option<NestedResponse>,
}

#[derive(Debug, Deserialize)]
struct NestedRequest {
    method: String,
    path: String,
}

#[derive(Debug, Deserialize)]
struct NestedResponse {
    status: u16,
    #[serde(default)]
    headers: HashMap<String, String>,
    body: Value,
}

/// Print validation results in a formatted way
pub fn print_results(results: &[ValidationResult], verbose: bool) {
    let valid_count = results.iter().filter(|r| r.valid).count();
    let invalid_count = results.len() - valid_count;
    let flat_count = results.iter().filter(|r| matches!(r.format, FixtureFormat::Flat)).count();
    let nested_count = results.iter().filter(|r| matches!(r.format, FixtureFormat::Nested)).count();

    println!("\n📊 Validation Summary");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Total files: {}", results.len());
    println!("✅ Valid: {}", valid_count);
    println!("❌ Invalid: {}", invalid_count);
    println!("📄 Flat format: {}", flat_count);
    println!("📦 Nested format: {}", nested_count);

    if invalid_count > 0 {
        println!("\n❌ Invalid Fixtures:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        for result in results.iter().filter(|r| !r.valid) {
            println!("\n  File: {}", result.file.display());
            if let Some(ref error) = result.error {
                println!("  Error: {}", error);
            }
        }
    }

    if verbose && valid_count > 0 {
        println!("\n✅ Valid Fixtures:");
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        for result in results.iter().filter(|r| r.valid) {
            let format_str = match result.format {
                FixtureFormat::Flat => "flat",
                FixtureFormat::Nested => "nested",
                FixtureFormat::Invalid => "invalid",
            };
            println!("  {} ({})", result.file.display(), format_str);
        }
    }
}
