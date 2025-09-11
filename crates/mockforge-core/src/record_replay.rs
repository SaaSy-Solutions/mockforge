//! Record and replay functionality for HTTP requests and responses
//! Implements the Replay and Record parts of the priority chain.

use crate::{RequestFingerprint, Error, Result};
use axum::http::{HeaderMap, Method};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Recorded request/response pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedRequest {
    /// Request fingerprint
    pub fingerprint: RequestFingerprint,
    /// Request timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Response status code
    pub status_code: u16,
    /// Response headers
    pub response_headers: HashMap<String, String>,
    /// Response body
    pub response_body: String,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Replay handler for serving recorded requests
pub struct ReplayHandler {
    /// Directory containing recorded fixtures
    fixtures_dir: PathBuf,
    /// Whether replay is enabled
    enabled: bool,
}

impl ReplayHandler {
    /// Create a new replay handler
    pub fn new(fixtures_dir: PathBuf, enabled: bool) -> Self {
        Self {
            fixtures_dir,
            enabled,
        }
    }

    /// Get the fixture path for a request fingerprint
    fn get_fixture_path(&self, fingerprint: &RequestFingerprint) -> PathBuf {
        let hash = fingerprint.to_hash();
        let method = fingerprint.method.to_lowercase();
        let path_hash = fingerprint.path.replace('/', "_").replace(':', "_");

        self.fixtures_dir
            .join("http")
            .join(&method)
            .join(&path_hash)
            .join(format!("{}.json", hash))
    }

    /// Check if a fixture exists for the given fingerprint
    pub async fn has_fixture(&self, fingerprint: &RequestFingerprint) -> bool {
        if !self.enabled {
            return false;
        }

        let fixture_path = self.get_fixture_path(fingerprint);
        fixture_path.exists()
    }

    /// Load a recorded request from fixture
    pub async fn load_fixture(&self, fingerprint: &RequestFingerprint) -> Result<Option<RecordedRequest>> {
        if !self.enabled {
            return Ok(None);
        }

        let fixture_path = self.get_fixture_path(fingerprint);

        if !fixture_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&fixture_path).await
            .map_err(|e| Error::generic(format!("Failed to read fixture {}: {}", fixture_path.display(), e)))?;

        let recorded_request: RecordedRequest = serde_json::from_str(&content)
            .map_err(|e| Error::generic(format!("Failed to parse fixture {}: {}", fixture_path.display(), e)))?;

        Ok(Some(recorded_request))
    }
}

/// Record handler for saving requests and responses
pub struct RecordHandler {
    /// Directory to save recorded fixtures
    fixtures_dir: PathBuf,
    /// Whether recording is enabled
    enabled: bool,
    /// Whether to record only GET requests
    record_get_only: bool,
}

impl RecordHandler {
    /// Create a new record handler
    pub fn new(fixtures_dir: PathBuf, enabled: bool, record_get_only: bool) -> Self {
        Self {
            fixtures_dir,
            enabled,
            record_get_only,
        }
    }

    /// Check if a request should be recorded
    pub fn should_record(&self, method: &Method) -> bool {
        if !self.enabled {
            return false;
        }

        if self.record_get_only {
            method == Method::GET
        } else {
            true
        }
    }

    /// Record a request and response
    pub async fn record_request(
        &self,
        fingerprint: &RequestFingerprint,
        status_code: u16,
        response_headers: &HeaderMap,
        response_body: &str,
        metadata: Option<HashMap<String, String>>,
    ) -> Result<()> {
        if !self.should_record(&Method::from_bytes(fingerprint.method.as_bytes()).unwrap_or(Method::GET)) {
            return Ok(());
        }

        let fixture_path = self.get_fixture_path(fingerprint);

        // Create directory if it doesn't exist
        if let Some(parent) = fixture_path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| Error::generic(format!("Failed to create directory {}: {}", parent.display(), e)))?;
        }

        // Convert response headers to HashMap
        let mut response_headers_map = HashMap::new();
        for (key, value) in response_headers.iter() {
            let key_str = key.as_str();
            if let Ok(value_str) = value.to_str() {
                response_headers_map.insert(key_str.to_string(), value_str.to_string());
            }
        }

        let recorded_request = RecordedRequest {
            fingerprint: fingerprint.clone(),
            timestamp: chrono::Utc::now(),
            status_code,
            response_headers: response_headers_map,
            response_body: response_body.to_string(),
            metadata: metadata.unwrap_or_default(),
        };

        let content = serde_json::to_string_pretty(&recorded_request)
            .map_err(|e| Error::generic(format!("Failed to serialize recorded request: {}", e)))?;

        fs::write(&fixture_path, content).await
            .map_err(|e| Error::generic(format!("Failed to write fixture {}: {}", fixture_path.display(), e)))?;

        tracing::info!("Recorded request to {}", fixture_path.display());
        Ok(())
    }

    /// Get the fixture path for a request fingerprint
    fn get_fixture_path(&self, fingerprint: &RequestFingerprint) -> PathBuf {
        let hash = fingerprint.to_hash();
        let method = fingerprint.method.to_lowercase();
        let path_hash = fingerprint.path.replace('/', "_").replace(':', "_");

        self.fixtures_dir
            .join("http")
            .join(&method)
            .join(&path_hash)
            .join(format!("{}.json", hash))
    }
}

/// Combined record/replay handler
pub struct RecordReplayHandler {
    replay_handler: ReplayHandler,
    record_handler: RecordHandler,
}

impl RecordReplayHandler {
    /// Create a new record/replay handler
    pub fn new(fixtures_dir: PathBuf, replay_enabled: bool, record_enabled: bool, record_get_only: bool) -> Self {
        Self {
            replay_handler: ReplayHandler::new(fixtures_dir.clone(), replay_enabled),
            record_handler: RecordHandler::new(fixtures_dir, record_enabled, record_get_only),
        }
    }

    /// Get the replay handler
    pub fn replay_handler(&self) -> &ReplayHandler {
        &self.replay_handler
    }

    /// Get the record handler
    pub fn record_handler(&self) -> &RecordHandler {
        &self.record_handler
    }
}

/// List all available fixtures
pub async fn list_fixtures(fixtures_dir: &Path) -> Result<Vec<RecordedRequest>> {
    let mut fixtures = Vec::new();

    if !fixtures_dir.exists() {
        return Ok(fixtures);
    }

    let http_dir = fixtures_dir.join("http");
    if !http_dir.exists() {
        return Ok(fixtures);
    }

    // Use globwalk to find all JSON files recursively
    let walker = globwalk::GlobWalkerBuilder::from_patterns(&http_dir, &["**/*.json"])
        .build()
        .map_err(|e| Error::generic(format!("Failed to build glob walker: {}", e)))?;

    for entry in walker {
        let entry = entry.map_err(|e| Error::generic(format!("Failed to read directory entry: {}", e)))?;
        let path = entry.path();
        
        if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
            if let Ok(content) = fs::read_to_string(&path).await {
                if let Ok(recorded_request) = serde_json::from_str::<RecordedRequest>(&content) {
                    fixtures.push(recorded_request);
                }
            }
        }
    }

    // Sort by timestamp (newest first)
    fixtures.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(fixtures)
}

/// Clean old fixtures (older than specified days)
pub async fn clean_old_fixtures(fixtures_dir: &Path, older_than_days: u32) -> Result<usize> {
    let cutoff_date = chrono::Utc::now() - chrono::Duration::days(older_than_days as i64);
    let mut cleaned_count = 0;

    if !fixtures_dir.exists() {
        return Ok(0);
    }

    let http_dir = fixtures_dir.join("http");
    if !http_dir.exists() {
        return Ok(0);
    }

    let mut entries = fs::read_dir(&http_dir).await
        .map_err(|e| Error::generic(format!("Failed to read fixtures directory: {}", e)))?;

    while let Some(entry) = entries.next_entry().await
        .map_err(|e| Error::generic(format!("Failed to read directory entry: {}", e)))? {

        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
            if let Ok(content) = fs::read_to_string(&path).await {
                if let Ok(recorded_request) = serde_json::from_str::<RecordedRequest>(&content) {
                    if recorded_request.timestamp < cutoff_date {
                        if let Err(e) = fs::remove_file(&path).await {
                            tracing::warn!("Failed to remove old fixture {}: {}", path.display(), e);
                        } else {
                            cleaned_count += 1;
                        }
                    }
                }
            }
        }
    }

    Ok(cleaned_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{Uri};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_record_and_replay() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        let handler = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Create a test fingerprint
        let method = Method::GET;
        let uri: Uri = "/api/users?page=1".parse().unwrap();
        let headers = HeaderMap::new();
        let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);

        // Record a request
        let mut response_headers = HeaderMap::new();
        response_headers.insert("content-type", "application/json".parse().unwrap());

        handler.record_handler().record_request(
            &fingerprint,
            200,
            &response_headers,
            r#"{"users": []}"#,
            None,
        ).await.unwrap();

        // Check if fixture exists
        assert!(handler.replay_handler().has_fixture(&fingerprint).await);

        // Load the fixture
        let recorded = handler.replay_handler().load_fixture(&fingerprint).await.unwrap().unwrap();
        assert_eq!(recorded.status_code, 200);
        assert_eq!(recorded.response_body, r#"{"users": []}"#);
    }

    #[tokio::test]
    async fn test_list_fixtures() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures_dir = temp_dir.path().to_path_buf();

        let handler = RecordReplayHandler::new(fixtures_dir.clone(), true, true, false);

        // Record a few requests
        for i in 0..3 {
            let method = Method::GET;
            let uri: Uri = format!("/api/users/{}", i).parse().unwrap();
            let headers = HeaderMap::new();
            let fingerprint = RequestFingerprint::new(method, &uri, &headers, None);

            handler.record_handler().record_request(
                &fingerprint,
                200,
                &HeaderMap::new(),
                &format!(r#"{{"id": {}}}"#, i),
                None,
            ).await.unwrap();
        }

        // List fixtures
        let fixtures = list_fixtures(&fixtures_dir).await.unwrap();
        assert_eq!(fixtures.len(), 3);
    }
}
