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
    topics: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, crate::topics::Topic>>,
    >,
}

impl KafkaSpecRegistry {
    /// Create a new Kafka spec registry
    pub async fn new(
        config: mockforge_core::config::KafkaConfig,
        topics: std::sync::Arc<
            tokio::sync::RwLock<std::collections::HashMap<String, crate::topics::Topic>>,
        >,
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
                metadata: std::collections::HashMap::new(),
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
                metadata: std::collections::HashMap::new(),
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

                // For now, return a mock offset since we don't have actual broker integration
                let offset = 0i64;

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
                let _offset =
                    request.metadata.get("offset").and_then(|s| s.parse().ok()).unwrap_or(0);

                // For now, return empty messages since we don't have actual broker integration
                let messages: Vec<crate::partitions::KafkaMessage> = vec![];

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
