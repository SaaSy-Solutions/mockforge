use async_trait::async_trait;
use serde_json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::exchanges::ExchangeManager;
use crate::messages::{Message, MessageProperties, QueuedMessage};
use crate::queues::QueueManager;
use mockforge_core::protocol_abstraction::{
    ProtocolRequest, ProtocolResponse, ResponseStatus, SpecOperation, SpecRegistry,
    ValidationError, ValidationResult,
};
use mockforge_core::{Protocol, Result};

/// AMQP-specific spec registry implementation
pub struct AmqpSpecRegistry {
    fixtures: Vec<Arc<crate::fixtures::AmqpFixture>>,
    template_engine: mockforge_core::templating::TemplateEngine,
    exchanges: std::sync::OnceLock<Arc<RwLock<ExchangeManager>>>,
    queues: std::sync::OnceLock<Arc<RwLock<QueueManager>>>,
}

impl std::fmt::Debug for AmqpSpecRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AmqpSpecRegistry")
            .field("fixtures", &self.fixtures.len())
            .finish()
    }
}

impl AmqpSpecRegistry {
    /// Create a new AMQP spec registry
    pub async fn new(config: mockforge_core::config::AmqpConfig) -> Result<Self> {
        let fixtures = if let Some(fixtures_dir) = &config.fixtures_dir {
            crate::fixtures::AmqpFixture::load_from_dir(fixtures_dir)?
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
            exchanges: std::sync::OnceLock::new(),
            queues: std::sync::OnceLock::new(),
        })
    }

    /// Set the exchange and queue managers for broker integration.
    /// Can be called on a shared `&self` reference (thread-safe, set-once).
    pub fn set_broker_managers(
        &self,
        exchanges: Arc<RwLock<ExchangeManager>>,
        queues: Arc<RwLock<QueueManager>>,
    ) {
        let _ = self.exchanges.set(exchanges);
        let _ = self.queues.set(queues);
    }

    /// Find fixture by queue name
    pub fn find_fixture_for_queue(&self, queue: &str) -> Option<Arc<crate::fixtures::AmqpFixture>> {
        self.fixtures.iter().find(|f| f.queues.iter().any(|q| q.name == queue)).cloned()
    }
}

#[async_trait]
impl SpecRegistry for AmqpSpecRegistry {
    fn protocol(&self) -> Protocol {
        Protocol::Amqp
    }

    fn operations(&self) -> Vec<SpecOperation> {
        self.fixtures
            .iter()
            .flat_map(|fixture| {
                vec![
                    SpecOperation {
                        name: format!("{}-publish", fixture.identifier),
                        path: fixture.identifier.clone(),
                        operation_type: "PUBLISH".to_string(),
                        input_schema: Some("AmqpMessage".to_string()),
                        output_schema: Some("PublishResponse".to_string()),
                        metadata: HashMap::new(),
                    },
                    SpecOperation {
                        name: format!("{}-consume", fixture.identifier),
                        path: fixture.identifier.clone(),
                        operation_type: "CONSUME".to_string(),
                        input_schema: Some("ConsumeRequest".to_string()),
                        output_schema: Some("AmqpMessage".to_string()),
                        metadata: HashMap::new(),
                    },
                ]
            })
            .collect()
    }

    fn find_operation(&self, operation: &str, path: &str) -> Option<SpecOperation> {
        self.fixtures
            .iter()
            .find(|fixture| fixture.identifier == path)
            .and_then(|fixture| match operation {
                "PUBLISH" => Some(SpecOperation {
                    name: format!("{}-publish", fixture.identifier),
                    path: fixture.identifier.clone(),
                    operation_type: "PUBLISH".to_string(),
                    input_schema: Some("AmqpMessage".to_string()),
                    output_schema: Some("PublishResponse".to_string()),
                    metadata: HashMap::new(),
                }),
                "CONSUME" => Some(SpecOperation {
                    name: format!("{}-consume", fixture.identifier),
                    path: fixture.identifier.clone(),
                    operation_type: "CONSUME".to_string(),
                    input_schema: Some("ConsumeRequest".to_string()),
                    output_schema: Some("AmqpMessage".to_string()),
                    metadata: HashMap::new(),
                }),
                _ => None,
            })
    }

    fn validate_request(&self, request: &ProtocolRequest) -> Result<ValidationResult> {
        let valid = match request.operation.as_str() {
            "PUBLISH" => {
                if let Some(exchange) = request.metadata.get("exchange") {
                    self.fixtures.iter().any(|f| f.exchanges.iter().any(|e| e.name == *exchange))
                } else {
                    false
                }
            }
            "CONSUME" => {
                self.fixtures.iter().any(|f| f.queues.iter().any(|q| q.name == request.path))
            }
            _ => false,
        };

        Ok(ValidationResult {
            valid,
            errors: if valid {
                vec![]
            } else {
                vec![ValidationError {
                    message: "Invalid AMQP operation or target not found".to_string(),
                    path: Some("operation".to_string()),
                    code: Some("INVALID_OPERATION".to_string()),
                }]
            },
            warnings: vec![],
        })
    }

    fn generate_mock_response(&self, request: &ProtocolRequest) -> Result<ProtocolResponse> {
        let operation = &request.operation;

        match operation.as_str() {
            "PUBLISH" => {
                let exchange_name = request
                    .metadata
                    .get("exchange")
                    .ok_or_else(|| mockforge_core::Error::generic("Missing exchange"))?;
                let routing_key = request
                    .routing_key
                    .as_ref()
                    .ok_or_else(|| mockforge_core::Error::generic("Missing routing key"))?;

                let body_bytes = request.body.clone().unwrap_or_default();

                // Route through exchange to target queues if broker is wired
                let mut routed_queues = Vec::new();
                if let (Some(exchanges), Some(queues)) = (self.exchanges.get(), self.queues.get()) {
                    let message = Message {
                        properties: MessageProperties::default(),
                        body: body_bytes,
                        routing_key: routing_key.clone(),
                    };

                    // Look up exchange and route
                    let target_queues = if let Ok(exchanges_guard) = exchanges.try_read() {
                        if let Some(exchange) = exchanges_guard.get_exchange(exchange_name) {
                            exchange.route_message(&message, routing_key)
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    };

                    // Enqueue to each target queue
                    if let Ok(mut queues_guard) = queues.try_write() {
                        for queue_name in &target_queues {
                            let queued_msg = QueuedMessage::new(message.clone());
                            let _ = queues_guard.enqueue_and_notify(queue_name, queued_msg);
                        }
                    }
                    routed_queues = target_queues;
                }

                Ok(ProtocolResponse {
                    status: ResponseStatus::AmqpStatus(200),
                    metadata: HashMap::from([
                        ("exchange".to_string(), exchange_name.clone()),
                        ("routing_key".to_string(), routing_key.clone()),
                        ("routed_queues".to_string(), routed_queues.join(",")),
                    ]),
                    body: vec![],
                    content_type: "application/octet-stream".to_string(),
                })
            }
            "CONSUME" => {
                let queue = &request.path;

                // Try to dequeue a real message from the broker first
                let dequeued = if let Some(queues) = self.queues.get() {
                    if let Ok(mut queues_guard) = queues.try_write() {
                        queues_guard.get_queue_mut(queue).and_then(|q| q.dequeue())
                    } else {
                        None
                    }
                } else {
                    None
                };

                let body = if let Some(queued_msg) = dequeued {
                    // Return the real dequeued message body
                    queued_msg.message.body
                } else {
                    // Fall back to template-generated message from fixtures
                    let fixture = self.find_fixture_for_queue(queue);
                    let queue_config =
                        fixture.as_ref().and_then(|f| f.queues.iter().find(|q| q.name == *queue));

                    if let Some(queue_config) = queue_config {
                        if let Some(message_template) = &queue_config.message_template {
                            let templating_context =
                                mockforge_core::templating::TemplatingContext::with_env(
                                    request.metadata.clone(),
                                );
                            let expanded = self
                                .template_engine
                                .expand_tokens_with_context(message_template, &templating_context);
                            serde_json::to_string(&expanded)
                                .map_err(mockforge_core::Error::Json)?
                                .into_bytes()
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    }
                };

                Ok(ProtocolResponse {
                    status: ResponseStatus::AmqpStatus(200),
                    metadata: HashMap::from([("queue".to_string(), queue.clone())]),
                    body,
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
    use mockforge_core::config::AmqpConfig;
    use mockforge_core::protocol_abstraction::{MessagePattern, SpecRegistry};
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_config() -> AmqpConfig {
        AmqpConfig {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: 5672,
            ..Default::default()
        }
    }

    fn create_protocol_request(
        operation: &str,
        path: &str,
        routing_key: Option<String>,
        metadata: HashMap<String, String>,
    ) -> ProtocolRequest {
        ProtocolRequest {
            protocol: Protocol::Amqp,
            pattern: MessagePattern::PubSub,
            operation: operation.to_string(),
            path: path.to_string(),
            topic: None,
            routing_key,
            partition: None,
            qos: None,
            metadata,
            body: None,
            client_ip: None,
        }
    }

    fn create_test_config_with_fixtures() -> (AmqpConfig, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = temp_dir.path().join("test-fixture.yaml");

        let yaml_content = r#"
identifier: test-fixture
name: Test Fixture
exchanges:
  - name: test-exchange
    type: direct
    durable: true
queues:
  - name: test-queue
    durable: true
    message_template:
      message: "Hello {{name}}"
      timestamp: "{{timestamp}}"
bindings:
  - exchange: test-exchange
    queue: test-queue
    routing_key: test.key
"#;

        let mut file = std::fs::File::create(&fixture_path).unwrap();
        file.write_all(yaml_content.as_bytes()).unwrap();

        let config = AmqpConfig {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: 5672,
            fixtures_dir: Some(temp_dir.path().to_path_buf()),
            ..Default::default()
        };

        (config, temp_dir)
    }

    #[tokio::test]
    async fn test_amqp_spec_registry_new() {
        let config = create_test_config();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();
        assert_eq!(registry.fixtures.len(), 0);
    }

    #[tokio::test]
    async fn test_amqp_spec_registry_with_fixtures() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();
        assert_eq!(registry.fixtures.len(), 1);
    }

    #[tokio::test]
    async fn test_protocol() {
        let config = create_test_config();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();
        assert_eq!(registry.protocol(), Protocol::Amqp);
    }

    #[tokio::test]
    async fn test_operations_empty() {
        let config = create_test_config();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();
        let operations = registry.operations();
        assert!(operations.is_empty());
    }

    #[tokio::test]
    async fn test_operations_with_fixture() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();
        let operations = registry.operations();

        // Each fixture generates two operations: PUBLISH and CONSUME
        assert_eq!(operations.len(), 2);
        assert!(operations.iter().any(|op| op.operation_type == "PUBLISH"));
        assert!(operations.iter().any(|op| op.operation_type == "CONSUME"));
    }

    #[tokio::test]
    async fn test_find_operation_publish() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let operation = registry.find_operation("PUBLISH", "test-fixture");
        assert!(operation.is_some());

        let op = operation.unwrap();
        assert_eq!(op.operation_type, "PUBLISH");
        assert_eq!(op.name, "test-fixture-publish");
        assert_eq!(op.input_schema, Some("AmqpMessage".to_string()));
        assert_eq!(op.output_schema, Some("PublishResponse".to_string()));
    }

    #[tokio::test]
    async fn test_find_operation_consume() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let operation = registry.find_operation("CONSUME", "test-fixture");
        assert!(operation.is_some());

        let op = operation.unwrap();
        assert_eq!(op.operation_type, "CONSUME");
        assert_eq!(op.name, "test-fixture-consume");
        assert_eq!(op.input_schema, Some("ConsumeRequest".to_string()));
        assert_eq!(op.output_schema, Some("AmqpMessage".to_string()));
    }

    #[tokio::test]
    async fn test_find_operation_invalid() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let operation = registry.find_operation("INVALID", "test-fixture");
        assert!(operation.is_none());
    }

    #[tokio::test]
    async fn test_find_operation_nonexistent_fixture() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let operation = registry.find_operation("PUBLISH", "nonexistent");
        assert!(operation.is_none());
    }

    #[tokio::test]
    async fn test_find_fixture_for_queue() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let fixture = registry.find_fixture_for_queue("test-queue");
        assert!(fixture.is_some());
        assert_eq!(fixture.unwrap().identifier, "test-fixture");
    }

    #[tokio::test]
    async fn test_find_fixture_for_queue_nonexistent() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let fixture = registry.find_fixture_for_queue("nonexistent-queue");
        assert!(fixture.is_none());
    }

    #[tokio::test]
    async fn test_validate_request_publish_valid() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let request = create_protocol_request(
            "PUBLISH",
            "test-fixture",
            Some("test.key".to_string()),
            HashMap::from([("exchange".to_string(), "test-exchange".to_string())]),
        );

        let result = registry.validate_request(&request).unwrap();
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_validate_request_publish_invalid_exchange() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let request = create_protocol_request(
            "PUBLISH",
            "test-fixture",
            Some("test.key".to_string()),
            HashMap::from([("exchange".to_string(), "nonexistent-exchange".to_string())]),
        );

        let result = registry.validate_request(&request).unwrap();
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].code, Some("INVALID_OPERATION".to_string()));
    }

    #[tokio::test]
    async fn test_validate_request_publish_missing_exchange() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let request = create_protocol_request(
            "PUBLISH",
            "test-fixture",
            Some("test.key".to_string()),
            HashMap::new(),
        );

        let result = registry.validate_request(&request).unwrap();
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
    }

    #[tokio::test]
    async fn test_validate_request_consume_valid() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let request = create_protocol_request("CONSUME", "test-queue", None, HashMap::new());

        let result = registry.validate_request(&request).unwrap();
        assert!(result.valid);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_validate_request_consume_invalid_queue() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let request = create_protocol_request("CONSUME", "nonexistent-queue", None, HashMap::new());

        let result = registry.validate_request(&request).unwrap();
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
    }

    #[tokio::test]
    async fn test_validate_request_invalid_operation() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let request = create_protocol_request("INVALID", "test-queue", None, HashMap::new());

        let result = registry.validate_request(&request).unwrap();
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
    }

    #[tokio::test]
    async fn test_generate_mock_response_publish() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let request = create_protocol_request(
            "PUBLISH",
            "test-fixture",
            Some("test.key".to_string()),
            HashMap::from([("exchange".to_string(), "test-exchange".to_string())]),
        );

        let response = registry.generate_mock_response(&request).unwrap();
        match response.status {
            ResponseStatus::AmqpStatus(code) => assert_eq!(code, 200),
            _ => panic!("Expected AmqpStatus"),
        }
        assert_eq!(response.metadata.get("exchange"), Some(&"test-exchange".to_string()));
        assert_eq!(response.metadata.get("routing_key"), Some(&"test.key".to_string()));
    }

    #[tokio::test]
    async fn test_generate_mock_response_publish_missing_exchange() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let request = create_protocol_request(
            "PUBLISH",
            "test-fixture",
            Some("test.key".to_string()),
            HashMap::new(),
        );

        let result = registry.generate_mock_response(&request);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_mock_response_publish_missing_routing_key() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let request = create_protocol_request(
            "PUBLISH",
            "test-fixture",
            None,
            HashMap::from([("exchange".to_string(), "test-exchange".to_string())]),
        );

        let result = registry.generate_mock_response(&request);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_generate_mock_response_consume() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let request = create_protocol_request("CONSUME", "test-queue", None, HashMap::new());

        let response = registry.generate_mock_response(&request).unwrap();
        match response.status {
            ResponseStatus::AmqpStatus(code) => assert_eq!(code, 200),
            _ => panic!("Expected AmqpStatus"),
        }
        assert_eq!(response.metadata.get("queue"), Some(&"test-queue".to_string()));
        assert_eq!(response.content_type, "application/json");
    }

    #[tokio::test]
    async fn test_generate_mock_response_consume_with_template() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let request = create_protocol_request(
            "CONSUME",
            "test-queue",
            None,
            HashMap::from([("name".to_string(), "World".to_string())]),
        );

        let response = registry.generate_mock_response(&request).unwrap();
        assert!(!response.body.is_empty());
        let body_str = String::from_utf8(response.body).unwrap();
        assert!(body_str.contains("Hello") || body_str.contains("message"));
    }

    #[tokio::test]
    async fn test_generate_mock_response_consume_nonexistent_queue() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let request = create_protocol_request("CONSUME", "nonexistent-queue", None, HashMap::new());

        let response = registry.generate_mock_response(&request).unwrap();
        // Should return empty body for nonexistent queue
        assert!(response.body.is_empty());
    }

    #[tokio::test]
    async fn test_generate_mock_response_unsupported_operation() {
        let (config, _temp_dir) = create_test_config_with_fixtures();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();

        let request = create_protocol_request("UNSUPPORTED", "test-queue", None, HashMap::new());

        let result = registry.generate_mock_response(&request);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_spec_registry_debug() {
        let config = create_test_config();
        let registry = AmqpSpecRegistry::new(config).await.unwrap();
        let debug = format!("{:?}", registry);
        assert!(debug.contains("AmqpSpecRegistry"));
    }
}
