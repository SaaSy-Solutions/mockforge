//! TCP spec registry for managing TCP fixtures

use crate::fixtures::TcpFixture;
use mockforge_core::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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
        for fixture in self.fixtures.values() {
            if fixture.matches(data) {
                return Some(fixture);
            }
        }

        None
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
