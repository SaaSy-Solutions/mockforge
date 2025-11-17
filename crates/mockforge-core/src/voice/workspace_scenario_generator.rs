//! Workspace scenario generator
//!
//! Generates complete workspace configurations from parsed scenario descriptions,
//! including OpenAPI specs, chaos configs, initial data, and workspace structure.

use crate::openapi::OpenApiSpec;
use crate::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_yaml;
use std::collections::HashMap;
use uuid::Uuid;

use super::command_parser::ParsedWorkspaceScenario;
use super::spec_generator::VoiceSpecGenerator;

/// Generated workspace scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedWorkspaceScenario {
    /// Workspace ID
    pub workspace_id: String,
    /// Workspace name
    pub name: String,
    /// Workspace description
    pub description: String,
    /// Generated OpenAPI specification (serialized as JSON string since OpenApiSpec doesn't implement Serialize)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openapi_spec: Option<String>, // Serialized as JSON string
    /// Chaos configuration (YAML)
    pub chaos_config: Option<String>,
    /// Initial fixture data
    pub fixtures: HashMap<String, Vec<Value>>,
    /// Workspace configuration summary
    pub config_summary: WorkspaceConfigSummary,
}

/// Workspace configuration summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfigSummary {
    /// Number of endpoints
    pub endpoint_count: usize,
    /// Number of models
    pub model_count: usize,
    /// Number of chaos characteristics
    pub chaos_characteristic_count: usize,
    /// Initial data counts
    pub initial_data_counts: HashMap<String, usize>,
}

/// Generator for workspace scenarios
pub struct WorkspaceScenarioGenerator;

impl WorkspaceScenarioGenerator {
    /// Create a new workspace scenario generator
    pub fn new() -> Self {
        Self
    }

    /// Generate a complete workspace scenario from parsed description
    pub async fn generate_scenario(
        &self,
        parsed: &ParsedWorkspaceScenario,
    ) -> Result<GeneratedWorkspaceScenario> {
        // Generate workspace ID
        let workspace_id = Uuid::new_v4().to_string();

        // Generate OpenAPI spec from API requirements
        let openapi_spec = if !parsed.api_requirements.endpoints.is_empty() {
            // Convert API requirements to ParsedCommand format for spec generation
            let mut parsed_command = super::command_parser::ParsedCommand {
                api_type: parsed.domain.clone(),
                title: parsed.title.clone(),
                description: parsed.description.clone(),
                endpoints: parsed.api_requirements.endpoints.clone(),
                models: parsed.api_requirements.models.clone(),
                relationships: vec![],
                sample_counts: HashMap::new(),
                flows: vec![],
            };

            // Add sample counts from initial data
            if let Some(user_count) = parsed.initial_data.users {
                parsed_command.sample_counts.insert("User".to_string(), user_count);
            }
            if let Some(dispute_count) = parsed.initial_data.disputes {
                parsed_command.sample_counts.insert("Dispute".to_string(), dispute_count);
            }
            if let Some(order_count) = parsed.initial_data.orders {
                parsed_command.sample_counts.insert("Order".to_string(), order_count);
            }
            for (entity, count) in &parsed.initial_data.custom {
                parsed_command.sample_counts.insert(entity.clone(), *count);
            }

            // Generate spec
            let spec_generator = VoiceSpecGenerator::new();
            let spec_result = spec_generator.generate_spec(&parsed_command).await;
            // Convert OpenApiSpec to JSON string for serialization
            spec_result.ok().and_then(|spec| {
                // Use raw_document if available, otherwise serialize the spec
                if let Some(ref raw) = spec.raw_document {
                    serde_json::to_string(raw).ok()
                } else {
                    // Fallback: try to serialize the spec struct
                    serde_json::to_string(&spec.spec).ok()
                }
            })
        } else {
            None
        };

        // Generate chaos configuration
        let chaos_config = if !parsed.chaos_characteristics.is_empty() {
            Some(self.generate_chaos_config(&parsed.chaos_characteristics)?)
        } else {
            None
        };

        // Generate initial fixture data
        let fixtures = self.generate_fixtures(parsed)?;

        // Build config summary
        let mut initial_data_counts = HashMap::new();
        if let Some(count) = parsed.initial_data.users {
            initial_data_counts.insert("users".to_string(), count);
        }
        if let Some(count) = parsed.initial_data.disputes {
            initial_data_counts.insert("disputes".to_string(), count);
        }
        if let Some(count) = parsed.initial_data.orders {
            initial_data_counts.insert("orders".to_string(), count);
        }
        for (entity, count) in &parsed.initial_data.custom {
            initial_data_counts.insert(entity.clone(), *count);
        }

        let config_summary = WorkspaceConfigSummary {
            endpoint_count: parsed.api_requirements.endpoints.len(),
            model_count: parsed.api_requirements.models.len(),
            chaos_characteristic_count: parsed.chaos_characteristics.len(),
            initial_data_counts,
        };

        Ok(GeneratedWorkspaceScenario {
            workspace_id,
            name: parsed.title.clone(),
            description: parsed.description.clone(),
            openapi_spec,
            chaos_config,
            fixtures,
            config_summary,
        })
    }

    /// Generate chaos configuration YAML from characteristics
    fn generate_chaos_config(
        &self,
        characteristics: &[super::command_parser::ChaosCharacteristic],
    ) -> Result<String> {
        let mut config = serde_yaml::Mapping::new();

        // Build chaos configuration
        let mut chaos = serde_yaml::Mapping::new();
        chaos.insert(
            serde_yaml::Value::String("enabled".to_string()),
            serde_yaml::Value::Bool(true),
        );

        // Process each characteristic
        for char in characteristics {
            match char.r#type.as_str() {
                "latency" | "slow" => {
                    let mut latency = serde_yaml::Mapping::new();
                    latency.insert(
                        serde_yaml::Value::String("enabled".to_string()),
                        serde_yaml::Value::Bool(true),
                    );

                    // Extract delay from config
                    if let Some(delay) = char.config.get("delay_ms").and_then(|v| v.as_u64()) {
                        latency.insert(
                            serde_yaml::Value::String("fixed_delay_ms".to_string()),
                            serde_yaml::Value::Number(delay.into()),
                        );
                    } else {
                        // Default slow latency
                        latency.insert(
                            serde_yaml::Value::String("fixed_delay_ms".to_string()),
                            serde_yaml::Value::Number(1000.into()),
                        );
                    }

                    chaos.insert(
                        serde_yaml::Value::String("latency".to_string()),
                        serde_yaml::Value::Mapping(latency),
                    );
                }
                "failure" | "flaky" | "error" => {
                    let mut fault = serde_yaml::Mapping::new();
                    fault.insert(
                        serde_yaml::Value::String("enabled".to_string()),
                        serde_yaml::Value::Bool(true),
                    );

                    // Extract error rate and codes
                    if let Some(rate) = char.config.get("error_rate").and_then(|v| v.as_f64()) {
                        // serde_yaml::Number doesn't have from_f64, so we convert to string and parse
                        let num_str = rate.to_string();
                        fault.insert(
                            serde_yaml::Value::String("http_error_probability".to_string()),
                            serde_yaml::Value::Number(
                                num_str
                                    .parse::<serde_yaml::Number>()
                                    .unwrap_or_else(|_| serde_yaml::Number::from(0.1)),
                            ),
                        );
                    } else {
                        fault.insert(
                            serde_yaml::Value::String("http_error_probability".to_string()),
                            serde_yaml::Value::Number(0.1.into()),
                        );
                    }

                    if let Some(codes) = char.config.get("error_codes").and_then(|v| v.as_array()) {
                        let codes: Vec<serde_yaml::Value> = codes
                            .iter()
                            .filter_map(|v| v.as_u64().map(|n| serde_yaml::Value::Number(n.into())))
                            .collect();
                        fault.insert(
                            serde_yaml::Value::String("http_errors".to_string()),
                            serde_yaml::Value::Sequence(codes),
                        );
                    } else {
                        fault.insert(
                            serde_yaml::Value::String("http_errors".to_string()),
                            serde_yaml::Value::Sequence(vec![
                                serde_yaml::Value::Number(500.into()),
                                serde_yaml::Value::Number(502.into()),
                                serde_yaml::Value::Number(503.into()),
                            ]),
                        );
                    }

                    chaos.insert(
                        serde_yaml::Value::String("fault_injection".to_string()),
                        serde_yaml::Value::Mapping(fault),
                    );
                }
                _ => {
                    // Generic characteristic - add to config as-is
                    if let Ok(value) = serde_yaml::to_value(&char.config) {
                        chaos.insert(serde_yaml::Value::String(char.r#type.clone()), value);
                    }
                }
            }
        }

        config.insert(
            serde_yaml::Value::String("chaos".to_string()),
            serde_yaml::Value::Mapping(chaos),
        );

        // Convert to YAML string
        serde_yaml::to_string(&config).map_err(|e| {
            crate::Error::generic(format!("Failed to serialize chaos config to YAML: {}", e))
        })
    }

    /// Generate initial fixture data
    fn generate_fixtures(
        &self,
        parsed: &ParsedWorkspaceScenario,
    ) -> Result<HashMap<String, Vec<Value>>> {
        let mut fixtures = HashMap::new();

        // Generate user fixtures
        if let Some(user_count) = parsed.initial_data.users {
            let mut users = Vec::new();
            for i in 0..user_count {
                users.push(serde_json::json!({
                    "id": i + 1,
                    "name": format!("User {}", i + 1),
                    "email": format!("user{}@example.com", i + 1),
                    "created_at": Utc::now().to_rfc3339(),
                }));
            }
            fixtures.insert("users".to_string(), users);
        }

        // Generate dispute fixtures
        if let Some(dispute_count) = parsed.initial_data.disputes {
            let mut disputes = Vec::new();
            for i in 0..dispute_count {
                disputes.push(serde_json::json!({
                    "id": i + 1,
                    "user_id": (i % parsed.initial_data.users.unwrap_or(1)) + 1,
                    "status": "open",
                    "description": format!("Dispute {}", i + 1),
                    "created_at": Utc::now().to_rfc3339(),
                }));
            }
            fixtures.insert("disputes".to_string(), disputes);
        }

        // Generate order fixtures
        if let Some(order_count) = parsed.initial_data.orders {
            let mut orders = Vec::new();
            for i in 0..order_count {
                orders.push(serde_json::json!({
                    "id": i + 1,
                    "user_id": (i % parsed.initial_data.users.unwrap_or(1)) + 1,
                    "status": "pending",
                    "total": 100.0 + (i as f64 * 10.0),
                    "created_at": Utc::now().to_rfc3339(),
                }));
            }
            fixtures.insert("orders".to_string(), orders);
        }

        // Generate custom entity fixtures
        for (entity_name, count) in &parsed.initial_data.custom {
            let mut entities = Vec::new();
            for i in 0..*count {
                entities.push(serde_json::json!({
                    "id": i + 1,
                    "name": format!("{} {}", entity_name, i + 1),
                    "created_at": Utc::now().to_rfc3339(),
                }));
            }
            fixtures.insert(entity_name.clone(), entities);
        }

        Ok(fixtures)
    }
}

impl Default for WorkspaceScenarioGenerator {
    fn default() -> Self {
        Self::new()
    }
}
