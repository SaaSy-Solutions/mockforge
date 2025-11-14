//! Custom fixture format support for simple JSON fixtures
//!
//! Supports fixtures in the format:
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
//! Path matching supports path parameters using curly braces:
//! - `/api/v1/hives/{hiveId}` matches `/api/v1/hives/hive_001`

use crate::{Error, RequestFingerprint, Result};
use axum::http::Method;
use serde::{Deserialize, Serialize};
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

/// Custom fixture loader that scans a directory for fixture files
pub struct CustomFixtureLoader {
    /// Directory containing custom fixtures
    fixtures_dir: PathBuf,
    /// Whether custom fixtures are enabled
    enabled: bool,
    /// Loaded fixtures cache (method -> path pattern -> fixture)
    fixtures: HashMap<String, HashMap<String, CustomFixture>>,
}

impl CustomFixtureLoader {
    /// Create a new custom fixture loader
    pub fn new(fixtures_dir: PathBuf, enabled: bool) -> Self {
        Self {
            fixtures_dir,
            enabled,
            fixtures: HashMap::new(),
        }
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

        // Scan directory for JSON files
        let mut entries = fs::read_dir(&self.fixtures_dir).await.map_err(|e| {
            Error::generic(format!(
                "Failed to read fixtures directory {}: {}",
                self.fixtures_dir.display(),
                e
            ))
        })?;

        let mut loaded_count = 0;
        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            Error::generic(format!(
                "Failed to read directory entry: {}",
                e
            ))
        })? {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "json") {
                if let Err(e) = self.load_fixture_file(&path).await {
                    tracing::warn!(
                        "Failed to load fixture file {}: {}",
                        path.display(),
                        e
                    );
                } else {
                    loaded_count += 1;
                }
            }
        }

        tracing::info!(
            "Loaded {} custom fixtures from {}",
            loaded_count,
            self.fixtures_dir.display()
        );

        Ok(())
    }

    /// Load a single fixture file
    async fn load_fixture_file(&mut self, path: &Path) -> Result<()> {
        let content = fs::read_to_string(path).await.map_err(|e| {
            Error::generic(format!("Failed to read fixture file {}: {}", path.display(), e))
        })?;

        let fixture: CustomFixture = serde_json::from_str(&content).map_err(|e| {
            Error::generic(format!(
                "Failed to parse fixture file {}: {}",
                path.display(),
                e
            ))
        })?;

        // Validate fixture
        if fixture.method.is_empty() || fixture.path.is_empty() {
            return Err(Error::generic(format!(
                "Invalid fixture in {}: method and path are required",
                path.display()
            )));
        }

        // Store fixture by method and path pattern
        let method = fixture.method.to_uppercase();
        let fixtures_by_method = self.fixtures.entry(method).or_insert_with(HashMap::new);
        fixtures_by_method.insert(fixture.path.clone(), fixture);

        Ok(())
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

        let request_path = &fingerprint.path;

        // Try exact match first
        if let Some(fixture) = fixtures_by_method.get(request_path) {
            return Some(fixture);
        }

        // Try pattern matching for path parameters
        for (pattern, fixture) in fixtures_by_method.iter() {
            if self.path_matches(pattern, request_path) {
                return Some(fixture);
            }
        }

        None
    }

    /// Check if a request path matches a fixture path pattern
    ///
    /// Supports path parameters using curly braces:
    /// - Pattern: `/api/v1/hives/{hiveId}`
    /// - Matches: `/api/v1/hives/hive_001`, `/api/v1/hives/123`, etc.
    fn path_matches(&self, pattern: &str, request_path: &str) -> bool {
        // Simple pattern matching without full regex (for performance)
        // Split both paths into segments
        let pattern_segments: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty()).collect();
        let request_segments: Vec<&str> = request_path.split('/').filter(|s| !s.is_empty()).collect();

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
    use axum::http::{HeaderMap, Uri};
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
        assert!(loader.path_matches(
            "/api/v1/hives/{hiveId}",
            "/api/v1/hives/hive_001"
        ));
        assert!(loader.path_matches(
            "/api/v1/hives/{hiveId}",
            "/api/v1/hives/123"
        ));
        assert!(!loader.path_matches(
            "/api/v1/hives/{hiveId}",
            "/api/v1/hives/hive_001/inspections"
        ));
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
            ("apiaries-list.json", r#"{
  "method": "GET",
  "path": "/api/v1/apiaries",
  "status": 200,
  "response": {"items": []}
}"#),
            ("hive-detail.json", r#"{
  "method": "GET",
  "path": "/api/v1/hives/{hiveId}",
  "status": 200,
  "response": {"id": "hive_001"}
}"#),
            ("user-profile.json", r#"{
  "method": "GET",
  "path": "/api/v1/users/me",
  "status": 200,
  "response": {"id": "user_001"}
}"#),
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

        let fixture = loader
            .load_fixture(&create_test_fingerprint("GET", "/api/v1/test"))
            .unwrap();

        assert_eq!(fixture.status, 201);
        assert_eq!(fixture.headers.get("content-type"), Some(&"application/json".to_string()));
        assert_eq!(fixture.headers.get("x-custom-header"), Some(&"test-value".to_string()));
    }

    #[tokio::test]
    async fn test_fixture_disabled() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        let fixture_file = fixtures_dir.join("test.json");
        fs::write(&fixture_file, r#"{"method": "GET", "path": "/test", "status": 200, "response": {}}"#)
            .await
            .unwrap();

        let mut loader = CustomFixtureLoader::new(fixtures_dir, false);
        loader.load_fixtures().await.unwrap();

        // Should not find fixture when disabled
        assert!(!loader.has_fixture(&create_test_fingerprint("GET", "/test")));
        assert!(loader.load_fixture(&create_test_fingerprint("GET", "/test")).is_none());
    }
}
