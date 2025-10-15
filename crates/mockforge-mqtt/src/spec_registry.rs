use mockforge_core::protocol_abstraction::{SpecRegistry, ProtocolRequest, ProtocolResponse, ResponseStatus, SpecOperation, ValidationResult, ValidationError};
use mockforge_core::{Protocol, Result, Error, templating};
use std::path::Path;
use tracing::{debug, warn};

use crate::fixtures::MqttFixtureRegistry;

/// MQTT implementation of SpecRegistry
pub struct MqttSpecRegistry {
    fixture_registry: MqttFixtureRegistry,
}

impl MqttSpecRegistry {
    pub fn new() -> Self {
        Self {
            fixture_registry: MqttFixtureRegistry::new(),
        }
    }

    pub fn add_fixture(&mut self, fixture: crate::fixtures::MqttFixture) {
        self.fixture_registry.add_fixture(fixture);
    }

    pub fn find_fixture_by_topic(&self, topic: &str) -> Option<&crate::fixtures::MqttFixture> {
        self.fixture_registry.find_by_topic(topic)
    }

    /// Load fixtures from a directory
    pub fn load_fixtures<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();

        if !path.exists() {
            warn!("Fixtures directory does not exist: {:?}", path);
            return Ok(());
        }

        let entries = std::fs::read_dir(path)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let extension = path.extension().and_then(|s| s.to_str());

                match extension {
                    Some("yaml") | Some("yml") => {
                        self.load_fixture_file(&path)?;
                    }
                    Some("json") => {
                        self.load_fixture_file_json(&path)?;
                    }
                    _ => {
                        debug!("Skipping non-fixture file: {:?}", path);
                    }
                }
            }
        }

        Ok(())
    }

    fn load_fixture_file(&mut self, path: &Path) -> Result<()> {
        use std::fs;

        let content = fs::read_to_string(path)?;
        let fixture: crate::fixtures::MqttFixture = serde_yaml::from_str(&content)?;
        self.add_fixture(fixture);
        Ok(())
    }

    fn load_fixture_file_json(&mut self, path: &Path) -> Result<()> {
        use std::fs;

        let content = fs::read_to_string(path)?;
        let fixture: crate::fixtures::MqttFixture = serde_json::from_str(&content)?;
        self.add_fixture(fixture);
        Ok(())
    }
}

impl SpecRegistry for MqttSpecRegistry {
    fn protocol(&self) -> Protocol {
        Protocol::Mqtt
    }

    fn operations(&self) -> Vec<SpecOperation> {
        self.fixture_registry.fixtures().map(|fixture| {
            SpecOperation {
                name: fixture.identifier.clone(),
                path: fixture.topic_pattern.clone(),
                operation_type: "PUBLISH".to_string(),
                input_schema: None,
                output_schema: Some(serde_json::to_string(&fixture.response.payload).unwrap_or_default()),
                metadata: std::collections::HashMap::new(),
            }
        }).collect()
    }

    fn find_operation(&self, _operation: &str, path: &str) -> Option<SpecOperation> {
        self.find_fixture_by_topic(path).map(|fixture| {
            SpecOperation {
                name: fixture.identifier.clone(),
                path: fixture.topic_pattern.clone(),
                operation_type: "PUBLISH".to_string(),
                input_schema: None,
                output_schema: Some(serde_json::to_string(&fixture.response.payload).unwrap_or_default()),
                metadata: std::collections::HashMap::new(),
            }
        })
    }

    fn validate_request(&self, request: &ProtocolRequest) -> Result<ValidationResult> {
        let topic = request.topic.as_ref();

        if topic.is_none() {
            return Ok(ValidationResult::failure(vec![ValidationError {
                message: "Missing topic in MQTT request".to_string(),
                path: Some("topic".to_string()),
                code: Some("MISSING_TOPIC".to_string()),
            }]));
        }

        let topic = topic.unwrap();
        let valid = self.find_fixture_by_topic(topic).is_some();

        if valid {
            Ok(ValidationResult::success())
        } else {
            Ok(ValidationResult::failure(vec![ValidationError {
                message: format!("No fixture found for topic: {}", topic),
                path: Some("topic".to_string()),
                code: Some("NO_FIXTURE".to_string()),
            }]))
        }
    }

    fn generate_mock_response(&self, request: &ProtocolRequest) -> Result<ProtocolResponse> {
        let topic = request.topic.as_ref()
            .ok_or_else(|| Error::Validation { message: "Missing topic".to_string() })?;

        let fixture = self.find_fixture_by_topic(topic)
            .ok_or_else(|| Error::Routing { message: format!("No fixture found for topic: {}", topic) })?;

        // Create templating context with environment variables
        let mut env_vars = std::collections::HashMap::new();
        env_vars.insert("topic".to_string(), topic.clone());

        let context = templating::TemplatingContext::with_env(env_vars);

        // Use template engine to render payload
        let template_str = serde_json::to_string(&fixture.response.payload)
            .map_err(|e| Error::Json(e))?;
        let expanded_payload = templating::expand_str_with_context(&template_str, &context);
        let payload = expanded_payload.into_bytes();

        Ok(ProtocolResponse {
            status: ResponseStatus::MqttStatus(true),
            metadata: std::collections::HashMap::from([
                ("topic".to_string(), topic.clone()),
                ("qos".to_string(), request.qos.unwrap_or(0).to_string()),
                ("retained".to_string(), fixture.retained.to_string()),
            ]),
            body: payload,
            content_type: "application/json".to_string(),
        })
    }
}
