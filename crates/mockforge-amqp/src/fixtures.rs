use crate::exchanges::ExchangeType;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

/// Configuration for an exchange in fixtures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub exchange_type: ExchangeType,
    pub durable: bool,
}

/// Configuration for a queue in fixtures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub name: String,
    pub durable: bool,
    pub message_template: Option<serde_json::Value>,
}

/// Configuration for a binding in fixtures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingConfig {
    pub exchange: String,
    pub queue: String,
    pub routing_key: String,
}

/// Configuration for auto-publish
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoPublishConfig {
    pub enabled: bool,
    pub exchange: String,
    pub routing_key: String,
    pub rate_per_second: u64,
    pub message_template: serde_json::Value,
}

/// AMQP fixture configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmqpFixture {
    pub identifier: String,
    pub name: String,
    pub exchanges: Vec<ExchangeConfig>,
    pub queues: Vec<QueueConfig>,
    pub bindings: Vec<BindingConfig>,
    pub auto_publish: Option<AutoPublishConfig>,
}

impl AmqpFixture {
    /// Load fixtures from a directory
    pub fn load_from_dir(dir: &std::path::PathBuf) -> mockforge_core::Result<Vec<Self>> {
        let mut fixtures = Vec::new();

        if !dir.exists() {
            return Ok(fixtures);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("yaml")
                || path.extension().and_then(|s| s.to_str()) == Some("yml")
            {
                match Self::load_from_file(&path) {
                    Ok(fixture) => fixtures.push(fixture),
                    Err(e) => {
                        tracing::warn!("Failed to load fixture from {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(fixtures)
    }

    /// Load a single fixture from a YAML file
    fn load_from_file(path: &std::path::Path) -> mockforge_core::Result<Self> {
        debug!("Loading fixture from: {:?}", path);
        let content = std::fs::read_to_string(path)?;
        debug!("File content length: {}", content.len());
        let fixture: AmqpFixture = serde_yaml::from_str(&content).map_err(|e| {
            warn!("YAML parsing error: {}", e);
            e
        })?;
        debug!("Successfully loaded fixture: {}", fixture.identifier);
        Ok(fixture)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_exchange_config_serialize() {
        let config = ExchangeConfig {
            name: "test-exchange".to_string(),
            exchange_type: ExchangeType::Direct,
            durable: true,
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test-exchange"));
        assert!(json.contains("direct"));
    }

    #[test]
    fn test_exchange_config_deserialize() {
        let json = r#"{
            "name": "test-exchange",
            "type": "fanout",
            "durable": false
        }"#;

        let config: ExchangeConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "test-exchange");
        assert_eq!(config.exchange_type, ExchangeType::Fanout);
        assert!(!config.durable);
    }

    #[test]
    fn test_exchange_config_clone() {
        let config = ExchangeConfig {
            name: "test".to_string(),
            exchange_type: ExchangeType::Topic,
            durable: true,
        };

        let cloned = config.clone();
        assert_eq!(config.name, cloned.name);
        assert_eq!(config.exchange_type, cloned.exchange_type);
        assert_eq!(config.durable, cloned.durable);
    }

    #[test]
    fn test_queue_config_serialize() {
        let config = QueueConfig {
            name: "test-queue".to_string(),
            durable: true,
            message_template: Some(serde_json::json!({"field": "value"})),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test-queue"));
    }

    #[test]
    fn test_queue_config_deserialize() {
        let json = r#"{
            "name": "test-queue",
            "durable": false,
            "message_template": {"key": "value"}
        }"#;

        let config: QueueConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "test-queue");
        assert!(!config.durable);
        assert!(config.message_template.is_some());
    }

    #[test]
    fn test_queue_config_without_template() {
        let json = r#"{
            "name": "test-queue",
            "durable": true
        }"#;

        let config: QueueConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "test-queue");
        assert!(config.durable);
        assert!(config.message_template.is_none());
    }

    #[test]
    fn test_queue_config_clone() {
        let config = QueueConfig {
            name: "test".to_string(),
            durable: false,
            message_template: Some(serde_json::json!({"test": "data"})),
        };

        let cloned = config.clone();
        assert_eq!(config.name, cloned.name);
        assert_eq!(config.durable, cloned.durable);
    }

    #[test]
    fn test_binding_config_serialize() {
        let config = BindingConfig {
            exchange: "exchange1".to_string(),
            queue: "queue1".to_string(),
            routing_key: "routing.key".to_string(),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("exchange1"));
        assert!(json.contains("queue1"));
        assert!(json.contains("routing.key"));
    }

    #[test]
    fn test_binding_config_deserialize() {
        let json = r#"{
            "exchange": "exchange1",
            "queue": "queue1",
            "routing_key": "user.created"
        }"#;

        let config: BindingConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.exchange, "exchange1");
        assert_eq!(config.queue, "queue1");
        assert_eq!(config.routing_key, "user.created");
    }

    #[test]
    fn test_binding_config_clone() {
        let config = BindingConfig {
            exchange: "ex".to_string(),
            queue: "q".to_string(),
            routing_key: "key".to_string(),
        };

        let cloned = config.clone();
        assert_eq!(config.exchange, cloned.exchange);
        assert_eq!(config.queue, cloned.queue);
        assert_eq!(config.routing_key, cloned.routing_key);
    }

    #[test]
    fn test_auto_publish_config_serialize() {
        let config = AutoPublishConfig {
            enabled: true,
            exchange: "exchange1".to_string(),
            routing_key: "test.key".to_string(),
            rate_per_second: 10,
            message_template: serde_json::json!({"message": "test"}),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("exchange1"));
        assert!(json.contains("test.key"));
    }

    #[test]
    fn test_auto_publish_config_deserialize() {
        let json = r#"{
            "enabled": false,
            "exchange": "test-exchange",
            "routing_key": "key",
            "rate_per_second": 5,
            "message_template": {"data": "value"}
        }"#;

        let config: AutoPublishConfig = serde_json::from_str(json).unwrap();
        assert!(!config.enabled);
        assert_eq!(config.exchange, "test-exchange");
        assert_eq!(config.routing_key, "key");
        assert_eq!(config.rate_per_second, 5);
    }

    #[test]
    fn test_auto_publish_config_clone() {
        let config = AutoPublishConfig {
            enabled: true,
            exchange: "ex".to_string(),
            routing_key: "key".to_string(),
            rate_per_second: 100,
            message_template: serde_json::json!({}),
        };

        let cloned = config.clone();
        assert_eq!(config.enabled, cloned.enabled);
        assert_eq!(config.rate_per_second, cloned.rate_per_second);
    }

    #[test]
    fn test_amqp_fixture_serialize() {
        let fixture = AmqpFixture {
            identifier: "test-fixture".to_string(),
            name: "Test Fixture".to_string(),
            exchanges: vec![ExchangeConfig {
                name: "exchange1".to_string(),
                exchange_type: ExchangeType::Direct,
                durable: true,
            }],
            queues: vec![QueueConfig {
                name: "queue1".to_string(),
                durable: true,
                message_template: None,
            }],
            bindings: vec![BindingConfig {
                exchange: "exchange1".to_string(),
                queue: "queue1".to_string(),
                routing_key: "key".to_string(),
            }],
            auto_publish: None,
        };

        let json = serde_json::to_string(&fixture).unwrap();
        assert!(json.contains("test-fixture"));
        assert!(json.contains("Test Fixture"));
    }

    #[test]
    fn test_amqp_fixture_deserialize() {
        let json = r#"{
            "identifier": "test-fixture",
            "name": "Test Fixture",
            "exchanges": [
                {
                    "name": "exchange1",
                    "type": "direct",
                    "durable": true
                }
            ],
            "queues": [
                {
                    "name": "queue1",
                    "durable": false
                }
            ],
            "bindings": [
                {
                    "exchange": "exchange1",
                    "queue": "queue1",
                    "routing_key": "test.key"
                }
            ]
        }"#;

        let fixture: AmqpFixture = serde_json::from_str(json).unwrap();
        assert_eq!(fixture.identifier, "test-fixture");
        assert_eq!(fixture.name, "Test Fixture");
        assert_eq!(fixture.exchanges.len(), 1);
        assert_eq!(fixture.queues.len(), 1);
        assert_eq!(fixture.bindings.len(), 1);
        assert!(fixture.auto_publish.is_none());
    }

    #[test]
    fn test_amqp_fixture_with_auto_publish() {
        let json = r#"{
            "identifier": "test-fixture",
            "name": "Test Fixture",
            "exchanges": [],
            "queues": [],
            "bindings": [],
            "auto_publish": {
                "enabled": true,
                "exchange": "exchange1",
                "routing_key": "key",
                "rate_per_second": 10,
                "message_template": {"field": "value"}
            }
        }"#;

        let fixture: AmqpFixture = serde_json::from_str(json).unwrap();
        assert!(fixture.auto_publish.is_some());
        let auto_publish = fixture.auto_publish.unwrap();
        assert!(auto_publish.enabled);
        assert_eq!(auto_publish.rate_per_second, 10);
    }

    #[test]
    fn test_amqp_fixture_clone() {
        let fixture = AmqpFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            exchanges: vec![],
            queues: vec![],
            bindings: vec![],
            auto_publish: None,
        };

        let cloned = fixture.clone();
        assert_eq!(fixture.identifier, cloned.identifier);
        assert_eq!(fixture.name, cloned.name);
    }

    #[test]
    fn test_load_from_dir_empty() {
        let temp_dir = TempDir::new().unwrap();
        let fixtures = AmqpFixture::load_from_dir(&temp_dir.path().to_path_buf()).unwrap();
        assert!(fixtures.is_empty());
    }

    #[test]
    fn test_load_from_dir_nonexistent() {
        let path = std::path::PathBuf::from("/nonexistent/path");
        let fixtures = AmqpFixture::load_from_dir(&path).unwrap();
        assert!(fixtures.is_empty());
    }

    #[test]
    fn test_load_from_file_valid_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("fixture.yaml");

        let yaml_content = r#"
identifier: test-fixture
name: Test Fixture
exchanges:
  - name: exchange1
    type: direct
    durable: true
queues:
  - name: queue1
    durable: true
bindings:
  - exchange: exchange1
    queue: queue1
    routing_key: test.key
"#;

        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(yaml_content.as_bytes()).unwrap();

        let fixture = AmqpFixture::load_from_file(&file_path).unwrap();
        assert_eq!(fixture.identifier, "test-fixture");
        assert_eq!(fixture.name, "Test Fixture");
        assert_eq!(fixture.exchanges.len(), 1);
        assert_eq!(fixture.queues.len(), 1);
        assert_eq!(fixture.bindings.len(), 1);
    }

    #[test]
    fn test_load_from_dir_with_yaml_files() {
        let temp_dir = TempDir::new().unwrap();

        // Create first fixture
        let file1_path = temp_dir.path().join("fixture1.yaml");
        let yaml1 = r#"
identifier: fixture1
name: Fixture 1
exchanges: []
queues: []
bindings: []
"#;
        let mut file1 = std::fs::File::create(&file1_path).unwrap();
        file1.write_all(yaml1.as_bytes()).unwrap();

        // Create second fixture
        let file2_path = temp_dir.path().join("fixture2.yml");
        let yaml2 = r#"
identifier: fixture2
name: Fixture 2
exchanges: []
queues: []
bindings: []
"#;
        let mut file2 = std::fs::File::create(&file2_path).unwrap();
        file2.write_all(yaml2.as_bytes()).unwrap();

        let fixtures = AmqpFixture::load_from_dir(&temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(fixtures.len(), 2);
    }

    #[test]
    fn test_load_from_dir_ignores_non_yaml() {
        let temp_dir = TempDir::new().unwrap();

        // Create a YAML file
        let yaml_path = temp_dir.path().join("fixture.yaml");
        let yaml = r#"
identifier: fixture1
name: Fixture 1
exchanges: []
queues: []
bindings: []
"#;
        let mut yaml_file = std::fs::File::create(&yaml_path).unwrap();
        yaml_file.write_all(yaml.as_bytes()).unwrap();

        // Create a non-YAML file
        let txt_path = temp_dir.path().join("readme.txt");
        let mut txt_file = std::fs::File::create(&txt_path).unwrap();
        txt_file.write_all(b"This is not a YAML file").unwrap();

        let fixtures = AmqpFixture::load_from_dir(&temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(fixtures.len(), 1);
    }

    #[test]
    fn test_load_from_file_invalid_yaml() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("invalid.yaml");

        let invalid_yaml = "this is not valid yaml: [unclosed bracket";
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(invalid_yaml.as_bytes()).unwrap();

        let result = AmqpFixture::load_from_file(&file_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_fixture_debug() {
        let fixture = AmqpFixture {
            identifier: "test".to_string(),
            name: "Test".to_string(),
            exchanges: vec![],
            queues: vec![],
            bindings: vec![],
            auto_publish: None,
        };

        let debug = format!("{:?}", fixture);
        assert!(debug.contains("AmqpFixture"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn test_exchange_config_debug() {
        let config = ExchangeConfig {
            name: "test".to_string(),
            exchange_type: ExchangeType::Direct,
            durable: true,
        };

        let debug = format!("{:?}", config);
        assert!(debug.contains("ExchangeConfig"));
    }

    #[test]
    fn test_queue_config_debug() {
        let config = QueueConfig {
            name: "test".to_string(),
            durable: true,
            message_template: None,
        };

        let debug = format!("{:?}", config);
        assert!(debug.contains("QueueConfig"));
    }

    #[test]
    fn test_binding_config_debug() {
        let config = BindingConfig {
            exchange: "ex".to_string(),
            queue: "q".to_string(),
            routing_key: "key".to_string(),
        };

        let debug = format!("{:?}", config);
        assert!(debug.contains("BindingConfig"));
    }

    #[test]
    fn test_auto_publish_config_debug() {
        let config = AutoPublishConfig {
            enabled: true,
            exchange: "ex".to_string(),
            routing_key: "key".to_string(),
            rate_per_second: 10,
            message_template: serde_json::json!({}),
        };

        let debug = format!("{:?}", config);
        assert!(debug.contains("AutoPublishConfig"));
    }
}
