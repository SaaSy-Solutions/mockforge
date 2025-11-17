//! LLM-based command parser for voice commands
//!
//! This module parses natural language voice commands and extracts API requirements
//! using MockForge's LLM infrastructure.

use crate::intelligent_behavior::{
    config::IntelligentBehaviorConfig, llm_client::LlmClient, types::LlmGenerationRequest,
};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Voice command parser that uses LLM to interpret natural language commands
pub struct VoiceCommandParser {
    /// LLM client for parsing commands
    llm_client: LlmClient,
    /// Configuration
    config: IntelligentBehaviorConfig,
}

impl VoiceCommandParser {
    /// Create a new voice command parser
    pub fn new(config: IntelligentBehaviorConfig) -> Self {
        let behavior_model = config.behavior_model.clone();
        let llm_client = LlmClient::new(behavior_model);

        Self { llm_client, config }
    }

    /// Parse a natural language command into structured API requirements
    ///
    /// This method uses the LLM to extract:
    /// - API type (e-commerce, social media, etc.)
    /// - Endpoints and HTTP methods
    /// - Data models and relationships
    /// - Sample data counts
    /// - Business flows (checkout, auth, etc.)
    pub async fn parse_command(&self, command: &str) -> Result<ParsedCommand> {
        // Build system prompt for command parsing
        let system_prompt = r#"You are an expert API designer. Your task is to parse natural language commands
that describe API requirements and extract structured information.

Extract the following information from the command:
1. API type/category (e.g., e-commerce, social media, blog, todo app)
2. Endpoints with HTTP methods (GET, POST, PUT, DELETE, PATCH)
3. Data models with fields and types
4. Relationships between models
5. Sample data counts (e.g., "20 products")
6. Business flows (e.g., checkout, authentication, user registration)

Return your response as a JSON object with this structure:
{
  "api_type": "string (e.g., e-commerce, social-media, blog)",
  "title": "string (API title)",
  "description": "string (API description)",
  "endpoints": [
    {
      "path": "string (e.g., /api/products)",
      "method": "string (GET, POST, PUT, DELETE, PATCH)",
      "description": "string",
      "request_body": {
        "schema": "object schema if applicable",
        "required": ["array of required fields"]
      },
      "response": {
        "status": 200,
        "schema": "object schema",
        "is_array": false,
        "count": null or number if specified
      }
    }
  ],
  "models": [
    {
      "name": "string (e.g., Product)",
      "fields": [
        {
          "name": "string",
          "type": "string (string, number, integer, boolean, array, object)",
          "description": "string",
          "required": true
        }
      ]
    }
  ],
  "relationships": [
    {
      "from": "string (model name)",
      "to": "string (model name)",
      "type": "string (one-to-many, many-to-many, one-to-one)"
    }
  ],
  "sample_counts": {
    "model_name": number
  },
  "flows": [
    {
      "name": "string (e.g., checkout)",
      "description": "string",
      "steps": ["array of step descriptions"]
    }
  ]
}

Be specific and extract all details mentioned in the command. If something is not mentioned,
don't include it in the response."#;

        // Build user prompt with the command
        let user_prompt =
            format!("Parse this API creation command and extract all requirements:\n\n{}", command);

        // Create LLM request
        let llm_request = LlmGenerationRequest {
            system_prompt: system_prompt.to_string(),
            user_prompt,
            temperature: 0.3, // Lower temperature for more consistent parsing
            max_tokens: 2000,
            schema: None,
        };

        // Generate response from LLM
        let response = self.llm_client.generate(&llm_request).await?;

        // Parse the response into ParsedCommand
        let response_str = serde_json::to_string(&response).unwrap_or_default();
        let parsed: ParsedCommand = serde_json::from_value(response).map_err(|e| {
            crate::Error::generic(format!(
                "Failed to parse LLM response as ParsedCommand: {}. Response: {}",
                e, response_str
            ))
        })?;

        Ok(parsed)
    }

    /// Parse a conversational command (for multi-turn interactions)
    ///
    /// This method parses commands that modify or extend an existing API specification.
    /// It takes the current conversation context into account.
    pub async fn parse_conversational_command(
        &self,
        command: &str,
        context: &super::conversation::ConversationContext,
    ) -> Result<ParsedCommand> {
        // Build system prompt for conversational parsing
        let system_prompt = r#"You are an expert API designer helping to build an API through conversation.
The user is providing incremental commands to modify or extend an existing API specification.

Extract the following information from the command:
1. What is being added/modified (endpoints, models, flows)
2. Details about the addition/modification
3. Any relationships or dependencies

Return your response as a JSON object with the same structure as parse_command, but focus only
on what is NEW or MODIFIED. If the command is asking to add something, include it. If it's asking
to modify something, include the modified version.

If the command is asking a question or requesting confirmation, return an empty endpoints array
and include a "question" or "confirmation" field in the response."#;

        // Build context summary
        let context_summary = format!(
            "Current API: {}\nExisting endpoints: {}\nExisting models: {}",
            context.current_spec.as_ref().map(|s| s.title()).unwrap_or("None"),
            context
                .current_spec
                .as_ref()
                .map(|s| {
                    s.all_paths_and_operations()
                        .iter()
                        .map(|(path, ops)| {
                            format!(
                                "{} ({})",
                                path,
                                ops.keys().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "None".to_string()),
            context
                .current_spec
                .as_ref()
                .and_then(|s| s.spec.components.as_ref())
                .map(|c| c.schemas.keys().cloned().collect::<Vec<_>>().join(", "))
                .unwrap_or_else(|| "None".to_string())
        );

        // Build user prompt
        let user_prompt = format!("Context:\n{}\n\nNew command:\n{}", context_summary, command);

        // Create LLM request
        let llm_request = LlmGenerationRequest {
            system_prompt: system_prompt.to_string(),
            user_prompt,
            temperature: 0.3,
            max_tokens: 2000,
            schema: None,
        };

        // Generate response
        let response = self.llm_client.generate(&llm_request).await?;

        // Parse response
        let response_str = serde_json::to_string(&response).unwrap_or_default();
        let parsed: ParsedCommand = serde_json::from_value(response).map_err(|e| {
            crate::Error::generic(format!(
                "Failed to parse conversational LLM response: {}. Response: {}",
                e, response_str
            ))
        })?;

        Ok(parsed)
    }

    /// Parse a workspace scenario description
    ///
    /// This method extracts information about creating a complete workspace scenario,
    /// including domain, chaos characteristics, initial data, and API requirements.
    pub async fn parse_workspace_scenario_command(&self, command: &str) -> Result<ParsedWorkspaceScenario> {
        // Build system prompt for workspace scenario parsing
        let system_prompt = r#"You are an expert at parsing natural language descriptions of workspace scenarios
and extracting structured information for creating complete mock environments.

Extract the following information from the command:
1. Domain/industry (e.g., bank, e-commerce, healthcare, etc.)
2. Chaos/failure characteristics (flaky rates, slow KYC, high latency, etc.)
3. Initial data requirements (number of users, disputes, orders, etc.)
4. API endpoints needed for the domain
5. Behavioral rules (failure rates, latency patterns, etc.)
6. Data models and relationships

Return your response as a JSON object with this structure:
{
  "domain": "string (e.g., bank, e-commerce, healthcare)",
  "title": "string (workspace title)",
  "description": "string (workspace description)",
  "chaos_characteristics": [
    {
      "type": "string (latency|failure|rate_limit|etc.)",
      "description": "string (e.g., flaky foreign exchange rates)",
      "config": {
        "probability": 0.0-1.0,
        "delay_ms": number,
        "error_rate": 0.0-1.0,
        "error_codes": [500, 502, 503],
        "details": "additional configuration details"
      }
    }
  ],
  "initial_data": {
    "users": number,
    "disputes": number,
    "orders": number,
    "custom": {
      "entity_name": number
    }
  },
  "api_requirements": {
    "endpoints": [
      {
        "path": "string",
        "method": "string",
        "description": "string"
      }
    ],
    "models": [
      {
        "name": "string",
        "fields": [
          {
            "name": "string",
            "type": "string"
          }
        ]
      }
    ]
  },
  "behavioral_rules": [
    {
      "description": "string",
      "type": "string",
      "config": {}
    }
  ]
}

Be specific and extract all details mentioned in the command."#;

        // Build user prompt with the command
        let user_prompt = format!(
            "Parse this workspace scenario description and extract all requirements:\n\n{}",
            command
        );

        // Create LLM request
        let llm_request = LlmGenerationRequest {
            system_prompt: system_prompt.to_string(),
            user_prompt,
            temperature: 0.3,
            max_tokens: 3000,
            schema: None,
        };

        // Generate response from LLM
        let response = self.llm_client.generate(&llm_request).await?;

        // Parse the response into ParsedWorkspaceScenario
        let response_str = serde_json::to_string(&response).unwrap_or_default();
        let parsed: ParsedWorkspaceScenario = serde_json::from_value(response).map_err(|e| {
            crate::Error::generic(format!(
                "Failed to parse LLM response as ParsedWorkspaceScenario: {}. Response: {}",
                e, response_str
            ))
        })?;

        Ok(parsed)
    }
}

/// Parsed command structure containing extracted API requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedCommand {
    /// API type/category
    pub api_type: String,
    /// API title
    pub title: String,
    /// API description
    pub description: String,
    /// List of endpoints
    pub endpoints: Vec<EndpointRequirement>,
    /// List of data models
    pub models: Vec<ModelRequirement>,
    /// Relationships between models
    #[serde(default)]
    pub relationships: Vec<RelationshipRequirement>,
    /// Sample data counts per model
    #[serde(default)]
    pub sample_counts: HashMap<String, usize>,
    /// Business flows
    #[serde(default)]
    pub flows: Vec<FlowRequirement>,
}

/// Endpoint requirement extracted from command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointRequirement {
    /// Path (e.g., /api/products)
    pub path: String,
    /// HTTP method
    pub method: String,
    /// Description
    pub description: String,
    /// Request body schema (if applicable)
    #[serde(default)]
    pub request_body: Option<RequestBodyRequirement>,
    /// Response schema
    #[serde(default)]
    pub response: Option<ResponseRequirement>,
}

/// Request body requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBodyRequirement {
    /// Schema definition
    #[serde(default)]
    pub schema: Option<serde_json::Value>,
    /// Required fields
    #[serde(default)]
    pub required: Vec<String>,
}

/// Response requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseRequirement {
    /// HTTP status code
    #[serde(default = "default_status")]
    pub status: u16,
    /// Response schema
    #[serde(default)]
    pub schema: Option<serde_json::Value>,
    /// Whether response is an array
    #[serde(default)]
    pub is_array: bool,
    /// Count of items (if specified)
    #[serde(default)]
    pub count: Option<usize>,
}

fn default_status() -> u16 {
    200
}

/// Model requirement extracted from command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRequirement {
    /// Model name
    pub name: String,
    /// List of fields
    pub fields: Vec<FieldRequirement>,
}

/// Field requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldRequirement {
    /// Field name
    pub name: String,
    /// Field type
    pub r#type: String,
    /// Field description
    #[serde(default)]
    pub description: String,
    /// Whether field is required
    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_true() -> bool {
    true
}

/// Relationship requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipRequirement {
    /// Source model
    pub from: String,
    /// Target model
    pub to: String,
    /// Relationship type
    pub r#type: String,
}

/// Flow requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowRequirement {
    /// Flow name
    pub name: String,
    /// Flow description
    pub description: String,
    /// Steps in the flow
    #[serde(default)]
    pub steps: Vec<String>,
}

/// Alias for API requirement (for backwards compatibility)
pub type ApiRequirement = ParsedCommand;

/// Parsed workspace scenario structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedWorkspaceScenario {
    /// Domain/industry
    pub domain: String,
    /// Workspace title
    pub title: String,
    /// Workspace description
    pub description: String,
    /// Chaos characteristics
    #[serde(default)]
    pub chaos_characteristics: Vec<ChaosCharacteristic>,
    /// Initial data requirements
    #[serde(default)]
    pub initial_data: InitialDataRequirements,
    /// API requirements
    #[serde(default)]
    pub api_requirements: ApiRequirements,
    /// Behavioral rules
    #[serde(default)]
    pub behavioral_rules: Vec<BehavioralRule>,
}

/// Chaos characteristic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosCharacteristic {
    /// Type of chaos (latency, failure, rate_limit, etc.)
    pub r#type: String,
    /// Description
    pub description: String,
    /// Configuration details
    #[serde(default)]
    pub config: serde_json::Value,
}

/// Initial data requirements
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InitialDataRequirements {
    /// Number of users
    #[serde(default)]
    pub users: Option<usize>,
    /// Number of disputes
    #[serde(default)]
    pub disputes: Option<usize>,
    /// Number of orders
    #[serde(default)]
    pub orders: Option<usize>,
    /// Custom entity counts
    #[serde(default)]
    pub custom: HashMap<String, usize>,
}

/// API requirements for the scenario
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiRequirements {
    /// List of endpoints
    #[serde(default)]
    pub endpoints: Vec<EndpointRequirement>,
    /// List of models
    #[serde(default)]
    pub models: Vec<ModelRequirement>,
}

/// Behavioral rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralRule {
    /// Rule description
    pub description: String,
    /// Rule type
    pub r#type: String,
    /// Rule configuration
    #[serde(default)]
    pub config: serde_json::Value,
}
