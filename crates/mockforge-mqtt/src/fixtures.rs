use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MQTT fixture for topic-based mocking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttFixture {
    pub identifier: String,
    pub name: String,
    pub topic_pattern: String, // Regex pattern for topic matching
    pub qos: u8,
    pub retained: bool,
    pub response: MqttResponse,
    pub auto_publish: Option<AutoPublishConfig>,
}

/// Response configuration for MQTT fixtures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MqttResponse {
    pub payload: serde_json::Value, // Template-enabled JSON payload
}

/// Auto-publish configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoPublishConfig {
    pub enabled: bool,
    pub interval_ms: u64,
    pub count: Option<usize>, // None = infinite
}

/// MQTT fixture registry
pub struct MqttFixtureRegistry {
    fixtures: HashMap<String, MqttFixture>,
}

impl Default for MqttFixtureRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MqttFixtureRegistry {
    pub fn new() -> Self {
        Self {
            fixtures: HashMap::new(),
        }
    }

    pub fn add_fixture(&mut self, fixture: MqttFixture) {
        self.fixtures.insert(fixture.identifier.clone(), fixture);
    }

    pub fn get_fixture(&self, identifier: &str) -> Option<&MqttFixture> {
        self.fixtures.get(identifier)
    }

    pub fn find_by_topic(&self, topic: &str) -> Option<&MqttFixture> {
        for fixture in self.fixtures.values() {
            if regex::Regex::new(&fixture.topic_pattern).ok()?.is_match(topic) {
                return Some(fixture);
            }
        }
        None
    }

    pub fn fixtures(&self) -> impl Iterator<Item = &MqttFixture> {
        self.fixtures.values()
    }

    /// Load fixtures from a directory
    pub fn load_from_directory(
        &mut self,
        path: &std::path::Path,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !path.exists() {
            return Err(format!("Fixtures directory does not exist: {}", path.display()).into());
        }

        if !path.is_dir() {
            return Err(format!("Path is not a directory: {}", path.display()).into());
        }

        let mut loaded_count = 0;

        // Read all .json and .yaml files from the directory
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(extension) = path.extension() {
                    if extension == "json" || extension == "yaml" || extension == "yml" {
                        match self.load_fixture_file(&path) {
                            Ok(fixture) => {
                                self.add_fixture(fixture);
                                loaded_count += 1;
                            }
                            Err(e) => {
                                eprintln!(
                                    "Warning: Failed to load fixture from {}: {}",
                                    path.display(),
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }

        println!("✅ Loaded {} MQTT fixtures from {}", loaded_count, path.display());
        Ok(())
    }

    /// Load a single fixture file
    fn load_fixture_file(
        &self,
        path: &std::path::Path,
    ) -> Result<MqttFixture, Box<dyn std::error::Error + Send + Sync>> {
        let content = std::fs::read_to_string(path)?;
        let fixture: MqttFixture = if path.extension().unwrap_or_default() == "json" {
            serde_json::from_str(&content)?
        } else {
            serde_yaml::from_str(&content)?
        };
        Ok(fixture)
    }
}
