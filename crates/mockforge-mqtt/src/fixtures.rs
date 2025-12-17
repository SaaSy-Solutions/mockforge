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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_fixture() -> MqttFixture {
        MqttFixture {
            identifier: "test-fixture".to_string(),
            name: "Test Fixture".to_string(),
            topic_pattern: "test/topic".to_string(),
            qos: 1,
            retained: false,
            response: MqttResponse {
                payload: serde_json::json!({"message": "test"}),
            },
            auto_publish: None,
        }
    }

    #[test]
    fn test_mqtt_fixture_clone() {
        let fixture = create_test_fixture();
        let cloned = fixture.clone();
        assert_eq!(fixture.identifier, cloned.identifier);
        assert_eq!(fixture.name, cloned.name);
        assert_eq!(fixture.topic_pattern, cloned.topic_pattern);
    }

    #[test]
    fn test_mqtt_fixture_debug() {
        let fixture = create_test_fixture();
        let debug = format!("{:?}", fixture);
        assert!(debug.contains("MqttFixture"));
        assert!(debug.contains("test-fixture"));
    }

    #[test]
    fn test_mqtt_fixture_serialize() {
        let fixture = create_test_fixture();
        let json = serde_json::to_string(&fixture).unwrap();
        assert!(json.contains("test-fixture"));
        assert!(json.contains("Test Fixture"));
    }

    #[test]
    fn test_mqtt_fixture_deserialize() {
        let json = r#"{
            "identifier": "test",
            "name": "Test",
            "topic_pattern": "topic",
            "qos": 0,
            "retained": false,
            "response": {
                "payload": {}
            }
        }"#;
        let fixture: MqttFixture = serde_json::from_str(json).unwrap();
        assert_eq!(fixture.identifier, "test");
    }

    #[test]
    fn test_mqtt_response_clone() {
        let response = MqttResponse {
            payload: serde_json::json!({"data": "test"}),
        };
        let cloned = response.clone();
        assert_eq!(response.payload, cloned.payload);
    }

    #[test]
    fn test_mqtt_response_debug() {
        let response = MqttResponse {
            payload: serde_json::json!({"test": "value"}),
        };
        let debug = format!("{:?}", response);
        assert!(debug.contains("MqttResponse"));
    }

    #[test]
    fn test_auto_publish_config_clone() {
        let config = AutoPublishConfig {
            enabled: true,
            interval_ms: 1000,
            count: Some(10),
        };
        let cloned = config.clone();
        assert_eq!(config.enabled, cloned.enabled);
        assert_eq!(config.interval_ms, cloned.interval_ms);
        assert_eq!(config.count, cloned.count);
    }

    #[test]
    fn test_auto_publish_config_debug() {
        let config = AutoPublishConfig {
            enabled: true,
            interval_ms: 500,
            count: None,
        };
        let debug = format!("{:?}", config);
        assert!(debug.contains("AutoPublishConfig"));
    }

    #[test]
    fn test_mqtt_fixture_registry_new() {
        let registry = MqttFixtureRegistry::new();
        assert_eq!(registry.fixtures().count(), 0);
    }

    #[test]
    fn test_mqtt_fixture_registry_default() {
        let registry = MqttFixtureRegistry::default();
        assert_eq!(registry.fixtures().count(), 0);
    }

    #[test]
    fn test_add_fixture() {
        let mut registry = MqttFixtureRegistry::new();
        let fixture = create_test_fixture();
        registry.add_fixture(fixture);

        assert_eq!(registry.fixtures().count(), 1);
    }

    #[test]
    fn test_get_fixture() {
        let mut registry = MqttFixtureRegistry::new();
        let fixture = create_test_fixture();
        registry.add_fixture(fixture);

        let retrieved = registry.get_fixture("test-fixture");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().identifier, "test-fixture");
    }

    #[test]
    fn test_get_fixture_not_found() {
        let registry = MqttFixtureRegistry::new();
        let retrieved = registry.get_fixture("nonexistent");
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_find_by_topic_exact() {
        let mut registry = MqttFixtureRegistry::new();
        let fixture = create_test_fixture();
        registry.add_fixture(fixture);

        let found = registry.find_by_topic("test/topic");
        assert!(found.is_some());
        assert_eq!(found.unwrap().identifier, "test-fixture");
    }

    #[test]
    fn test_find_by_topic_regex() {
        let mut registry = MqttFixtureRegistry::new();
        let fixture = MqttFixture {
            identifier: "regex-fixture".to_string(),
            name: "Regex Fixture".to_string(),
            topic_pattern: r"sensor/\w+/temp".to_string(),
            qos: 1,
            retained: false,
            response: MqttResponse {
                payload: serde_json::json!({}),
            },
            auto_publish: None,
        };
        registry.add_fixture(fixture);

        let found = registry.find_by_topic("sensor/room1/temp");
        assert!(found.is_some());
        assert_eq!(found.unwrap().identifier, "regex-fixture");
    }

    #[test]
    fn test_find_by_topic_not_found() {
        let registry = MqttFixtureRegistry::new();
        let found = registry.find_by_topic("nonexistent");
        assert!(found.is_none());
    }

    #[test]
    fn test_find_by_topic_invalid_regex() {
        let mut registry = MqttFixtureRegistry::new();
        let fixture = MqttFixture {
            identifier: "invalid-regex".to_string(),
            name: "Invalid Regex".to_string(),
            topic_pattern: "[invalid(regex".to_string(), // Invalid regex
            qos: 0,
            retained: false,
            response: MqttResponse {
                payload: serde_json::json!({}),
            },
            auto_publish: None,
        };
        registry.add_fixture(fixture);

        let found = registry.find_by_topic("anything");
        assert!(found.is_none());
    }

    #[test]
    fn test_fixtures_iterator() {
        let mut registry = MqttFixtureRegistry::new();
        registry.add_fixture(create_test_fixture());

        let mut count = 0;
        for fixture in registry.fixtures() {
            assert_eq!(fixture.identifier, "test-fixture");
            count += 1;
        }
        assert_eq!(count, 1);
    }

    #[test]
    fn test_load_from_directory_nonexistent() {
        let mut registry = MqttFixtureRegistry::new();
        let result = registry.load_from_directory(std::path::Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_from_directory_not_a_directory() {
        let mut registry = MqttFixtureRegistry::new();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file.txt");
        std::fs::write(&file_path, "test").unwrap();

        let result = registry.load_from_directory(&file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_from_directory_empty() {
        let mut registry = MqttFixtureRegistry::new();
        let temp_dir = TempDir::new().unwrap();

        let result = registry.load_from_directory(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(registry.fixtures().count(), 0);
    }

    #[test]
    fn test_load_from_directory_json() {
        let mut registry = MqttFixtureRegistry::new();
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = temp_dir.path().join("fixture.json");

        let json_content = r#"{
            "identifier": "json-fixture",
            "name": "JSON Fixture",
            "topic_pattern": "test/json",
            "qos": 1,
            "retained": false,
            "response": {
                "payload": {"test": "data"}
            }
        }"#;
        std::fs::write(&fixture_path, json_content).unwrap();

        let result = registry.load_from_directory(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(registry.fixtures().count(), 1);

        let fixture = registry.get_fixture("json-fixture");
        assert!(fixture.is_some());
    }

    #[test]
    fn test_load_from_directory_yaml() {
        let mut registry = MqttFixtureRegistry::new();
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = temp_dir.path().join("fixture.yaml");

        let yaml_content = r#"
identifier: yaml-fixture
name: YAML Fixture
topic_pattern: test/yaml
qos: 2
retained: true
response:
  payload:
    key: value
"#;
        std::fs::write(&fixture_path, yaml_content).unwrap();

        let result = registry.load_from_directory(temp_dir.path());
        assert!(result.is_ok());

        let fixture = registry.get_fixture("yaml-fixture");
        assert!(fixture.is_some());
        assert_eq!(fixture.unwrap().qos, 2);
        assert!(fixture.unwrap().retained);
    }

    #[test]
    fn test_load_from_directory_yml_extension() {
        let mut registry = MqttFixtureRegistry::new();
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = temp_dir.path().join("fixture.yml");

        let yaml_content = r#"
identifier: yml-fixture
name: YML Fixture
topic_pattern: test/yml
qos: 0
retained: false
response:
  payload: {}
"#;
        std::fs::write(&fixture_path, yaml_content).unwrap();

        let result = registry.load_from_directory(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(registry.fixtures().count(), 1);
    }

    #[test]
    fn test_load_from_directory_multiple_files() {
        let mut registry = MqttFixtureRegistry::new();
        let temp_dir = TempDir::new().unwrap();

        // Create JSON fixture
        let json_path = temp_dir.path().join("fixture1.json");
        std::fs::write(
            &json_path,
            r#"{
            "identifier": "fixture1",
            "name": "Fixture 1",
            "topic_pattern": "topic1",
            "qos": 0,
            "retained": false,
            "response": {"payload": {}}
        }"#,
        )
        .unwrap();

        // Create YAML fixture
        let yaml_path = temp_dir.path().join("fixture2.yaml");
        std::fs::write(
            &yaml_path,
            r#"
identifier: fixture2
name: Fixture 2
topic_pattern: topic2
qos: 1
retained: false
response:
  payload: {}
"#,
        )
        .unwrap();

        let result = registry.load_from_directory(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(registry.fixtures().count(), 2);
    }

    #[test]
    fn test_load_from_directory_skips_other_files() {
        let mut registry = MqttFixtureRegistry::new();
        let temp_dir = TempDir::new().unwrap();

        // Create non-fixture files
        std::fs::write(temp_dir.path().join("readme.txt"), "readme").unwrap();
        std::fs::write(temp_dir.path().join("config.toml"), "config").unwrap();

        let result = registry.load_from_directory(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(registry.fixtures().count(), 0);
    }

    #[test]
    fn test_load_from_directory_invalid_json() {
        let mut registry = MqttFixtureRegistry::new();
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = temp_dir.path().join("invalid.json");

        std::fs::write(&fixture_path, "{invalid json}").unwrap();

        // Should continue loading other files
        let result = registry.load_from_directory(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(registry.fixtures().count(), 0);
    }

    #[test]
    fn test_load_from_directory_invalid_yaml() {
        let mut registry = MqttFixtureRegistry::new();
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = temp_dir.path().join("invalid.yaml");

        std::fs::write(&fixture_path, "invalid: yaml: content:").unwrap();

        // Should continue loading other files
        let result = registry.load_from_directory(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(registry.fixtures().count(), 0);
    }

    #[test]
    fn test_fixture_with_auto_publish() {
        let fixture = MqttFixture {
            identifier: "auto-pub".to_string(),
            name: "Auto Publish".to_string(),
            topic_pattern: "auto/topic".to_string(),
            qos: 1,
            retained: false,
            response: MqttResponse {
                payload: serde_json::json!({}),
            },
            auto_publish: Some(AutoPublishConfig {
                enabled: true,
                interval_ms: 1000,
                count: Some(5),
            }),
        };

        assert!(fixture.auto_publish.is_some());
        assert!(fixture.auto_publish.unwrap().enabled);
    }

    #[test]
    fn test_fixture_replace_existing() {
        let mut registry = MqttFixtureRegistry::new();

        let fixture1 = MqttFixture {
            identifier: "same-id".to_string(),
            name: "First".to_string(),
            topic_pattern: "topic1".to_string(),
            qos: 0,
            retained: false,
            response: MqttResponse {
                payload: serde_json::json!({"version": 1}),
            },
            auto_publish: None,
        };

        let fixture2 = MqttFixture {
            identifier: "same-id".to_string(),
            name: "Second".to_string(),
            topic_pattern: "topic2".to_string(),
            qos: 1,
            retained: true,
            response: MqttResponse {
                payload: serde_json::json!({"version": 2}),
            },
            auto_publish: None,
        };

        registry.add_fixture(fixture1);
        registry.add_fixture(fixture2);

        // Should have replaced the first fixture
        let fixture = registry.get_fixture("same-id").unwrap();
        assert_eq!(fixture.name, "Second");
        assert_eq!(fixture.qos, 1);
    }

    #[test]
    fn test_complex_regex_pattern() {
        let mut registry = MqttFixtureRegistry::new();
        let fixture = MqttFixture {
            identifier: "complex".to_string(),
            name: "Complex Pattern".to_string(),
            topic_pattern: r"^sensor/(temp|humidity)/\d+$".to_string(),
            qos: 1,
            retained: false,
            response: MqttResponse {
                payload: serde_json::json!({}),
            },
            auto_publish: None,
        };
        registry.add_fixture(fixture);

        assert!(registry.find_by_topic("sensor/temp/123").is_some());
        assert!(registry.find_by_topic("sensor/humidity/456").is_some());
        assert!(registry.find_by_topic("sensor/pressure/789").is_none());
    }

    #[test]
    fn test_qos_levels() {
        let qos0 = MqttFixture {
            identifier: "qos0".to_string(),
            name: "QoS 0".to_string(),
            topic_pattern: "qos0".to_string(),
            qos: 0,
            retained: false,
            response: MqttResponse {
                payload: serde_json::json!({}),
            },
            auto_publish: None,
        };

        let qos1 = MqttFixture {
            identifier: "qos1".to_string(),
            name: "QoS 1".to_string(),
            topic_pattern: "qos1".to_string(),
            qos: 1,
            retained: false,
            response: MqttResponse {
                payload: serde_json::json!({}),
            },
            auto_publish: None,
        };

        let qos2 = MqttFixture {
            identifier: "qos2".to_string(),
            name: "QoS 2".to_string(),
            topic_pattern: "qos2".to_string(),
            qos: 2,
            retained: false,
            response: MqttResponse {
                payload: serde_json::json!({}),
            },
            auto_publish: None,
        };

        assert_eq!(qos0.qos, 0);
        assert_eq!(qos1.qos, 1);
        assert_eq!(qos2.qos, 2);
    }
}
