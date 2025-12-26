use mockforge_core::protocol_abstraction::{
    ProtocolRequest, ProtocolResponse, ResponseStatus, SpecOperation, SpecRegistry,
    ValidationError, ValidationResult,
};
use mockforge_core::{templating, Error, Protocol, Result};
use std::path::Path;
use tracing::{debug, warn};

use crate::fixtures::MqttFixtureRegistry;

/// MQTT implementation of SpecRegistry
pub struct MqttSpecRegistry {
    fixture_registry: MqttFixtureRegistry,
}

impl Default for MqttSpecRegistry {
    fn default() -> Self {
        Self::new()
    }
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
                        if let Err(e) = self.load_fixture_file(&path) {
                            warn!("Failed to load YAML fixture {:?}: {}", path, e);
                        }
                    }
                    Some("json") => {
                        if let Err(e) = self.load_fixture_file_json(&path) {
                            warn!("Failed to load JSON fixture {:?}: {}", path, e);
                        }
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
        self.fixture_registry
            .fixtures()
            .map(|fixture| SpecOperation {
                name: fixture.identifier.clone(),
                path: fixture.topic_pattern.clone(),
                operation_type: "PUBLISH".to_string(),
                input_schema: None,
                output_schema: Some(
                    serde_json::to_string(&fixture.response.payload).unwrap_or_default(),
                ),
                metadata: std::collections::HashMap::new(),
            })
            .collect()
    }

    fn find_operation(&self, _operation: &str, path: &str) -> Option<SpecOperation> {
        self.find_fixture_by_topic(path).map(|fixture| SpecOperation {
            name: fixture.identifier.clone(),
            path: fixture.topic_pattern.clone(),
            operation_type: "PUBLISH".to_string(),
            input_schema: None,
            output_schema: Some(
                serde_json::to_string(&fixture.response.payload).unwrap_or_default(),
            ),
            metadata: std::collections::HashMap::new(),
        })
    }

    fn validate_request(&self, request: &ProtocolRequest) -> Result<ValidationResult> {
        let topic = request.topic.as_ref();

        let topic = if let Some(t) = topic {
            t
        } else {
            return Ok(ValidationResult::failure(vec![ValidationError {
                message: "Missing topic in MQTT request".to_string(),
                path: Some("topic".to_string()),
                code: Some("MISSING_TOPIC".to_string()),
            }]));
        };
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
        let topic = request.topic.as_ref().ok_or_else(|| Error::Validation {
            message: "Missing topic".to_string(),
        })?;

        let fixture = self.find_fixture_by_topic(topic).ok_or_else(|| Error::Routing {
            message: format!("No fixture found for topic: {}", topic),
        })?;

        // Create templating context with environment variables
        let mut env_vars = std::collections::HashMap::new();
        env_vars.insert("topic".to_string(), topic.clone());

        let context = templating::TemplatingContext::with_env(env_vars);

        // Use template engine to render payload
        let template_str = serde_json::to_string(&fixture.response.payload).map_err(Error::Json)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_fixture() -> crate::fixtures::MqttFixture {
        crate::fixtures::MqttFixture {
            identifier: "test-fixture".to_string(),
            name: "Test Fixture".to_string(),
            topic_pattern: "test/topic".to_string(),
            qos: 1,
            retained: false,
            response: crate::fixtures::MqttResponse {
                payload: serde_json::json!({"message": "test response"}),
            },
            auto_publish: None,
        }
    }

    #[test]
    fn test_mqtt_spec_registry_new() {
        let registry = MqttSpecRegistry::new();
        assert_eq!(registry.operations().len(), 0);
    }

    #[test]
    fn test_mqtt_spec_registry_default() {
        let registry = MqttSpecRegistry::default();
        assert_eq!(registry.protocol(), Protocol::Mqtt);
    }

    #[test]
    fn test_add_fixture() {
        let mut registry = MqttSpecRegistry::new();
        let fixture = create_test_fixture();
        registry.add_fixture(fixture);

        assert_eq!(registry.operations().len(), 1);
    }

    #[test]
    fn test_find_fixture_by_topic() {
        let mut registry = MqttSpecRegistry::new();
        let fixture = create_test_fixture();
        registry.add_fixture(fixture);

        let found = registry.find_fixture_by_topic("test/topic");
        assert!(found.is_some());
        assert_eq!(found.unwrap().identifier, "test-fixture");
    }

    #[test]
    fn test_find_fixture_by_topic_not_found() {
        let registry = MqttSpecRegistry::new();
        let found = registry.find_fixture_by_topic("nonexistent/topic");
        assert!(found.is_none());
    }

    #[test]
    fn test_protocol() {
        let registry = MqttSpecRegistry::new();
        assert_eq!(registry.protocol(), Protocol::Mqtt);
    }

    #[test]
    fn test_operations_empty() {
        let registry = MqttSpecRegistry::new();
        let ops = registry.operations();
        assert_eq!(ops.len(), 0);
    }

    #[test]
    fn test_operations_with_fixtures() {
        let mut registry = MqttSpecRegistry::new();
        registry.add_fixture(create_test_fixture());

        let ops = registry.operations();
        assert_eq!(ops.len(), 1);
        assert_eq!(ops[0].name, "test-fixture");
        assert_eq!(ops[0].path, "test/topic");
        assert_eq!(ops[0].operation_type, "PUBLISH");
    }

    #[test]
    fn test_find_operation() {
        let mut registry = MqttSpecRegistry::new();
        registry.add_fixture(create_test_fixture());

        let op = registry.find_operation("PUBLISH", "test/topic");
        assert!(op.is_some());
        assert_eq!(op.unwrap().name, "test-fixture");
    }

    #[test]
    fn test_find_operation_not_found() {
        let registry = MqttSpecRegistry::new();
        let op = registry.find_operation("PUBLISH", "nonexistent");
        assert!(op.is_none());
    }

    #[test]
    fn test_validate_request_missing_topic() {
        let registry = MqttSpecRegistry::new();
        let request = ProtocolRequest {
            topic: None,
            qos: Some(1),
            ..Default::default()
        };

        let result = registry.validate_request(&request).unwrap();
        assert!(!result.valid);
        assert_eq!(result.errors.len(), 1);
        assert_eq!(result.errors[0].code, Some("MISSING_TOPIC".to_string()));
    }

    #[test]
    fn test_validate_request_no_fixture() {
        let registry = MqttSpecRegistry::new();
        let request = ProtocolRequest {
            topic: Some("test/topic".to_string()),
            qos: Some(1),
            ..Default::default()
        };

        let result = registry.validate_request(&request).unwrap();
        assert!(!result.valid);
        assert_eq!(result.errors[0].code, Some("NO_FIXTURE".to_string()));
    }

    #[test]
    fn test_validate_request_success() {
        let mut registry = MqttSpecRegistry::new();
        registry.add_fixture(create_test_fixture());

        let request = ProtocolRequest {
            topic: Some("test/topic".to_string()),
            qos: Some(1),
            ..Default::default()
        };

        let result = registry.validate_request(&request).unwrap();
        assert!(result.valid);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_generate_mock_response_missing_topic() {
        let registry = MqttSpecRegistry::new();
        let request = ProtocolRequest {
            topic: None,
            ..Default::default()
        };

        let result = registry.generate_mock_response(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_mock_response_no_fixture() {
        let registry = MqttSpecRegistry::new();
        let request = ProtocolRequest {
            topic: Some("nonexistent".to_string()),
            ..Default::default()
        };

        let result = registry.generate_mock_response(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_mock_response_success() {
        let mut registry = MqttSpecRegistry::new();
        registry.add_fixture(create_test_fixture());

        let request = ProtocolRequest {
            topic: Some("test/topic".to_string()),
            qos: Some(1),
            ..Default::default()
        };

        let result = registry.generate_mock_response(&request);
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(matches!(response.status, ResponseStatus::MqttStatus(true)));
        assert_eq!(response.metadata.get("topic").unwrap(), "test/topic");
    }

    #[test]
    fn test_load_fixtures_nonexistent_directory() {
        let mut registry = MqttSpecRegistry::new();
        let result = registry.load_fixtures("/nonexistent/directory");
        // Should not error on missing directory
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_fixtures_yaml() {
        let mut registry = MqttSpecRegistry::new();
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = temp_dir.path().join("fixture.yaml");

        let yaml_content = r#"
identifier: yaml-fixture
name: YAML Fixture
topic_pattern: sensor/temp
qos: 1
retained: false
response:
  payload:
    temperature: 25.5
"#;
        std::fs::write(&fixture_path, yaml_content).unwrap();

        let result = registry.load_fixtures(temp_dir.path());
        assert!(result.is_ok());

        let fixture = registry.find_fixture_by_topic("sensor/temp");
        assert!(fixture.is_some());
        assert_eq!(fixture.unwrap().identifier, "yaml-fixture");
    }

    #[test]
    fn test_load_fixtures_json() {
        let mut registry = MqttSpecRegistry::new();
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = temp_dir.path().join("fixture.json");

        let json_content = r#"{
  "identifier": "json-fixture",
  "name": "JSON Fixture",
  "topic_pattern": "sensor/humidity",
  "qos": 1,
  "retained": false,
  "response": {
    "payload": {
      "humidity": 60
    }
  }
}"#;
        std::fs::write(&fixture_path, json_content).unwrap();

        let result = registry.load_fixtures(temp_dir.path());
        assert!(result.is_ok());

        let fixture = registry.find_fixture_by_topic("sensor/humidity");
        assert!(fixture.is_some());
        assert_eq!(fixture.unwrap().identifier, "json-fixture");
    }

    #[test]
    fn test_load_fixtures_multiple_files() {
        let mut registry = MqttSpecRegistry::new();
        let temp_dir = TempDir::new().unwrap();

        // Create YAML fixture
        let yaml_path = temp_dir.path().join("fixture1.yaml");
        std::fs::write(
            &yaml_path,
            r#"
identifier: fixture1
name: Fixture 1
topic_pattern: topic1
qos: 0
retained: false
response:
  payload: {}
"#,
        )
        .unwrap();

        // Create JSON fixture
        let json_path = temp_dir.path().join("fixture2.json");
        std::fs::write(
            &json_path,
            r#"{
  "identifier": "fixture2",
  "name": "Fixture 2",
  "topic_pattern": "topic2",
  "qos": 1,
  "retained": false,
  "response": {
    "payload": {}
  }
}"#,
        )
        .unwrap();

        let result = registry.load_fixtures(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(registry.operations().len(), 2);
    }

    #[test]
    fn test_load_fixtures_skips_non_fixture_files() {
        let mut registry = MqttSpecRegistry::new();
        let temp_dir = TempDir::new().unwrap();

        // Create a non-fixture file
        let txt_path = temp_dir.path().join("readme.txt");
        std::fs::write(&txt_path, "Not a fixture").unwrap();

        let result = registry.load_fixtures(temp_dir.path());
        assert!(result.is_ok());
        assert_eq!(registry.operations().len(), 0);
    }

    #[test]
    fn test_load_fixture_file_invalid_yaml() {
        let mut registry = MqttSpecRegistry::new();
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = temp_dir.path().join("invalid.yaml");

        std::fs::write(&fixture_path, "invalid: yaml: content:").unwrap();

        let result = registry.load_fixtures(temp_dir.path());
        // Should not panic, but won't load the invalid file
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_fixture_file_invalid_json() {
        let mut registry = MqttSpecRegistry::new();
        let temp_dir = TempDir::new().unwrap();
        let fixture_path = temp_dir.path().join("invalid.json");

        std::fs::write(&fixture_path, "{invalid json}").unwrap();

        let result = registry.load_fixtures(temp_dir.path());
        // Should not panic, but won't load the invalid file
        assert!(result.is_ok());
    }

    #[test]
    fn test_multiple_fixtures_same_pattern() {
        let mut registry = MqttSpecRegistry::new();

        let fixture1 = crate::fixtures::MqttFixture {
            identifier: "fixture1".to_string(),
            name: "Fixture 1".to_string(),
            topic_pattern: "test/.*".to_string(),
            qos: 1,
            retained: false,
            response: crate::fixtures::MqttResponse {
                payload: serde_json::json!({"id": 1}),
            },
            auto_publish: None,
        };

        let fixture2 = crate::fixtures::MqttFixture {
            identifier: "fixture2".to_string(),
            name: "Fixture 2".to_string(),
            topic_pattern: "test/.*".to_string(),
            qos: 1,
            retained: false,
            response: crate::fixtures::MqttResponse {
                payload: serde_json::json!({"id": 2}),
            },
            auto_publish: None,
        };

        registry.add_fixture(fixture1);
        registry.add_fixture(fixture2);

        // Should find the first matching fixture
        let found = registry.find_fixture_by_topic("test/topic");
        assert!(found.is_some());
    }

    #[test]
    fn test_response_metadata() {
        let mut registry = MqttSpecRegistry::new();
        registry.add_fixture(create_test_fixture());

        let request = ProtocolRequest {
            topic: Some("test/topic".to_string()),
            qos: Some(2),
            ..Default::default()
        };

        let response = registry.generate_mock_response(&request).unwrap();
        assert_eq!(response.metadata.get("qos").unwrap(), "2");
        assert_eq!(response.metadata.get("retained").unwrap(), "false");
        assert_eq!(response.content_type, "application/json");
    }

    #[test]
    fn test_yml_extension() {
        let mut registry = MqttSpecRegistry::new();
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

        let result = registry.load_fixtures(temp_dir.path());
        assert!(result.is_ok());

        let fixture = registry.find_fixture_by_topic("test/yml");
        assert!(fixture.is_some());
    }
}
