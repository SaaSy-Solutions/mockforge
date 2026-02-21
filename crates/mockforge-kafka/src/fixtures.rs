use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

/// Kafka fixture for message generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaFixture {
    pub identifier: String,
    pub name: String,
    pub topic: String,
    pub partition: Option<i32>,      // None = all partitions
    pub key_pattern: Option<String>, // Template
    pub value_template: serde_json::Value,
    pub headers: HashMap<String, String>,
    pub auto_produce: Option<AutoProduceConfig>,
}

/// Configuration for auto-producing messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoProduceConfig {
    pub enabled: bool,
    pub rate_per_second: u64,
    pub duration_seconds: Option<u64>,
    pub total_count: Option<usize>,
}

/// Auto-producer for fixtures
pub struct AutoProducer {
    fixtures: Arc<RwLock<HashMap<String, KafkaFixture>>>,
    template_engine: mockforge_core::templating::TemplateEngine,
    broker: Arc<super::broker::KafkaMockBroker>,
}

impl AutoProducer {
    /// Create a new auto-producer
    pub fn new(
        broker: Arc<super::broker::KafkaMockBroker>,
        template_engine: mockforge_core::templating::TemplateEngine,
    ) -> Self {
        Self {
            fixtures: Arc::new(RwLock::new(HashMap::new())),
            template_engine,
            broker,
        }
    }

    /// Add a fixture for auto-production
    pub async fn add_fixture(&self, fixture: KafkaFixture) {
        if fixture.auto_produce.as_ref().is_some_and(|ap| ap.enabled) {
            let fixture_id = fixture.identifier.clone();
            self.fixtures.write().await.insert(fixture_id, fixture);
        }
    }

    /// Start auto-producing messages
    pub async fn start(&self) -> anyhow::Result<()> {
        let fixtures = self.fixtures.clone();
        let _template_engine = self.template_engine.clone();
        let _broker = self.broker.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1));

            loop {
                interval.tick().await;

                let fixtures_read = fixtures.read().await.clone();
                for fixture in fixtures_read.values() {
                    if let Some(auto_produce) = &fixture.auto_produce {
                        if auto_produce.enabled {
                            // Generate and produce messages
                            for _ in 0..auto_produce.rate_per_second {
                                if let Ok(message) = fixture.generate_message(&HashMap::new()) {
                                    // Produce the message to the broker
                                    let mut topics = _broker.topics.write().await;
                                    if let Some(topic) = topics.get_mut(&fixture.topic) {
                                        let partition = fixture.partition.unwrap_or_else(|| {
                                            topic.assign_partition(message.key.as_deref())
                                        });

                                        if let Err(e) = topic.produce(partition, message).await {
                                            tracing::error!(
                                                "Failed to produce message to topic {}: {}",
                                                fixture.topic,
                                                e
                                            );
                                        } else {
                                            tracing::debug!(
                                                "Auto-produced message to topic {} partition {}",
                                                fixture.topic,
                                                partition
                                            );
                                        }
                                    } else {
                                        tracing::warn!(
                                            "Topic {} not found for auto-production",
                                            fixture.topic
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop auto-producing for a specific fixture
    pub async fn stop_fixture(&self, fixture_id: &str) {
        if let Some(fixture) = self.fixtures.write().await.get_mut(fixture_id) {
            if let Some(auto_produce) = &mut fixture.auto_produce {
                auto_produce.enabled = false;
            }
        }
    }
}

impl KafkaFixture {
    /// Load fixtures from a directory
    pub fn load_from_dir(dir: &PathBuf) -> mockforge_core::Result<Vec<Self>> {
        let mut fixtures = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml")
                || path.extension().and_then(|s| s.to_str()) == Some("yml")
            {
                let file = fs::File::open(&path)?;
                let file_fixtures: Vec<Self> = serde_yaml::from_reader(file)?;
                fixtures.extend(file_fixtures);
            }
        }
        Ok(fixtures)
    }

    /// Generate a message using the fixture
    pub fn generate_message(
        &self,
        context: &HashMap<String, String>,
    ) -> mockforge_core::Result<crate::partitions::KafkaMessage> {
        // Render key if pattern provided
        let key = self.key_pattern.as_ref().map(|pattern| self.render_template(pattern, context));

        // Render value template
        let value_str = serde_json::to_string(&self.value_template)?;
        let value_rendered = self.render_template(&value_str, context);
        let value = value_rendered.into_bytes();

        // Render headers
        let headers = self
            .headers
            .iter()
            .map(|(k, v)| (k.clone(), self.render_template(v, context).into_bytes()))
            .collect();

        Ok(crate::partitions::KafkaMessage {
            offset: 0,
            timestamp: Utc::now().timestamp_millis(),
            key: key.map(|k| k.into_bytes()),
            value,
            headers,
        })
    }

    fn render_template(&self, template: &str, context: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in context {
            result = result.replace(&format!("{{{{{}}}}}", key), value);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;

    // ==================== KafkaFixture Tests ====================

    #[test]
    fn test_kafka_fixture_creation() {
        let fixture = KafkaFixture {
            identifier: "test-fixture".to_string(),
            name: "Test Fixture".to_string(),
            topic: "test-topic".to_string(),
            partition: Some(0),
            key_pattern: Some("key-{{id}}".to_string()),
            value_template: serde_json::json!({"message": "test"}),
            headers: HashMap::new(),
            auto_produce: None,
        };

        assert_eq!(fixture.identifier, "test-fixture");
        assert_eq!(fixture.topic, "test-topic");
        assert_eq!(fixture.partition, Some(0));
        assert!(fixture.auto_produce.is_none());
    }

    #[test]
    fn test_kafka_fixture_with_auto_produce() {
        let auto_produce = AutoProduceConfig {
            enabled: true,
            rate_per_second: 10,
            duration_seconds: Some(60),
            total_count: Some(100),
        };

        let fixture = KafkaFixture {
            identifier: "auto-fixture".to_string(),
            name: "Auto Fixture".to_string(),
            topic: "auto-topic".to_string(),
            partition: None,
            key_pattern: None,
            value_template: serde_json::json!({"auto": true}),
            headers: HashMap::new(),
            auto_produce: Some(auto_produce),
        };

        assert!(fixture.auto_produce.is_some());
        let ap = fixture.auto_produce.as_ref().unwrap();
        assert!(ap.enabled);
        assert_eq!(ap.rate_per_second, 10);
        assert_eq!(ap.duration_seconds, Some(60));
        assert_eq!(ap.total_count, Some(100));
    }

    #[test]
    fn test_kafka_fixture_clone() {
        let fixture = KafkaFixture {
            identifier: "clone-test".to_string(),
            name: "Clone Test".to_string(),
            topic: "clone-topic".to_string(),
            partition: Some(1),
            key_pattern: Some("key".to_string()),
            value_template: serde_json::json!({"data": "value"}),
            headers: HashMap::new(),
            auto_produce: None,
        };

        let cloned = fixture.clone();
        assert_eq!(fixture.identifier, cloned.identifier);
        assert_eq!(fixture.topic, cloned.topic);
        assert_eq!(fixture.partition, cloned.partition);
    }

    #[test]
    fn test_kafka_fixture_serialize_deserialize() {
        let fixture = KafkaFixture {
            identifier: "serde-test".to_string(),
            name: "Serde Test".to_string(),
            topic: "serde-topic".to_string(),
            partition: Some(0),
            key_pattern: Some("key-pattern".to_string()),
            value_template: serde_json::json!({"test": "data"}),
            headers: HashMap::new(),
            auto_produce: None,
        };

        let yaml = serde_yaml::to_string(&fixture).unwrap();
        let deserialized: KafkaFixture = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(fixture.identifier, deserialized.identifier);
        assert_eq!(fixture.topic, deserialized.topic);
    }

    // ==================== AutoProduceConfig Tests ====================

    #[test]
    fn test_auto_produce_config_enabled() {
        let config = AutoProduceConfig {
            enabled: true,
            rate_per_second: 5,
            duration_seconds: None,
            total_count: None,
        };

        assert!(config.enabled);
        assert_eq!(config.rate_per_second, 5);
        assert!(config.duration_seconds.is_none());
        assert!(config.total_count.is_none());
    }

    #[test]
    fn test_auto_produce_config_disabled() {
        let config = AutoProduceConfig {
            enabled: false,
            rate_per_second: 0,
            duration_seconds: None,
            total_count: None,
        };

        assert!(!config.enabled);
    }

    #[test]
    fn test_auto_produce_config_with_limits() {
        let config = AutoProduceConfig {
            enabled: true,
            rate_per_second: 100,
            duration_seconds: Some(300),
            total_count: Some(10000),
        };

        assert_eq!(config.rate_per_second, 100);
        assert_eq!(config.duration_seconds, Some(300));
        assert_eq!(config.total_count, Some(10000));
    }

    #[test]
    fn test_auto_produce_config_clone() {
        let config = AutoProduceConfig {
            enabled: true,
            rate_per_second: 10,
            duration_seconds: Some(60),
            total_count: Some(100),
        };

        let cloned = config.clone();
        assert_eq!(config.enabled, cloned.enabled);
        assert_eq!(config.rate_per_second, cloned.rate_per_second);
        assert_eq!(config.duration_seconds, cloned.duration_seconds);
        assert_eq!(config.total_count, cloned.total_count);
    }

    // ==================== KafkaFixture::generate_message Tests ====================

    #[test]
    fn test_generate_message_basic() {
        let fixture = KafkaFixture {
            identifier: "msg-test".to_string(),
            name: "Message Test".to_string(),
            topic: "test-topic".to_string(),
            partition: Some(0),
            key_pattern: None,
            value_template: serde_json::json!({"message": "hello"}),
            headers: HashMap::new(),
            auto_produce: None,
        };

        let context = HashMap::new();
        let message = fixture.generate_message(&context).unwrap();

        assert!(message.key.is_none());
        assert!(!message.value.is_empty());
        assert_eq!(message.offset, 0);
        assert!(message.timestamp > 0);
    }

    #[test]
    fn test_generate_message_with_key() {
        let fixture = KafkaFixture {
            identifier: "key-test".to_string(),
            name: "Key Test".to_string(),
            topic: "test-topic".to_string(),
            partition: Some(0),
            key_pattern: Some("order-12345".to_string()),
            value_template: serde_json::json!({"order": "data"}),
            headers: HashMap::new(),
            auto_produce: None,
        };

        let context = HashMap::new();
        let message = fixture.generate_message(&context).unwrap();

        assert!(message.key.is_some());
        assert_eq!(message.key.unwrap(), b"order-12345".to_vec());
    }

    #[test]
    fn test_generate_message_with_template_substitution() {
        let fixture = KafkaFixture {
            identifier: "template-test".to_string(),
            name: "Template Test".to_string(),
            topic: "test-topic".to_string(),
            partition: Some(0),
            key_pattern: Some("user-{{userId}}".to_string()),
            value_template: serde_json::json!({"userId": "{{userId}}", "action": "login"}),
            headers: HashMap::new(),
            auto_produce: None,
        };

        let mut context = HashMap::new();
        context.insert("userId".to_string(), "123".to_string());

        let message = fixture.generate_message(&context).unwrap();

        assert!(message.key.is_some());
        assert_eq!(message.key.unwrap(), b"user-123".to_vec());

        let value_str = String::from_utf8(message.value).unwrap();
        assert!(value_str.contains("123"));
    }

    #[test]
    fn test_generate_message_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("correlation-id".to_string(), "abc-123".to_string());
        headers.insert("source".to_string(), "test-service".to_string());

        let fixture = KafkaFixture {
            identifier: "header-test".to_string(),
            name: "Header Test".to_string(),
            topic: "test-topic".to_string(),
            partition: Some(0),
            key_pattern: None,
            value_template: serde_json::json!({"data": "test"}),
            headers,
            auto_produce: None,
        };

        let context = HashMap::new();
        let message = fixture.generate_message(&context).unwrap();

        assert_eq!(message.headers.len(), 2);
        assert!(message.headers.iter().any(|(k, _)| k == "correlation-id"));
        assert!(message.headers.iter().any(|(k, _)| k == "source"));
    }

    #[test]
    fn test_generate_message_with_template_headers() {
        let mut headers = HashMap::new();
        headers.insert("trace-id".to_string(), "trace-{{traceId}}".to_string());

        let fixture = KafkaFixture {
            identifier: "header-template-test".to_string(),
            name: "Header Template Test".to_string(),
            topic: "test-topic".to_string(),
            partition: Some(0),
            key_pattern: None,
            value_template: serde_json::json!({"data": "test"}),
            headers,
            auto_produce: None,
        };

        let mut context = HashMap::new();
        context.insert("traceId".to_string(), "xyz789".to_string());

        let message = fixture.generate_message(&context).unwrap();

        let trace_header = message.headers.iter().find(|(k, _)| k == "trace-id");
        assert!(trace_header.is_some());
        assert_eq!(trace_header.unwrap().1, b"trace-xyz789".to_vec());
    }

    #[test]
    fn test_generate_message_empty_context() {
        let fixture = KafkaFixture {
            identifier: "empty-context".to_string(),
            name: "Empty Context".to_string(),
            topic: "test-topic".to_string(),
            partition: Some(0),
            key_pattern: Some("static-key".to_string()),
            value_template: serde_json::json!({"static": "value"}),
            headers: HashMap::new(),
            auto_produce: None,
        };

        let context = HashMap::new();
        let message = fixture.generate_message(&context).unwrap();

        assert!(message.key.is_some());
        assert_eq!(message.key.unwrap(), b"static-key".to_vec());
    }

    // ==================== KafkaFixture::render_template Tests ====================

    #[test]
    fn test_render_template_no_substitution() {
        let fixture = KafkaFixture {
            identifier: "render-test".to_string(),
            name: "Render Test".to_string(),
            topic: "test-topic".to_string(),
            partition: Some(0),
            key_pattern: None,
            value_template: serde_json::json!({}),
            headers: HashMap::new(),
            auto_produce: None,
        };

        let context = HashMap::new();
        let result = fixture.render_template("static text", &context);
        assert_eq!(result, "static text");
    }

    #[test]
    fn test_render_template_single_substitution() {
        let fixture = KafkaFixture {
            identifier: "render-test".to_string(),
            name: "Render Test".to_string(),
            topic: "test-topic".to_string(),
            partition: Some(0),
            key_pattern: None,
            value_template: serde_json::json!({}),
            headers: HashMap::new(),
            auto_produce: None,
        };

        let mut context = HashMap::new();
        context.insert("name".to_string(), "Alice".to_string());

        let result = fixture.render_template("Hello {{name}}", &context);
        assert_eq!(result, "Hello Alice");
    }

    #[test]
    fn test_render_template_multiple_substitutions() {
        let fixture = KafkaFixture {
            identifier: "render-test".to_string(),
            name: "Render Test".to_string(),
            topic: "test-topic".to_string(),
            partition: Some(0),
            key_pattern: None,
            value_template: serde_json::json!({}),
            headers: HashMap::new(),
            auto_produce: None,
        };

        let mut context = HashMap::new();
        context.insert("first".to_string(), "John".to_string());
        context.insert("last".to_string(), "Doe".to_string());

        let result = fixture.render_template("{{first}} {{last}}", &context);
        assert_eq!(result, "John Doe");
    }

    #[test]
    fn test_render_template_missing_variable() {
        let fixture = KafkaFixture {
            identifier: "render-test".to_string(),
            name: "Render Test".to_string(),
            topic: "test-topic".to_string(),
            partition: Some(0),
            key_pattern: None,
            value_template: serde_json::json!({}),
            headers: HashMap::new(),
            auto_produce: None,
        };

        let context = HashMap::new();
        let result = fixture.render_template("Hello {{name}}", &context);
        // Missing variables are left as-is
        assert_eq!(result, "Hello {{name}}");
    }

    // ==================== KafkaFixture::load_from_dir Tests ====================

    #[test]
    fn test_load_from_dir_empty_directory() {
        let temp_dir = TempDir::new().unwrap();
        let result = KafkaFixture::load_from_dir(&temp_dir.path().to_path_buf()).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_load_from_dir_with_yaml_files() {
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = temp_dir.path().join("fixtures.yaml");

        let fixtures = vec![KafkaFixture {
            identifier: "test-fixture".to_string(),
            name: "Test Fixture".to_string(),
            topic: "test-topic".to_string(),
            partition: Some(0),
            key_pattern: None,
            value_template: serde_json::json!({"test": "data"}),
            headers: HashMap::new(),
            auto_produce: None,
        }];

        let yaml_content = serde_yaml::to_string(&fixtures).unwrap();
        fs::write(&fixture_path, yaml_content).unwrap();

        let loaded = KafkaFixture::load_from_dir(&temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].identifier, "test-fixture");
    }

    #[test]
    fn test_load_from_dir_with_yml_extension() {
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = temp_dir.path().join("fixtures.yml");

        let fixtures = vec![KafkaFixture {
            identifier: "yml-test".to_string(),
            name: "YML Test".to_string(),
            topic: "yml-topic".to_string(),
            partition: None,
            key_pattern: None,
            value_template: serde_json::json!({"yml": true}),
            headers: HashMap::new(),
            auto_produce: None,
        }];

        let yaml_content = serde_yaml::to_string(&fixtures).unwrap();
        fs::write(&fixture_path, yaml_content).unwrap();

        let loaded = KafkaFixture::load_from_dir(&temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].identifier, "yml-test");
    }

    #[test]
    fn test_load_from_dir_ignores_non_yaml_files() {
        let temp_dir = TempDir::new().unwrap();
        let txt_path = temp_dir.path().join("readme.txt");
        fs::write(&txt_path, "This is not a YAML file").unwrap();

        let loaded = KafkaFixture::load_from_dir(&temp_dir.path().to_path_buf()).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_load_from_dir_multiple_files() {
        let temp_dir = TempDir::new().unwrap();

        let fixtures1 = vec![KafkaFixture {
            identifier: "fixture-1".to_string(),
            name: "Fixture 1".to_string(),
            topic: "topic-1".to_string(),
            partition: None,
            key_pattern: None,
            value_template: serde_json::json!({"id": 1}),
            headers: HashMap::new(),
            auto_produce: None,
        }];

        let fixtures2 = vec![KafkaFixture {
            identifier: "fixture-2".to_string(),
            name: "Fixture 2".to_string(),
            topic: "topic-2".to_string(),
            partition: None,
            key_pattern: None,
            value_template: serde_json::json!({"id": 2}),
            headers: HashMap::new(),
            auto_produce: None,
        }];

        fs::write(
            temp_dir.path().join("fixtures1.yaml"),
            serde_yaml::to_string(&fixtures1).unwrap(),
        )
        .unwrap();

        fs::write(
            temp_dir.path().join("fixtures2.yaml"),
            serde_yaml::to_string(&fixtures2).unwrap(),
        )
        .unwrap();

        let loaded = KafkaFixture::load_from_dir(&temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(loaded.len(), 2);
    }

    #[test]
    fn test_load_from_dir_nonexistent() {
        let result = KafkaFixture::load_from_dir(&PathBuf::from("/nonexistent/path"));
        assert!(result.is_err());
    }

    // ==================== AutoProducer Tests ====================

    #[tokio::test]
    async fn test_auto_producer_creation() {
        let config = mockforge_core::config::KafkaConfig::default();
        let broker = Arc::new(crate::broker::KafkaMockBroker::new(config).await.unwrap());
        let template_engine = mockforge_core::templating::TemplateEngine::new();

        let producer = AutoProducer::new(broker, template_engine);
        let fixtures = producer.fixtures.read().await;
        assert!(fixtures.is_empty());
    }

    #[tokio::test]
    async fn test_auto_producer_add_fixture_enabled() {
        let config = mockforge_core::config::KafkaConfig::default();
        let broker = Arc::new(crate::broker::KafkaMockBroker::new(config).await.unwrap());
        let template_engine = mockforge_core::templating::TemplateEngine::new();

        let producer = AutoProducer::new(broker, template_engine);

        let fixture = KafkaFixture {
            identifier: "auto-enabled".to_string(),
            name: "Auto Enabled".to_string(),
            topic: "auto-topic".to_string(),
            partition: None,
            key_pattern: None,
            value_template: serde_json::json!({"auto": true}),
            headers: HashMap::new(),
            auto_produce: Some(AutoProduceConfig {
                enabled: true,
                rate_per_second: 1,
                duration_seconds: None,
                total_count: None,
            }),
        };

        producer.add_fixture(fixture).await;

        let fixtures = producer.fixtures.read().await;
        assert_eq!(fixtures.len(), 1);
        assert!(fixtures.contains_key("auto-enabled"));
    }

    #[tokio::test]
    async fn test_auto_producer_add_fixture_disabled() {
        let config = mockforge_core::config::KafkaConfig::default();
        let broker = Arc::new(crate::broker::KafkaMockBroker::new(config).await.unwrap());
        let template_engine = mockforge_core::templating::TemplateEngine::new();

        let producer = AutoProducer::new(broker, template_engine);

        let fixture = KafkaFixture {
            identifier: "auto-disabled".to_string(),
            name: "Auto Disabled".to_string(),
            topic: "disabled-topic".to_string(),
            partition: None,
            key_pattern: None,
            value_template: serde_json::json!({"auto": false}),
            headers: HashMap::new(),
            auto_produce: Some(AutoProduceConfig {
                enabled: false,
                rate_per_second: 1,
                duration_seconds: None,
                total_count: None,
            }),
        };

        producer.add_fixture(fixture).await;

        let fixtures = producer.fixtures.read().await;
        assert!(fixtures.is_empty());
    }

    #[tokio::test]
    async fn test_auto_producer_add_fixture_no_auto_produce() {
        let config = mockforge_core::config::KafkaConfig::default();
        let broker = Arc::new(crate::broker::KafkaMockBroker::new(config).await.unwrap());
        let template_engine = mockforge_core::templating::TemplateEngine::new();

        let producer = AutoProducer::new(broker, template_engine);

        let fixture = KafkaFixture {
            identifier: "no-auto".to_string(),
            name: "No Auto".to_string(),
            topic: "manual-topic".to_string(),
            partition: None,
            key_pattern: None,
            value_template: serde_json::json!({"manual": true}),
            headers: HashMap::new(),
            auto_produce: None,
        };

        producer.add_fixture(fixture).await;

        let fixtures = producer.fixtures.read().await;
        assert!(fixtures.is_empty());
    }

    #[tokio::test]
    async fn test_auto_producer_stop_fixture() {
        let config = mockforge_core::config::KafkaConfig::default();
        let broker = Arc::new(crate::broker::KafkaMockBroker::new(config).await.unwrap());
        let template_engine = mockforge_core::templating::TemplateEngine::new();

        let producer = AutoProducer::new(broker, template_engine);

        let fixture = KafkaFixture {
            identifier: "stop-test".to_string(),
            name: "Stop Test".to_string(),
            topic: "stop-topic".to_string(),
            partition: None,
            key_pattern: None,
            value_template: serde_json::json!({"test": true}),
            headers: HashMap::new(),
            auto_produce: Some(AutoProduceConfig {
                enabled: true,
                rate_per_second: 1,
                duration_seconds: None,
                total_count: None,
            }),
        };

        producer.add_fixture(fixture).await;
        producer.stop_fixture("stop-test").await;

        let fixtures = producer.fixtures.read().await;
        let fixture = fixtures.get("stop-test");
        assert!(fixture.is_some());
        assert_eq!(fixture.unwrap().auto_produce.as_ref().unwrap().enabled, false);
    }

    #[tokio::test]
    async fn test_auto_producer_stop_nonexistent_fixture() {
        let config = mockforge_core::config::KafkaConfig::default();
        let broker = Arc::new(crate::broker::KafkaMockBroker::new(config).await.unwrap());
        let template_engine = mockforge_core::templating::TemplateEngine::new();

        let producer = AutoProducer::new(broker, template_engine);
        producer.stop_fixture("nonexistent").await;

        // Should not panic
        let fixtures = producer.fixtures.read().await;
        assert!(fixtures.is_empty());
    }
}
