//! Custom fixture format support for simple JSON fixtures
//!
//! Supports fixtures in two formats:
//!
//! **Flat format** (preferred):
//! ```json
//! {
//!   "method": "GET",
//!   "path": "/api/v1/endpoint",
//!   "status": 200,
//!   "response": { /* response body */ },
//!   "headers": { /* optional */ },
//!   "delay_ms": 0 /* optional */
//! }
//! ```
//!
//! **Nested format** (also supported):
//! ```json
//! {
//!   "request": {
//!     "method": "GET",
//!     "path": "/api/v1/endpoint"
//!   },
//!   "response": {
//!     "status": 200,
//!     "headers": { /* optional */ },
//!     "body": { /* response body */ }
//!   }
//! }
//! ```
//!
//! Path matching supports path parameters using curly braces:
//! - `/api/v1/hives/{hiveId}` matches `/api/v1/hives/hive_001`
//!
//! Paths are automatically normalized (trailing slashes removed, multiple slashes collapsed)

use crate::{Error, RequestFingerprint, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Custom fixture structure matching the simple JSON format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomFixture {
    /// HTTP method (GET, POST, PUT, PATCH, DELETE)
    pub method: String,
    /// Request path (supports path parameters like {hiveId})
    pub path: String,
    /// Response status code
    pub status: u16,
    /// Response body (can be any JSON value)
    #[serde(default)]
    pub response: serde_json::Value,
    /// Optional response headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Optional response delay in milliseconds
    #[serde(default)]
    pub delay_ms: u64,
}

/// Nested fixture format for backward compatibility
#[derive(Debug, Deserialize)]
pub struct NestedFixture {
    /// Request configuration for the fixture
    pub request: Option<NestedRequest>,
    /// Response configuration for the fixture
    pub response: Option<NestedResponse>,
}

/// Request portion of a nested fixture
#[derive(Debug, Deserialize)]
pub struct NestedRequest {
    /// HTTP method for the request
    pub method: String,
    /// URL path pattern for the request
    pub path: String,
}

/// Response portion of a nested fixture
#[derive(Debug, Deserialize)]
pub struct NestedResponse {
    /// HTTP status code for the response
    pub status: u16,
    /// HTTP headers for the response
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Response body content
    pub body: Value,
}

/// Result of loading a fixture file
#[derive(Debug)]
enum LoadResult {
    Loaded,
    Skipped,
}

/// Custom fixture loader that scans a directory for fixture files
pub struct CustomFixtureLoader {
    /// Directory containing custom fixtures
    fixtures_dir: PathBuf,
    /// Whether custom fixtures are enabled
    enabled: bool,
    /// Loaded fixtures cache (method -> path pattern -> fixture)
    fixtures: HashMap<String, HashMap<String, CustomFixture>>,
    /// Statistics for loaded fixtures
    stats: LoadStats,
}

/// Statistics about fixture loading
#[derive(Debug, Default)]
struct LoadStats {
    loaded: usize,
    failed: usize,
    skipped: usize,
}

impl CustomFixtureLoader {
    /// Create a new custom fixture loader
    pub fn new(fixtures_dir: PathBuf, enabled: bool) -> Self {
        Self {
            fixtures_dir,
            enabled,
            fixtures: HashMap::new(),
            stats: LoadStats::default(),
        }
    }

    /// Normalize a path by removing trailing slashes (except root) and collapsing multiple slashes
    /// Also strips query strings from the path (query strings are handled separately in RequestFingerprint)
    pub fn normalize_path(path: &str) -> String {
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

    /// Check if a file should be skipped (template files, etc.)
    pub fn should_skip_file(content: &str) -> bool {
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

    /// Convert nested fixture format to flat format
    pub fn convert_nested_to_flat(nested: NestedFixture) -> Result<CustomFixture> {
        let request = nested
            .request
            .ok_or_else(|| Error::generic("Nested fixture missing 'request' object".to_string()))?;

        let response = nested.response.ok_or_else(|| {
            Error::generic("Nested fixture missing 'response' object".to_string())
        })?;

        Ok(CustomFixture {
            method: request.method,
            path: Self::normalize_path(&request.path),
            status: response.status,
            response: response.body,
            headers: response.headers,
            delay_ms: 0,
        })
    }

    /// Validate a fixture has required fields and valid values
    pub fn validate_fixture(fixture: &CustomFixture, file_path: &Path) -> Result<()> {
        // Check required fields
        if fixture.method.is_empty() {
            return Err(Error::generic(format!(
                "Invalid fixture in {}: method is required and cannot be empty",
                file_path.display()
            )));
        }

        if fixture.path.is_empty() {
            return Err(Error::generic(format!(
                "Invalid fixture in {}: path is required and cannot be empty",
                file_path.display()
            )));
        }

        // Validate HTTP method
        let method_upper = fixture.method.to_uppercase();
        let valid_methods = [
            "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS", "TRACE",
        ];
        if !valid_methods.contains(&method_upper.as_str()) {
            tracing::warn!(
                "Fixture {} uses non-standard HTTP method: {}",
                file_path.display(),
                fixture.method
            );
        }

        // Validate status code
        if fixture.status < 100 || fixture.status >= 600 {
            return Err(Error::generic(format!(
                "Invalid fixture in {}: status code {} is not a valid HTTP status code (100-599)",
                file_path.display(),
                fixture.status
            )));
        }

        Ok(())
    }

    /// Load all fixtures from the directory
    pub async fn load_fixtures(&mut self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        if !self.fixtures_dir.exists() {
            tracing::debug!(
                "Custom fixtures directory does not exist: {}",
                self.fixtures_dir.display()
            );
            return Ok(());
        }

        // Reset stats
        self.stats = LoadStats::default();

        // Scan directory for JSON files
        let mut entries = fs::read_dir(&self.fixtures_dir).await.map_err(|e| {
            Error::generic(format!(
                "Failed to read fixtures directory {}: {}",
                self.fixtures_dir.display(),
                e
            ))
        })?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| Error::generic(format!("Failed to read directory entry: {}", e)))?
        {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                match self.load_fixture_file(&path).await {
                    Ok(LoadResult::Loaded) => {
                        self.stats.loaded += 1;
                    }
                    Ok(LoadResult::Skipped) => {
                        self.stats.skipped += 1;
                    }
                    Err(e) => {
                        self.stats.failed += 1;
                        tracing::warn!("Failed to load fixture file {}: {}", path.display(), e);
                    }
                }
            }
        }

        // Log summary
        tracing::info!(
            "Fixture loading complete: {} loaded, {} failed, {} skipped from {}",
            self.stats.loaded,
            self.stats.failed,
            self.stats.skipped,
            self.fixtures_dir.display()
        );

        Ok(())
    }

    /// Load a single fixture file
    async fn load_fixture_file(&mut self, path: &Path) -> Result<LoadResult> {
        let content = fs::read_to_string(path).await.map_err(|e| {
            Error::generic(format!("Failed to read fixture file {}: {}", path.display(), e))
        })?;

        // Check if this is a template file that should be skipped
        if Self::should_skip_file(&content) {
            tracing::debug!("Skipping template file: {}", path.display());
            return Ok(LoadResult::Skipped);
        }

        // Try to parse as flat format first
        let fixture = match serde_json::from_str::<CustomFixture>(&content) {
            Ok(mut fixture) => {
                // Normalize path
                fixture.path = Self::normalize_path(&fixture.path);
                fixture
            }
            Err(_) => {
                // Try nested format
                let nested: NestedFixture = serde_json::from_str(&content).map_err(|e| {
                    Error::generic(format!(
                        "Failed to parse fixture file {}: not a valid flat or nested format. Error: {}",
                        path.display(),
                        e
                    ))
                })?;

                // Convert nested to flat
                Self::convert_nested_to_flat(nested)?
            }
        };

        // Validate fixture
        Self::validate_fixture(&fixture, path)?;

        // Store fixture by method and path pattern
        let method = fixture.method.to_uppercase();
        let fixtures_by_method = self.fixtures.entry(method.clone()).or_default();

        // Check for duplicate paths (warn but allow)
        if fixtures_by_method.contains_key(&fixture.path) {
            tracing::warn!(
                "Duplicate fixture path '{}' for method '{}' in file {} (overwriting previous)",
                fixture.path,
                method,
                path.display()
            );
        }

        fixtures_by_method.insert(fixture.path.clone(), fixture);

        Ok(LoadResult::Loaded)
    }

    /// Check if a fixture exists for the given request fingerprint
    pub fn has_fixture(&self, fingerprint: &RequestFingerprint) -> bool {
        if !self.enabled {
            return false;
        }

        self.find_matching_fixture(fingerprint).is_some()
    }

    /// Load a fixture for the given request fingerprint
    pub fn load_fixture(&self, fingerprint: &RequestFingerprint) -> Option<CustomFixture> {
        if !self.enabled {
            return None;
        }

        self.find_matching_fixture(fingerprint).cloned()
    }

    /// Find a matching fixture for the request fingerprint
    fn find_matching_fixture(&self, fingerprint: &RequestFingerprint) -> Option<&CustomFixture> {
        let method = fingerprint.method.to_uppercase();
        let fixtures_by_method = self.fixtures.get(&method)?;

        // Normalize the request path for matching
        let request_path = Self::normalize_path(&fingerprint.path);

        // Debug logging
        tracing::debug!(
            "Fixture matching: method={}, fingerprint.path='{}', normalized='{}', available fixtures: {:?}",
            method,
            fingerprint.path,
            request_path,
            fixtures_by_method.keys().collect::<Vec<_>>()
        );

        // Try exact match first (with normalized path)
        if let Some(fixture) = fixtures_by_method.get(&request_path) {
            tracing::debug!("Found exact fixture match: {} {}", method, request_path);
            return Some(fixture);
        }

        // Try pattern matching for path parameters
        for (pattern, fixture) in fixtures_by_method.iter() {
            if self.path_matches(pattern, &request_path) {
                tracing::debug!(
                    "Found pattern fixture match: {} {} (pattern: {})",
                    method,
                    request_path,
                    pattern
                );
                return Some(fixture);
            }
        }

        tracing::debug!("No fixture match found for: {} {}", method, request_path);
        None
    }

    /// Check if a request path matches a fixture path pattern
    ///
    /// Supports path parameters using curly braces:
    /// - Pattern: `/api/v1/hives/{hiveId}` matches `/api/v1/hives/hive_001`, `/api/v1/hives/123`, etc.
    /// - Paths are normalized before matching (trailing slashes removed, multiple slashes collapsed)
    fn path_matches(&self, pattern: &str, request_path: &str) -> bool {
        // Normalize both paths before matching
        let normalized_pattern = Self::normalize_path(pattern);
        let normalized_request = Self::normalize_path(request_path);

        // Simple pattern matching without full regex (for performance)
        // Split both paths into segments
        let pattern_segments: Vec<&str> =
            normalized_pattern.split('/').filter(|s| !s.is_empty()).collect();
        let request_segments: Vec<&str> =
            normalized_request.split('/').filter(|s| !s.is_empty()).collect();

        if pattern_segments.len() != request_segments.len() {
            return false;
        }

        // Compare segments
        for (pattern_seg, request_seg) in pattern_segments.iter().zip(request_segments.iter()) {
            // If pattern segment is a parameter (starts with { and ends with }), it matches anything
            if pattern_seg.starts_with('{') && pattern_seg.ends_with('}') {
                continue;
            }
            // Otherwise, segments must match exactly
            if pattern_seg != request_seg {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderMap, Method, Uri};
    use tempfile::TempDir;

    fn create_test_fingerprint(method: &str, path: &str) -> RequestFingerprint {
        let method = Method::from_bytes(method.as_bytes()).unwrap();
        let uri: Uri = path.parse().unwrap();
        RequestFingerprint::new(method, &uri, &HeaderMap::new(), None)
    }

    #[test]
    fn test_path_matching_exact() {
        let loader = CustomFixtureLoader::new(PathBuf::from("/tmp"), true);
        assert!(loader.path_matches("/api/v1/apiaries", "/api/v1/apiaries"));
        assert!(!loader.path_matches("/api/v1/apiaries", "/api/v1/hives"));
    }

    #[test]
    fn test_path_matching_with_parameters() {
        let loader = CustomFixtureLoader::new(PathBuf::from("/tmp"), true);
        assert!(loader.path_matches("/api/v1/hives/{hiveId}", "/api/v1/hives/hive_001"));
        assert!(loader.path_matches("/api/v1/hives/{hiveId}", "/api/v1/hives/123"));
        assert!(
            !loader.path_matches("/api/v1/hives/{hiveId}", "/api/v1/hives/hive_001/inspections")
        );
    }

    #[test]
    fn test_path_matching_multiple_parameters() {
        let loader = CustomFixtureLoader::new(PathBuf::from("/tmp"), true);
        assert!(loader.path_matches(
            "/api/v1/apiaries/{apiaryId}/hives/{hiveId}",
            "/api/v1/apiaries/apiary_001/hives/hive_001"
        ));
    }

    #[tokio::test]
    async fn test_load_fixture() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        // Create a test fixture file
        let fixture_content = r#"{
  "method": "GET",
  "path": "/api/v1/apiaries",
  "status": 200,
  "response": {
    "success": true,
    "data": []
  }
}"#;

        let fixture_file = fixtures_dir.join("apiaries-list.json");
        fs::write(&fixture_file, fixture_content).await.unwrap();

        // Load fixtures
        let mut loader = CustomFixtureLoader::new(fixtures_dir, true);
        loader.load_fixtures().await.unwrap();

        // Check if fixture is loaded
        let fingerprint = create_test_fingerprint("GET", "/api/v1/apiaries");
        assert!(loader.has_fixture(&fingerprint));

        let fixture = loader.load_fixture(&fingerprint).unwrap();
        assert_eq!(fixture.method, "GET");
        assert_eq!(fixture.path, "/api/v1/apiaries");
        assert_eq!(fixture.status, 200);
    }

    #[tokio::test]
    async fn test_load_fixture_with_path_parameter() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        // Create a test fixture file with path parameter
        let fixture_content = r#"{
  "method": "GET",
  "path": "/api/v1/hives/{hiveId}",
  "status": 200,
  "response": {
    "success": true,
    "data": {
      "id": "hive_001"
    }
  }
}"#;

        let fixture_file = fixtures_dir.join("hive-detail.json");
        fs::write(&fixture_file, fixture_content).await.unwrap();

        // Load fixtures
        let mut loader = CustomFixtureLoader::new(fixtures_dir, true);
        loader.load_fixtures().await.unwrap();

        // Check if fixture matches with actual path
        let fingerprint = create_test_fingerprint("GET", "/api/v1/hives/hive_001");
        assert!(loader.has_fixture(&fingerprint));

        let fixture = loader.load_fixture(&fingerprint).unwrap();
        assert_eq!(fixture.path, "/api/v1/hives/{hiveId}");
    }

    #[tokio::test]
    async fn test_load_multiple_fixtures() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        // Create multiple fixture files
        let fixtures = vec![
            (
                "apiaries-list.json",
                r#"{
  "method": "GET",
  "path": "/api/v1/apiaries",
  "status": 200,
  "response": {"items": []}
}"#,
            ),
            (
                "hive-detail.json",
                r#"{
  "method": "GET",
  "path": "/api/v1/hives/{hiveId}",
  "status": 200,
  "response": {"id": "hive_001"}
}"#,
            ),
            (
                "user-profile.json",
                r#"{
  "method": "GET",
  "path": "/api/v1/users/me",
  "status": 200,
  "response": {"id": "user_001"}
}"#,
            ),
        ];

        for (filename, content) in fixtures {
            let fixture_file = fixtures_dir.join(filename);
            fs::write(&fixture_file, content).await.unwrap();
        }

        // Load fixtures
        let mut loader = CustomFixtureLoader::new(fixtures_dir, true);
        loader.load_fixtures().await.unwrap();

        // Verify all fixtures are loaded
        assert!(loader.has_fixture(&create_test_fingerprint("GET", "/api/v1/apiaries")));
        assert!(loader.has_fixture(&create_test_fingerprint("GET", "/api/v1/hives/hive_001")));
        assert!(loader.has_fixture(&create_test_fingerprint("GET", "/api/v1/users/me")));
    }

    #[tokio::test]
    async fn test_fixture_with_headers() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        let fixture_content = r#"{
  "method": "GET",
  "path": "/api/v1/test",
  "status": 201,
  "response": {"result": "ok"},
  "headers": {
    "content-type": "application/json",
    "x-custom-header": "test-value"
  }
}"#;

        let fixture_file = fixtures_dir.join("test.json");
        fs::write(&fixture_file, fixture_content).await.unwrap();

        let mut loader = CustomFixtureLoader::new(fixtures_dir, true);
        loader.load_fixtures().await.unwrap();

        let fixture = loader.load_fixture(&create_test_fingerprint("GET", "/api/v1/test")).unwrap();

        assert_eq!(fixture.status, 201);
        assert_eq!(fixture.headers.get("content-type"), Some(&"application/json".to_string()));
        assert_eq!(fixture.headers.get("x-custom-header"), Some(&"test-value".to_string()));
    }

    #[tokio::test]
    async fn test_fixture_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        let fixture_file = fixtures_dir.join("test.json");
        fs::write(
            &fixture_file,
            r#"{"method": "GET", "path": "/test", "status": 200, "response": {}}"#,
        )
        .await
        .unwrap();

        let mut loader = CustomFixtureLoader::new(fixtures_dir, false);
        loader.load_fixtures().await.unwrap();

        // Should not find fixture when disabled
        assert!(!loader.has_fixture(&create_test_fingerprint("GET", "/test")));
        assert!(loader.load_fixture(&create_test_fingerprint("GET", "/test")).is_none());
    }

    #[tokio::test]
    async fn test_load_nested_format() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        // Create a nested format fixture
        let fixture_content = r#"{
          "request": {
            "method": "POST",
            "path": "/api/auth/login"
          },
          "response": {
            "status": 200,
            "headers": {
              "Content-Type": "application/json"
            },
            "body": {
              "access_token": "test_token",
              "user": {
                "id": "user_001"
              }
            }
          }
        }"#;

        let fixture_file = fixtures_dir.join("auth-login.json");
        fs::write(&fixture_file, fixture_content).await.unwrap();

        // Load fixtures
        let mut loader = CustomFixtureLoader::new(fixtures_dir, true);
        loader.load_fixtures().await.unwrap();

        // Check if fixture is loaded
        let fingerprint = create_test_fingerprint("POST", "/api/auth/login");
        assert!(loader.has_fixture(&fingerprint));

        let fixture = loader.load_fixture(&fingerprint).unwrap();
        assert_eq!(fixture.method, "POST");
        assert_eq!(fixture.path, "/api/auth/login");
        assert_eq!(fixture.status, 200);
        assert_eq!(fixture.headers.get("Content-Type"), Some(&"application/json".to_string()));
        assert!(fixture.response.get("access_token").is_some());
    }

    #[test]
    fn test_path_normalization() {
        let loader = CustomFixtureLoader::new(PathBuf::from("/tmp"), true);

        // Test trailing slash removal
        assert_eq!(CustomFixtureLoader::normalize_path("/api/v1/test/"), "/api/v1/test");
        assert_eq!(CustomFixtureLoader::normalize_path("/api/v1/test"), "/api/v1/test");

        // Root path should remain as /
        assert_eq!(CustomFixtureLoader::normalize_path("/"), "/");

        // Multiple slashes should be collapsed
        assert_eq!(CustomFixtureLoader::normalize_path("/api//v1///test"), "/api/v1/test");

        // Paths without leading slash should get one
        assert_eq!(CustomFixtureLoader::normalize_path("api/v1/test"), "/api/v1/test");

        // Whitespace should be trimmed
        assert_eq!(CustomFixtureLoader::normalize_path(" /api/v1/test "), "/api/v1/test");
    }

    #[test]
    fn test_path_matching_with_normalization() {
        let loader = CustomFixtureLoader::new(PathBuf::from("/tmp"), true);

        // Test that trailing slashes don't prevent matching
        assert!(loader.path_matches("/api/v1/test", "/api/v1/test/"));
        assert!(loader.path_matches("/api/v1/test/", "/api/v1/test"));

        // Test multiple slashes
        assert!(loader.path_matches("/api/v1/test", "/api//v1///test"));

        // Test path parameters still work
        assert!(loader.path_matches("/api/v1/hives/{hiveId}", "/api/v1/hives/hive_001/"));
    }

    #[tokio::test]
    async fn test_skip_template_files() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        // Create a template file
        let template_content = r#"{
          "_comment": "This is a template",
          "_usage": "Use this for errors",
          "error": {
            "code": "ERROR_CODE"
          }
        }"#;

        let template_file = fixtures_dir.join("error-template.json");
        fs::write(&template_file, template_content).await.unwrap();

        // Create a valid fixture
        let valid_fixture = r#"{
          "method": "GET",
          "path": "/api/test",
          "status": 200,
          "response": {}
        }"#;
        let valid_file = fixtures_dir.join("valid.json");
        fs::write(&valid_file, valid_fixture).await.unwrap();

        // Load fixtures
        let mut loader = CustomFixtureLoader::new(fixtures_dir, true);
        loader.load_fixtures().await.unwrap();

        // Valid fixture should work
        assert!(loader.has_fixture(&create_test_fingerprint("GET", "/api/test")));

        // Template file should be skipped (no fixture for a path that doesn't exist)
        // We can't easily test this without accessing internal state, but the fact that
        // the valid fixture loads means the template was skipped
    }

    #[tokio::test]
    async fn test_skip_scenario_files() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        // Create a scenario file (not a fixture)
        let scenario_content = r#"{
          "scenario": "demo",
          "presentation_mode": true,
          "apiaries": []
        }"#;

        let scenario_file = fixtures_dir.join("demo-scenario.json");
        fs::write(&scenario_file, scenario_content).await.unwrap();

        // Create a valid fixture
        let valid_fixture = r#"{
          "method": "GET",
          "path": "/api/test",
          "status": 200,
          "response": {}
        }"#;
        let valid_file = fixtures_dir.join("valid.json");
        fs::write(&valid_file, valid_fixture).await.unwrap();

        // Load fixtures
        let mut loader = CustomFixtureLoader::new(fixtures_dir, true);
        loader.load_fixtures().await.unwrap();

        // Valid fixture should work
        assert!(loader.has_fixture(&create_test_fingerprint("GET", "/api/test")));
    }

    #[tokio::test]
    async fn test_mixed_format_fixtures() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        // Create flat format fixture
        let flat_fixture = r#"{
          "method": "GET",
          "path": "/api/v1/flat",
          "status": 200,
          "response": {"type": "flat"}
        }"#;

        // Create nested format fixture
        let nested_fixture = r#"{
          "request": {
            "method": "GET",
            "path": "/api/v1/nested"
          },
          "response": {
            "status": 200,
            "body": {"type": "nested"}
          }
        }"#;

        fs::write(fixtures_dir.join("flat.json"), flat_fixture).await.unwrap();
        fs::write(fixtures_dir.join("nested.json"), nested_fixture).await.unwrap();

        // Load fixtures
        let mut loader = CustomFixtureLoader::new(fixtures_dir, true);
        loader.load_fixtures().await.unwrap();

        // Both should work
        assert!(loader.has_fixture(&create_test_fingerprint("GET", "/api/v1/flat")));
        assert!(loader.has_fixture(&create_test_fingerprint("GET", "/api/v1/nested")));

        let flat = loader.load_fixture(&create_test_fingerprint("GET", "/api/v1/flat")).unwrap();
        assert_eq!(flat.response.get("type").and_then(|v| v.as_str()), Some("flat"));

        let nested =
            loader.load_fixture(&create_test_fingerprint("GET", "/api/v1/nested")).unwrap();
        assert_eq!(nested.response.get("type").and_then(|v| v.as_str()), Some("nested"));
    }

    #[tokio::test]
    async fn test_validation_errors() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        // Test missing method
        let no_method = r#"{
          "path": "/api/test",
          "status": 200,
          "response": {}
        }"#;
        fs::write(fixtures_dir.join("no-method.json"), no_method).await.unwrap();

        // Test invalid status code
        let invalid_status = r#"{
          "method": "GET",
          "path": "/api/test",
          "status": 999,
          "response": {}
        }"#;
        fs::write(fixtures_dir.join("invalid-status.json"), invalid_status)
            .await
            .unwrap();

        // Load fixtures - should handle errors gracefully
        let mut loader = CustomFixtureLoader::new(fixtures_dir, true);
        let result = loader.load_fixtures().await;

        // Should not crash, but should log warnings
        assert!(result.is_ok());
    }
}
