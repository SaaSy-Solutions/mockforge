//! TCP spec registry for managing TCP fixtures

use crate::fixtures::TcpFixture;
use mockforge_core::Result;
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, info, warn};

/// Registry for TCP fixtures
#[derive(Debug, Clone)]
pub struct TcpSpecRegistry {
    fixtures: HashMap<String, TcpFixture>,
}

impl TcpSpecRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            fixtures: HashMap::new(),
        }
    }

    /// Load fixtures from a directory
    pub fn load_fixtures<P: AsRef<Path>>(&mut self, fixtures_dir: P) -> Result<()> {
        let fixtures_dir = fixtures_dir.as_ref();
        if !fixtures_dir.exists() {
            debug!("TCP fixtures directory does not exist: {:?}", fixtures_dir);
            return Ok(());
        }

        info!("Loading TCP fixtures from {:?}", fixtures_dir);

        let entries = std::fs::read_dir(fixtures_dir).map_err(|e| {
            mockforge_core::Error::generic(format!("Failed to read fixtures directory: {}", e))
        })?;

        let mut loaded_count = 0;

        for entry in entries {
            let entry = entry.map_err(|e| {
                mockforge_core::Error::generic(format!("Failed to read directory entry: {}", e))
            })?;
            let path = entry.path();

            if path.is_file() {
                match path.extension().and_then(|s| s.to_str()) {
                    Some("yaml") | Some("yml") | Some("json") => {
                        if let Err(e) = self.load_fixture_file(&path) {
                            warn!("Failed to load fixture from {:?}: {}", path, e);
                        } else {
                            loaded_count += 1;
                        }
                    }
                    _ => {
                        debug!("Skipping non-fixture file: {:?}", path);
                    }
                }
            }
        }

        info!("Loaded {} TCP fixture(s)", loaded_count);
        Ok(())
    }

    /// Load a single fixture file
    fn load_fixture_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path).map_err(|e| {
            mockforge_core::Error::generic(format!("Failed to read fixture file: {}", e))
        })?;

        let fixtures: Vec<TcpFixture> = if path.extension().and_then(|s| s.to_str()) == Some("json")
        {
            serde_json::from_str(&content).map_err(|e| {
                mockforge_core::Error::generic(format!("Failed to parse JSON fixture: {}", e))
            })?
        } else {
            serde_yaml::from_str(&content).map_err(|e| {
                mockforge_core::Error::generic(format!("Failed to parse YAML fixture: {}", e))
            })?
        };

        for fixture in fixtures {
            let identifier = fixture.identifier.clone();
            self.fixtures.insert(identifier, fixture);
        }

        Ok(())
    }

    /// Add a fixture to the registry
    pub fn add_fixture(&mut self, fixture: TcpFixture) {
        let identifier = fixture.identifier.clone();
        self.fixtures.insert(identifier, fixture);
    }

    /// Get a fixture by identifier
    pub fn get_fixture(&self, identifier: &str) -> Option<&TcpFixture> {
        self.fixtures.get(identifier)
    }

    /// Find a fixture matching the given data
    pub fn find_matching_fixture(&self, data: &[u8]) -> Option<&TcpFixture> {
        // Try to match against all fixtures
        self.fixtures.values().find(|&fixture| fixture.matches(data)).map(|v| v as _)
    }

    /// Get all fixtures
    pub fn get_all_fixtures(&self) -> Vec<&TcpFixture> {
        self.fixtures.values().collect()
    }

    /// Remove a fixture
    pub fn remove_fixture(&mut self, identifier: &str) -> Option<TcpFixture> {
        self.fixtures.remove(identifier)
    }

    /// Clear all fixtures
    pub fn clear(&mut self) {
        self.fixtures.clear();
    }
}

impl Default for TcpSpecRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl TcpFixture {
    /// Check if this fixture matches the given data
    pub fn matches(&self, data: &[u8]) -> bool {
        let criteria = &self.match_criteria;

        // Check length constraints
        if let Some(min_len) = criteria.min_length {
            if data.len() < min_len {
                return false;
            }
        }

        if let Some(max_len) = criteria.max_length {
            if data.len() > max_len {
                return false;
            }
        }

        // Match all - always matches
        if criteria.match_all {
            return true;
        }

        // Match by exact bytes
        if let Some(ref exact_bytes_b64) = criteria.exact_bytes {
            if let Ok(expected) = base64::decode(exact_bytes_b64) {
                return data == expected.as_slice();
            }
        }

        // Match by hex pattern
        if let Some(ref hex_pattern) = criteria.data_pattern {
            if let Ok(expected) = hex::decode(hex_pattern) {
                return data == expected.as_slice();
            }
        }

        // Match by text pattern (regex)
        if let Some(ref text_pattern) = criteria.text_pattern {
            if let Ok(re) = regex::Regex::new(text_pattern) {
                if let Ok(text) = String::from_utf8(data.to_vec()) {
                    return re.is_match(&text);
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::{BehaviorConfig, MatchCriteria, TcpResponse};

    fn create_test_fixture(id: &str, match_all: bool) -> TcpFixture {
        TcpFixture {
            identifier: id.to_string(),
            name: format!("Fixture {}", id),
            description: "Test fixture".to_string(),
            match_criteria: MatchCriteria {
                match_all,
                ..Default::default()
            },
            response: TcpResponse {
                data: "response".to_string(),
                encoding: "text".to_string(),
                file_path: None,
                delay_ms: 0,
                close_after_response: false,
                keep_alive: true,
            },
            behavior: BehaviorConfig::default(),
        }
    }

    #[test]
    fn test_registry_new() {
        let registry = TcpSpecRegistry::new();
        assert!(registry.get_all_fixtures().is_empty());
    }

    #[test]
    fn test_registry_default() {
        let registry = TcpSpecRegistry::default();
        assert!(registry.get_all_fixtures().is_empty());
    }

    #[test]
    fn test_registry_add_fixture() {
        let mut registry = TcpSpecRegistry::new();
        let fixture = create_test_fixture("test-1", true);

        registry.add_fixture(fixture);

        assert_eq!(registry.get_all_fixtures().len(), 1);
    }

    #[test]
    fn test_registry_get_fixture() {
        let mut registry = TcpSpecRegistry::new();
        let fixture = create_test_fixture("test-1", true);

        registry.add_fixture(fixture);

        let retrieved = registry.get_fixture("test-1");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().identifier, "test-1");
    }

    #[test]
    fn test_registry_get_fixture_not_found() {
        let registry = TcpSpecRegistry::new();
        assert!(registry.get_fixture("nonexistent").is_none());
    }

    #[test]
    fn test_registry_remove_fixture() {
        let mut registry = TcpSpecRegistry::new();
        let fixture = create_test_fixture("test-1", true);

        registry.add_fixture(fixture);
        let removed = registry.remove_fixture("test-1");

        assert!(removed.is_some());
        assert!(registry.get_fixture("test-1").is_none());
    }

    #[test]
    fn test_registry_remove_fixture_not_found() {
        let mut registry = TcpSpecRegistry::new();
        let removed = registry.remove_fixture("nonexistent");
        assert!(removed.is_none());
    }

    #[test]
    fn test_registry_clear() {
        let mut registry = TcpSpecRegistry::new();
        registry.add_fixture(create_test_fixture("test-1", true));
        registry.add_fixture(create_test_fixture("test-2", true));

        registry.clear();

        assert!(registry.get_all_fixtures().is_empty());
    }

    #[test]
    fn test_registry_clone() {
        let mut registry = TcpSpecRegistry::new();
        registry.add_fixture(create_test_fixture("test-1", true));

        let cloned = registry.clone();
        assert_eq!(cloned.get_all_fixtures().len(), 1);
    }

    #[test]
    fn test_registry_debug() {
        let registry = TcpSpecRegistry::new();
        let debug = format!("{:?}", registry);
        assert!(debug.contains("TcpSpecRegistry"));
    }

    #[test]
    fn test_fixture_matches_match_all() {
        let fixture = create_test_fixture("test", true);
        assert!(fixture.matches(b"any data"));
        assert!(fixture.matches(b""));
        assert!(fixture.matches(b"Hello World"));
    }

    #[test]
    fn test_fixture_matches_min_length() {
        let fixture = TcpFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            description: String::new(),
            match_criteria: MatchCriteria {
                min_length: Some(5),
                match_all: true,
                ..Default::default()
            },
            response: TcpResponse {
                data: "ok".to_string(),
                encoding: "text".to_string(),
                file_path: None,
                delay_ms: 0,
                close_after_response: false,
                keep_alive: true,
            },
            behavior: BehaviorConfig::default(),
        };

        assert!(!fixture.matches(b"1234"));
        assert!(fixture.matches(b"12345"));
        assert!(fixture.matches(b"123456"));
    }

    #[test]
    fn test_fixture_matches_max_length() {
        let fixture = TcpFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            description: String::new(),
            match_criteria: MatchCriteria {
                max_length: Some(5),
                match_all: true,
                ..Default::default()
            },
            response: TcpResponse {
                data: "ok".to_string(),
                encoding: "text".to_string(),
                file_path: None,
                delay_ms: 0,
                close_after_response: false,
                keep_alive: true,
            },
            behavior: BehaviorConfig::default(),
        };

        assert!(fixture.matches(b"12345"));
        assert!(!fixture.matches(b"123456"));
    }

    #[test]
    fn test_fixture_matches_text_pattern() {
        let fixture = TcpFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            description: String::new(),
            match_criteria: MatchCriteria {
                text_pattern: Some("hello.*world".to_string()),
                ..Default::default()
            },
            response: TcpResponse {
                data: "ok".to_string(),
                encoding: "text".to_string(),
                file_path: None,
                delay_ms: 0,
                close_after_response: false,
                keep_alive: true,
            },
            behavior: BehaviorConfig::default(),
        };

        assert!(fixture.matches(b"hello world"));
        assert!(fixture.matches(b"hello beautiful world"));
        assert!(!fixture.matches(b"goodbye world"));
    }

    #[test]
    fn test_fixture_matches_hex_pattern() {
        let fixture = TcpFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            description: String::new(),
            match_criteria: MatchCriteria {
                data_pattern: Some("48656c6c6f".to_string()), // "Hello" in hex
                ..Default::default()
            },
            response: TcpResponse {
                data: "ok".to_string(),
                encoding: "text".to_string(),
                file_path: None,
                delay_ms: 0,
                close_after_response: false,
                keep_alive: true,
            },
            behavior: BehaviorConfig::default(),
        };

        assert!(fixture.matches(b"Hello"));
        assert!(!fixture.matches(b"hello"));
        assert!(!fixture.matches(b"World"));
    }

    #[test]
    fn test_fixture_matches_exact_bytes() {
        let fixture = TcpFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            description: String::new(),
            match_criteria: MatchCriteria {
                exact_bytes: Some("SGVsbG8=".to_string()), // "Hello" in base64
                ..Default::default()
            },
            response: TcpResponse {
                data: "ok".to_string(),
                encoding: "text".to_string(),
                file_path: None,
                delay_ms: 0,
                close_after_response: false,
                keep_alive: true,
            },
            behavior: BehaviorConfig::default(),
        };

        assert!(fixture.matches(b"Hello"));
        assert!(!fixture.matches(b"hello"));
    }

    #[test]
    fn test_fixture_no_match() {
        let fixture = TcpFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            description: String::new(),
            match_criteria: MatchCriteria::default(), // No criteria, match_all is false
            response: TcpResponse {
                data: "ok".to_string(),
                encoding: "text".to_string(),
                file_path: None,
                delay_ms: 0,
                close_after_response: false,
                keep_alive: true,
            },
            behavior: BehaviorConfig::default(),
        };

        // With no matching criteria and match_all=false, should not match
        assert!(!fixture.matches(b"anything"));
    }

    #[test]
    fn test_find_matching_fixture() {
        let mut registry = TcpSpecRegistry::new();

        // Add a fixture that matches all
        registry.add_fixture(create_test_fixture("catch-all", true));

        let matched = registry.find_matching_fixture(b"test data");
        assert!(matched.is_some());
        assert_eq!(matched.unwrap().identifier, "catch-all");
    }

    #[test]
    fn test_find_matching_fixture_none() {
        let mut registry = TcpSpecRegistry::new();

        // Add a fixture that doesn't match
        registry.add_fixture(create_test_fixture("no-match", false));

        let matched = registry.find_matching_fixture(b"test data");
        assert!(matched.is_none());
    }
}
