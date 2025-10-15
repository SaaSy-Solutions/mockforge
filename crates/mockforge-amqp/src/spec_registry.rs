use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use mockforge_core::protocol_abstraction::{
    ProtocolRequest, ProtocolResponse, ResponseStatus, SpecOperation, SpecRegistry,
    ValidationError, ValidationResult,
};
use mockforge_core::{Protocol, Result};

/// AMQP-specific spec registry implementation
#[derive(Debug)]
pub struct AmqpSpecRegistry {
    fixtures: Vec<Arc<crate::fixtures::AmqpFixture>>,
    template_engine: mockforge_core::templating::TemplateEngine,
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
        })
    }

    /// Find fixture by queue name
    pub fn find_fixture_for_queue(&self, queue: &str) -> Option<Arc<crate::fixtures::AmqpFixture>> {
        self.fixtures.iter().find(|f| {
            f.queues.iter().any(|q| q.name == queue)
        }).cloned()
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
            .and_then(|fixture| {
                match operation {
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
                }
            })
    }

    fn validate_request(&self, request: &ProtocolRequest) -> Result<ValidationResult> {
        let valid = match request.operation.as_str() {
            "PUBLISH" => {
                if let Some(exchange) = request.metadata.get("exchange") {
                    self.fixtures.iter().any(|f| {
                        f.exchanges.iter().any(|e| e.name == *exchange)
                    })
                } else {
                    false
                }
            }
            "CONSUME" => {
                self.fixtures.iter().any(|f| {
                    f.queues.iter().any(|q| q.name == request.path)
                })
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
                let exchange = request.metadata.get("exchange")
                    .ok_or_else(|| mockforge_core::Error::generic("Missing exchange"))?;
                let routing_key = request.routing_key.as_ref()
                    .ok_or_else(|| mockforge_core::Error::generic("Missing routing key"))?;

                // For now, just acknowledge the publish
                Ok(ProtocolResponse {
                    status: ResponseStatus::AmqpStatus(200),
                    metadata: HashMap::from([
                        ("exchange".to_string(), exchange.clone()),
                        ("routing_key".to_string(), routing_key.clone()),
                    ]),
                    body: vec![],
                    content_type: "application/octet-stream".to_string(),
                })
            }
            "CONSUME" => {
                let queue = &request.path;

                // For now, return empty response
                Ok(ProtocolResponse {
                    status: ResponseStatus::AmqpStatus(200),
                    metadata: HashMap::from([
                        ("queue".to_string(), queue.clone()),
                    ]),
                    body: vec![],
                    content_type: "application/octet-stream".to_string(),
                })
            }
            _ => Err(mockforge_core::Error::generic(format!("Unsupported operation: {}", operation))),
        }
    }
}