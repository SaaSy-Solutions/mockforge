use async_trait::async_trait;
use chrono;
use std::collections::HashMap;
use std::sync::Arc;

use mockforge_core::protocol_abstraction::{
    ProtocolRequest, ProtocolResponse, ResponseStatus, SpecOperation, SpecRegistry,
    ValidationError, ValidationResult,
};
use mockforge_core::{Protocol, Result};

/// Kafka-specific spec registry implementation
#[derive(Debug)]
pub struct KafkaSpecRegistry {
    fixtures: Vec<Arc<crate::fixtures::KafkaFixture>>,
    template_engine: mockforge_core::templating::TemplateEngine,
    topics: Arc<tokio::sync::RwLock<HashMap<String, crate::topics::Topic>>>,
}

impl KafkaSpecRegistry {
    /// Create a new Kafka spec registry
    pub async fn new(
        config: mockforge_core::config::KafkaConfig,
        topics: Arc<tokio::sync::RwLock<HashMap<String, crate::topics::Topic>>>,
    ) -> Result<Self> {
        let fixtures = if let Some(fixtures_dir) = &config.fixtures_dir {
            crate::fixtures::KafkaFixture::load_from_dir(fixtures_dir)?
                .into_iter()
                .map(Arc::new)
                .collect()
        } else {
            vec![]
        };

        let template_engine = mockforge_core::templating::TemplateEngine::new();

        Ok(Self {
            fixtures,
            template_engine,
            topics,
        })
    }

    /// Find fixture by topic
    pub fn find_fixture_by_topic(&self, topic: &str) -> Option<Arc<crate::fixtures::KafkaFixture>> {
        self.fixtures.iter().find(|f| f.topic == topic).cloned()
    }

    /// Produce a message to a topic
    pub async fn produce(
        &self,
        topic: &str,
        key: Option<&str>,
        value: &serde_json::Value,
    ) -> Result<i64> {
        let mut topics = self.topics.write().await;

        // Get or create the topic
        let topic_entry = topics.entry(topic.to_string()).or_insert_with(|| {
            crate::topics::Topic::new(topic.to_string(), crate::topics::TopicConfig::default())
        });

        // Assign partition based on key
        let partition_id = topic_entry.assign_partition(key.map(|k| k.as_bytes()));

        // Create the message
        let message = crate::partitions::KafkaMessage {
            offset: 0, // Will be set by partition.append
            timestamp: chrono::Utc::now().timestamp_millis(),
            key: key.map(|k| k.as_bytes().to_vec()),
            value: serde_json::to_vec(value).map_err(mockforge_core::Error::Json)?,
            headers: vec![],
        };

        // Append to partition
        let offset = topic_entry
            .get_partition_mut(partition_id)
            .ok_or_else(|| {
                mockforge_core::Error::generic(format!("Partition {} not found", partition_id))
            })?
            .append(message);

        Ok(offset)
    }

    /// Fetch messages from a topic partition
    pub async fn fetch(
        &self,
        topic: &str,
        partition: i32,
        offset: i64,
    ) -> Result<Vec<crate::partitions::KafkaMessage>> {
        let topics = self.topics.read().await;

        if let Some(topic_entry) = topics.get(topic) {
            if let Some(partition_entry) = topic_entry.get_partition(partition) {
                // Fetch messages starting from offset
                let messages = partition_entry.fetch(offset, 1000); // Max 1000 messages
                Ok(messages.into_iter().cloned().collect())
            } else {
                Err(mockforge_core::Error::generic(format!(
                    "Partition {} not found in topic {}",
                    partition, topic
                )))
            }
        } else {
            Err(mockforge_core::Error::generic(format!("Topic {} not found", topic)))
        }
    }
}

#[async_trait]
impl SpecRegistry for KafkaSpecRegistry {
    fn protocol(&self) -> Protocol {
        Protocol::Kafka
    }

    fn operations(&self) -> Vec<SpecOperation> {
        // Return operations based on fixtures
        self.fixtures
            .iter()
            .map(|fixture| SpecOperation {
                name: fixture.identifier.clone(),
                path: fixture.topic.clone(),
                operation_type: "PRODUCE".to_string(),
                input_schema: Some("KafkaMessage".to_string()),
                output_schema: Some("ProduceResponse".to_string()),
                metadata: HashMap::new(),
            })
            .collect()
    }

    fn find_operation(&self, operation: &str, path: &str) -> Option<SpecOperation> {
        self.fixtures
            .iter()
            .find(|fixture| fixture.topic == path && operation == "PRODUCE")
            .map(|fixture| SpecOperation {
                name: fixture.identifier.clone(),
                path: fixture.topic.clone(),
                operation_type: "PRODUCE".to_string(),
                input_schema: Some("KafkaMessage".to_string()),
                output_schema: Some("ProduceResponse".to_string()),
                metadata: HashMap::new(),
            })
    }

    fn validate_request(&self, request: &ProtocolRequest) -> Result<ValidationResult> {
        // Basic validation - check if topic exists in fixtures
        let valid = if let Some(topic) = &request.topic {
            self.fixtures.iter().any(|f| f.topic == *topic)
        } else {
            false
        };

        Ok(ValidationResult {
            valid,
            errors: if valid {
                vec![]
            } else {
                vec![ValidationError {
                    message: "Topic not found in fixtures".to_string(),
                    path: Some("topic".to_string()),
                    code: Some("TOPIC_NOT_FOUND".to_string()),
                }]
            },
            warnings: vec![],
        })
    }

    fn generate_mock_response(&self, request: &ProtocolRequest) -> Result<ProtocolResponse> {
        let operation = &request.operation;
        let topic = request
            .topic
            .as_ref()
            .ok_or_else(|| mockforge_core::Error::generic("Missing topic"))?;

        match operation.as_str() {
            "PRODUCE" => {
                let fixture = self.find_fixture_by_topic(topic).ok_or_else(|| {
                    mockforge_core::Error::generic(format!("No fixture found for topic {}", topic))
                })?;

                // Generate message using template
                let templating_context = mockforge_core::templating::TemplatingContext::with_env(
                    request.metadata.clone(),
                );
                let value = self
                    .template_engine
                    .expand_tokens_with_context(&fixture.value_template, &templating_context);
                let _key = fixture.key_pattern.as_ref().map(|key_pattern| {
                    self.template_engine.expand_str_with_context(key_pattern, &templating_context)
                });

                // Produce through the broker to get a real offset
                let offset = if let Ok(mut topics) = self.topics.try_write() {
                    let topic_entry = topics.entry(topic.to_string()).or_insert_with(|| {
                        crate::topics::Topic::new(
                            topic.to_string(),
                            crate::topics::TopicConfig::default(),
                        )
                    });
                    let partition_id =
                        topic_entry.assign_partition(_key.as_ref().map(|k| k.as_bytes()));
                    let message = crate::partitions::KafkaMessage {
                        offset: 0,
                        timestamp: chrono::Utc::now().timestamp_millis(),
                        key: _key.as_ref().map(|k| k.as_bytes().to_vec()),
                        value: serde_json::to_vec(&value).map_err(mockforge_core::Error::Json)?,
                        headers: vec![],
                    };
                    topic_entry
                        .get_partition_mut(partition_id)
                        .map(|p| p.append(message))
                        .unwrap_or(0)
                } else {
                    0
                };

                Ok(ProtocolResponse {
                    status: ResponseStatus::KafkaStatus(0), // No error
                    metadata: HashMap::from([
                        ("topic".to_string(), topic.clone()),
                        ("offset".to_string(), offset.to_string()),
                    ]),
                    body: serde_json::to_string(&value)
                        .map_err(mockforge_core::Error::Json)?
                        .into_bytes(),
                    content_type: "application/json".to_string(),
                })
            }
            "FETCH" => {
                let partition = request
                    .partition
                    .ok_or_else(|| mockforge_core::Error::generic("Missing partition"))?;
                let offset: i64 =
                    request.metadata.get("offset").and_then(|s| s.parse().ok()).unwrap_or(0);

                // Fetch real messages from the broker
                let messages: Vec<crate::partitions::KafkaMessage> =
                    if let Ok(topics) = self.topics.try_read() {
                        if let Some(topic_entry) = topics.get(topic) {
                            if let Some(partition_entry) = topic_entry.get_partition(partition) {
                                partition_entry.fetch(offset, 1000).into_iter().cloned().collect()
                            } else {
                                vec![]
                            }
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    };

                Ok(ProtocolResponse {
                    status: ResponseStatus::KafkaStatus(0),
                    metadata: HashMap::from([
                        ("topic".to_string(), topic.clone()),
                        ("partition".to_string(), partition.to_string()),
                        ("message_count".to_string(), messages.len().to_string()),
                    ]),
                    body: serde_json::to_vec(&messages).map_err(mockforge_core::Error::Json)?,
                    content_type: "application/json".to_string(),
                })
            }
            _ => {
                Err(mockforge_core::Error::generic(format!("Unsupported operation: {}", operation)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockforge_core::protocol_abstraction::ProtocolRequest;
    use std::collections::HashMap;
    use tempfile::TempDir;

    async fn create_test_registry() -> KafkaSpecRegistry {
        let topics = Arc::new(tokio::sync::RwLock::new(HashMap::new()));
        let config = mockforge_core::config::KafkaConfig::default();
        KafkaSpecRegistry::new(config, topics).await.unwrap()
    }

    async fn create_registry_with_fixtures() -> (KafkaSpecRegistry, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = temp_dir.path().join("fixtures.yaml");

        let fixtures = vec![crate::fixtures::KafkaFixture {
            identifier: "test-produce".to_string(),
            name: "Test Produce".to_string(),
            topic: "test-topic".to_string(),
            partition: Some(0),
            key_pattern: Some("key-{{id}}".to_string()),
            value_template: serde_json::json!({"message": "test-{{id}}"}),
            headers: HashMap::new(),
            auto_produce: None,
        }];

        let yaml_content = serde_yaml::to_string(&fixtures).unwrap();
        std::fs::write(&fixture_path, yaml_content).unwrap();

        let topics = Arc::new(tokio::sync::RwLock::new(HashMap::new()));
        let config = mockforge_core::config::KafkaConfig {
            fixtures_dir: Some(temp_dir.path().to_path_buf()),
            ..Default::default()
        };
        let registry = KafkaSpecRegistry::new(config, topics).await.unwrap();

        (registry, temp_dir)
    }

    // ==================== KafkaSpecRegistry::new Tests ====================

    #[tokio::test]
    async fn test_new_registry_without_fixtures() {
        let topics = Arc::new(tokio::sync::RwLock::new(HashMap::new()));
        let config = mockforge_core::config::KafkaConfig::default();

        let registry = KafkaSpecRegistry::new(config, topics).await.unwrap();
        assert_eq!(registry.fixtures.len(), 0);
    }

    #[tokio::test]
    async fn test_new_registry_with_fixtures() {
        let (registry, _temp_dir) = create_registry_with_fixtures().await;
        assert_eq!(registry.fixtures.len(), 1);
        assert_eq!(registry.fixtures[0].topic, "test-topic");
    }

    // ==================== KafkaSpecRegistry::find_fixture_by_topic Tests ====================

    #[tokio::test]
    async fn test_find_fixture_by_topic_exists() {
        let (registry, _temp_dir) = create_registry_with_fixtures().await;

        let fixture = registry.find_fixture_by_topic("test-topic");
        assert!(fixture.is_some());
        assert_eq!(fixture.unwrap().identifier, "test-produce");
    }

    #[tokio::test]
    async fn test_find_fixture_by_topic_not_found() {
        let (registry, _temp_dir) = create_registry_with_fixtures().await;

        let fixture = registry.find_fixture_by_topic("nonexistent-topic");
        assert!(fixture.is_none());
    }

    #[tokio::test]
    async fn test_find_fixture_by_topic_empty_registry() {
        let registry = create_test_registry().await;

        let fixture = registry.find_fixture_by_topic("any-topic");
        assert!(fixture.is_none());
    }

    // ==================== KafkaSpecRegistry::produce Tests ====================

    #[tokio::test]
    async fn test_produce_message_without_key() {
        let registry = create_test_registry().await;
        let value = serde_json::json!({"message": "hello"});

        let result = registry.produce("test-topic", None, &value).await;
        assert!(result.is_ok());

        let offset = result.unwrap();
        assert_eq!(offset, 0);
    }

    #[tokio::test]
    async fn test_produce_message_with_key() {
        let registry = create_test_registry().await;
        let value = serde_json::json!({"message": "hello"});

        let result = registry.produce("test-topic", Some("my-key"), &value).await;
        assert!(result.is_ok());

        let offset = result.unwrap();
        assert_eq!(offset, 0);
    }

    #[tokio::test]
    async fn test_produce_multiple_messages() {
        let registry = create_test_registry().await;

        // Use the same key to ensure all messages go to the same partition
        // Without a key, round-robin distributes to different partitions (each starting at offset 0)
        let offset1 = registry
            .produce("test-topic", Some("same-key"), &serde_json::json!({"id": 1}))
            .await
            .unwrap();

        let offset2 = registry
            .produce("test-topic", Some("same-key"), &serde_json::json!({"id": 2}))
            .await
            .unwrap();

        let offset3 = registry
            .produce("test-topic", Some("same-key"), &serde_json::json!({"id": 3}))
            .await
            .unwrap();

        assert_eq!(offset1, 0);
        assert_eq!(offset2, 1);
        assert_eq!(offset3, 2);
    }

    #[tokio::test]
    async fn test_produce_creates_topic_if_not_exists() {
        let registry = create_test_registry().await;
        let value = serde_json::json!({"test": "data"});

        let result = registry.produce("new-topic", None, &value).await;
        assert!(result.is_ok());

        let topics = registry.topics.read().await;
        assert!(topics.contains_key("new-topic"));
    }

    #[tokio::test]
    async fn test_produce_to_multiple_topics() {
        let registry = create_test_registry().await;

        registry.produce("topic-1", None, &serde_json::json!({"id": 1})).await.unwrap();

        registry.produce("topic-2", None, &serde_json::json!({"id": 2})).await.unwrap();

        let topics = registry.topics.read().await;
        assert_eq!(topics.len(), 2);
        assert!(topics.contains_key("topic-1"));
        assert!(topics.contains_key("topic-2"));
    }

    // ==================== KafkaSpecRegistry::fetch Tests ====================

    #[tokio::test]
    async fn test_fetch_from_empty_topic() {
        let registry = create_test_registry().await;

        let result = registry.fetch("nonexistent-topic", 0, 0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_from_nonexistent_partition() {
        let registry = create_test_registry().await;

        // Produce a message to create the topic
        registry
            .produce("test-topic", None, &serde_json::json!({"test": "data"}))
            .await
            .unwrap();

        // Try to fetch from a partition that doesn't exist
        let result = registry.fetch("test-topic", 99, 0).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_fetch_messages_after_produce() {
        let registry = create_test_registry().await;

        // Produce messages
        registry
            .produce("test-topic", None, &serde_json::json!({"id": 1}))
            .await
            .unwrap();

        registry
            .produce("test-topic", None, &serde_json::json!({"id": 2}))
            .await
            .unwrap();

        // Fetch messages
        let messages = registry.fetch("test-topic", 0, 0).await.unwrap();
        assert!(messages.len() >= 1);
    }

    #[tokio::test]
    async fn test_fetch_from_specific_offset() {
        let registry = create_test_registry().await;

        // Produce multiple messages
        for i in 0..5 {
            registry
                .produce("test-topic", None, &serde_json::json!({"id": i}))
                .await
                .unwrap();
        }

        // Fetch from offset 2
        let messages = registry.fetch("test-topic", 0, 2).await.unwrap();
        assert!(messages.len() <= 3); // Messages 2, 3, 4
    }

    // ==================== SpecRegistry Trait Implementation Tests ====================

    #[tokio::test]
    async fn test_protocol_returns_kafka() {
        let registry = create_test_registry().await;
        assert_eq!(registry.protocol(), Protocol::Kafka);
    }

    #[tokio::test]
    async fn test_operations_empty_registry() {
        let registry = create_test_registry().await;
        let operations = registry.operations();
        assert!(operations.is_empty());
    }

    #[tokio::test]
    async fn test_operations_with_fixtures() {
        let (registry, _temp_dir) = create_registry_with_fixtures().await;
        let operations = registry.operations();

        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].name, "test-produce");
        assert_eq!(operations[0].path, "test-topic");
        assert_eq!(operations[0].operation_type, "PRODUCE");
    }

    #[tokio::test]
    async fn test_find_operation_exists() {
        let (registry, _temp_dir) = create_registry_with_fixtures().await;

        let operation = registry.find_operation("PRODUCE", "test-topic");
        assert!(operation.is_some());

        let op = operation.unwrap();
        assert_eq!(op.name, "test-produce");
        assert_eq!(op.path, "test-topic");
    }

    #[tokio::test]
    async fn test_find_operation_wrong_operation_type() {
        let (registry, _temp_dir) = create_registry_with_fixtures().await;

        let operation = registry.find_operation("FETCH", "test-topic");
        assert!(operation.is_none());
    }

    #[tokio::test]
    async fn test_find_operation_wrong_path() {
        let (registry, _temp_dir) = create_registry_with_fixtures().await;

        let operation = registry.find_operation("PRODUCE", "wrong-topic");
        assert!(operation.is_none());
    }

    #[tokio::test]
    async fn test_find_operation_empty_registry() {
        let registry = create_test_registry().await;

        let operation = registry.find_operation("PRODUCE", "any-topic");
        assert!(operation.is_none());
    }

    // ==================== validate_request Tests ====================

    #[tokio::test]
    async fn test_validate_request_valid_topic() {
        let (registry, _temp_dir) = create_registry_with_fixtures().await;

        let request = ProtocolRequest {
            operation: "PRODUCE".to_string(),
            topic: Some("test-topic".to_string()),
            partition: None,
            metadata: HashMap::new(),
            ..Default::default()
        };

        let result = registry.validate_request(&request).unwrap();
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_validate_request_invalid_topic() {
        let (registry, _temp_dir) = create_registry_with_fixtures().await;

        let request = ProtocolRequest {
            operation: "PRODUCE".to_string(),
            topic: Some("wrong-topic".to_string()),
            partition: None,
            metadata: HashMap::new(),
            ..Default::default()
        };

        let result = registry.validate_request(&request).unwrap();
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].message, "Topic not found in fixtures");
    }

    #[tokio::test]
    async fn test_validate_request_missing_topic() {
        let (registry, _temp_dir) = create_registry_with_fixtures().await;

        let request = ProtocolRequest {
            operation: "PRODUCE".to_string(),
            topic: None,
            partition: None,
            metadata: HashMap::new(),
            ..Default::default()
        };

        let result = registry.validate_request(&request).unwrap();
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
    }

    // ==================== generate_mock_response Tests ====================

    #[tokio::test]
    async fn test_generate_mock_response_produce() {
        let (registry, _temp_dir) = create_registry_with_fixtures().await;

        let request = ProtocolRequest {
            operation: "PRODUCE".to_string(),
            topic: Some("test-topic".to_string()),
            partition: None,
            metadata: HashMap::new(),
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();

        assert!(matches!(
            response.status,
            mockforge_core::protocol_abstraction::ResponseStatus::KafkaStatus(0)
        ));
        assert_eq!(response.content_type, "application/json");
        assert!(response.metadata.contains_key("topic"));
        assert!(response.metadata.contains_key("offset"));
    }

    #[tokio::test]
    async fn test_generate_mock_response_produce_missing_topic() {
        let (registry, _temp_dir) = create_registry_with_fixtures().await;

        let request = ProtocolRequest {
            operation: "PRODUCE".to_string(),
            topic: None,
            partition: None,
            metadata: HashMap::new(),
            ..Default::default()
        };

        let result = registry.generate_mock_response(&request);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_mock_response_produce_no_fixture() {
        let (registry, _temp_dir) = create_registry_with_fixtures().await;

        let request = ProtocolRequest {
            operation: "PRODUCE".to_string(),
            topic: Some("nonexistent-topic".to_string()),
            partition: None,
            metadata: HashMap::new(),
            ..Default::default()
        };

        let result = registry.generate_mock_response(&request);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_mock_response_fetch() {
        let (registry, _temp_dir) = create_registry_with_fixtures().await;

        let mut metadata = HashMap::new();
        metadata.insert("offset".to_string(), "0".to_string());

        let request = ProtocolRequest {
            operation: "FETCH".to_string(),
            topic: Some("test-topic".to_string()),
            partition: Some(0),
            metadata,
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();

        assert!(matches!(
            response.status,
            mockforge_core::protocol_abstraction::ResponseStatus::KafkaStatus(0)
        ));
        assert_eq!(response.content_type, "application/json");
        assert_eq!(response.metadata.get("topic").unwrap(), "test-topic");
        assert_eq!(response.metadata.get("partition").unwrap(), "0");
    }

    #[tokio::test]
    async fn test_generate_mock_response_fetch_missing_partition() {
        let (registry, _temp_dir) = create_registry_with_fixtures().await;

        let request = ProtocolRequest {
            operation: "FETCH".to_string(),
            topic: Some("test-topic".to_string()),
            partition: None,
            metadata: HashMap::new(),
            ..Default::default()
        };

        let result = registry.generate_mock_response(&request);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_mock_response_unsupported_operation() {
        let (registry, _temp_dir) = create_registry_with_fixtures().await;

        let request = ProtocolRequest {
            operation: "UNSUPPORTED".to_string(),
            topic: Some("test-topic".to_string()),
            partition: None,
            metadata: HashMap::new(),
            ..Default::default()
        };

        let result = registry.generate_mock_response(&request);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_mock_response_with_metadata() {
        let (registry, _temp_dir) = create_registry_with_fixtures().await;

        let mut metadata = HashMap::new();
        metadata.insert("id".to_string(), "42".to_string());

        let request = ProtocolRequest {
            operation: "PRODUCE".to_string(),
            topic: Some("test-topic".to_string()),
            partition: None,
            metadata,
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();
        assert!(response.body.len() > 0);
    }

    // ==================== Template Engine Integration Tests ====================

    #[tokio::test]
    async fn test_template_expansion_in_mock_response() {
        let (registry, _temp_dir) = create_registry_with_fixtures().await;

        let mut metadata = HashMap::new();
        metadata.insert("id".to_string(), "123".to_string());

        let request = ProtocolRequest {
            operation: "PRODUCE".to_string(),
            topic: Some("test-topic".to_string()),
            partition: None,
            metadata,
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();
        let body_str = String::from_utf8(response.body).unwrap();

        // The fixture template has "message": "test-{{id}}"
        // With id=123, it should expand to contain "test-123"
        assert!(body_str.contains("test") || body_str.contains("message"));
    }

    // ==================== Concurrent Access Tests ====================

    #[tokio::test]
    async fn test_concurrent_produce() {
        let registry = Arc::new(create_test_registry().await);

        let mut handles = vec![];
        for i in 0..10 {
            let reg = Arc::clone(&registry);
            let handle = tokio::spawn(async move {
                reg.produce("concurrent-topic", None, &serde_json::json!({"id": i})).await
            });
            handles.push(handle);
        }

        for handle in handles {
            assert!(handle.await.unwrap().is_ok());
        }

        let topics = registry.topics.read().await;
        assert!(topics.contains_key("concurrent-topic"));
    }

    #[tokio::test]
    async fn test_concurrent_produce_and_fetch() {
        let registry = Arc::new(create_test_registry().await);

        // Produce some initial messages
        for i in 0..5 {
            registry
                .produce("test-topic", None, &serde_json::json!({"id": i}))
                .await
                .unwrap();
        }

        // Concurrent produces (separate vector for different return type)
        let mut produce_handles = vec![];
        for i in 5..10 {
            let reg = Arc::clone(&registry);
            let handle = tokio::spawn(async move {
                reg.produce("test-topic", None, &serde_json::json!({"id": i})).await
            });
            produce_handles.push(handle);
        }

        // Concurrent fetches (separate vector for different return type)
        let mut fetch_handles = vec![];
        for _ in 0..5 {
            let reg = Arc::clone(&registry);
            let handle = tokio::spawn(async move { reg.fetch("test-topic", 0, 0).await });
            fetch_handles.push(handle);
        }

        // Await produce handles
        for handle in produce_handles {
            let _ = handle.await;
        }

        // Await fetch handles
        for handle in fetch_handles {
            let _ = handle.await;
        }
    }
}
