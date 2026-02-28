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
    #[allow(dead_code)]
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
    pub async fn parse_workspace_scenario_command(
        &self,
        command: &str,
    ) -> Result<ParsedWorkspaceScenario> {
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

    /// Parse a workspace creation command
    ///
    /// This method extracts information about creating a complete workspace including:
    /// - Workspace name and description
    /// - Entities (customers, orders, payments, etc.)
    /// - Personas with relationships
    /// - Behavioral scenarios (happy path, failure, slow path)
    /// - Reality continuum preferences
    /// - Drift budget preferences
    pub async fn parse_workspace_creation_command(
        &self,
        command: &str,
    ) -> Result<ParsedWorkspaceCreation> {
        // Build system prompt for workspace creation parsing
        let system_prompt = r#"You are an expert at parsing natural language descriptions of workspace creation
and extracting structured information for creating complete mock backends with personas, scenarios, and configuration.

Extract the following information from the command:
1. Workspace name and description
2. Entities (customers, orders, payments, products, etc.)
3. Personas with their traits and relationships (e.g., customer owns orders)
4. Behavioral scenarios:
   - Happy path scenarios (successful flows)
   - Failure path scenarios (error cases)
   - Slow path scenarios (latency/performance issues)
5. Reality continuum preferences (e.g., "80% mock, 20% real prod for catalog only")
6. Drift budget preferences (e.g., "strict drift budget", "moderate tolerance")

Return your response as a JSON object with this structure:
{
  "workspace_name": "string (e.g., e-commerce-workspace)",
  "workspace_description": "string",
  "entities": [
    {
      "name": "string (e.g., Customer, Order, Payment)",
      "description": "string",
      "endpoints": [
        {
          "path": "string",
          "method": "string",
          "description": "string"
        }
      ],
      "fields": [
        {
          "name": "string",
          "type": "string",
          "description": "string"
        }
      ]
    }
  ],
  "personas": [
    {
      "name": "string (e.g., premium-customer, regular-customer)",
      "description": "string",
      "traits": {
        "trait_name": "trait_value"
      },
      "relationships": [
        {
          "type": "string (e.g., owns, belongs_to, has)",
          "target_entity": "string (e.g., Order, Payment)"
        }
      ]
    }
  ],
  "scenarios": [
    {
      "name": "string (e.g., happy-path-checkout, failed-payment, slow-shipping)",
      "type": "string (happy_path|failure|slow_path)",
      "description": "string",
      "steps": [
        {
          "description": "string (e.g., Create order, Process payment)",
          "endpoint": "string (e.g., POST /api/orders)",
          "expected_outcome": "string"
        }
      ]
    }
  ],
  "reality_continuum": {
    "default_ratio": 0.0-1.0 (0.0 = 100% mock, 1.0 = 100% real),
    "route_rules": [
      {
        "pattern": "string (e.g., /api/catalog/*)",
        "ratio": 0.0-1.0,
        "description": "string"
      }
    ],
    "transition_mode": "string (manual|time_based|scheduled)"
  },
  "drift_budget": {
    "strictness": "string (strict|moderate|lenient)",
    "max_breaking_changes": number,
    "max_non_breaking_changes": number,
    "description": "string"
  }
}

Be specific and extract all details mentioned in the command. Ensure at least 2-3 endpoints per entity,
2-3 personas with relationships, and 2-3 behavioral scenarios."#;

        // Build user prompt with the command
        let user_prompt = format!(
            "Parse this workspace creation command and extract all requirements:\n\n{}",
            command
        );

        // Create LLM request
        let llm_request = LlmGenerationRequest {
            system_prompt: system_prompt.to_string(),
            user_prompt,
            temperature: 0.3,
            max_tokens: 4000,
            schema: None,
        };

        // Generate response from LLM
        let response = self.llm_client.generate(&llm_request).await?;

        // Parse the response into ParsedWorkspaceCreation
        let response_str = serde_json::to_string(&response).unwrap_or_default();
        let parsed: ParsedWorkspaceCreation = serde_json::from_value(response).map_err(|e| {
            crate::Error::generic(format!(
                "Failed to parse LLM response as ParsedWorkspaceCreation: {}. Response: {}",
                e, response_str
            ))
        })?;

        Ok(parsed)
    }

    /// Parse a reality continuum configuration command
    ///
    /// This method extracts reality continuum preferences from natural language,
    /// such as "80% mock, 20% real prod for catalog only".
    pub async fn parse_reality_continuum_command(
        &self,
        command: &str,
    ) -> Result<ParsedRealityContinuum> {
        // Build system prompt for reality continuum parsing
        let system_prompt = r#"You are an expert at parsing natural language descriptions of reality continuum
configuration and extracting structured blend ratio settings.

Extract the following information from the command:
1. Default blend ratio (e.g., "80% mock, 20% real" means ratio 0.2)
2. Route-specific rules (e.g., "catalog only", "for /api/products/*")
3. Transition mode preferences (manual, time-based, scheduled)

Return your response as a JSON object with this structure:
{
  "default_ratio": 0.0-1.0 (0.0 = 100% mock, 1.0 = 100% real),
  "enabled": true/false,
  "route_rules": [
    {
      "pattern": "string (e.g., /api/catalog/*, /api/products/*)",
      "ratio": 0.0-1.0,
      "description": "string"
    }
  ],
  "transition_mode": "string (manual|time_based|scheduled)",
  "merge_strategy": "string (field_level|weighted|body_blend)"
}

Examples:
- "80% mock, 20% real" → default_ratio: 0.2
- "Make catalog 50% real" → route_rules: [{pattern: "/api/catalog/*", ratio: 0.5}]
- "100% mock for now" → default_ratio: 0.0, enabled: true"#;

        // Build user prompt with the command
        let user_prompt =
            format!("Parse this reality continuum configuration command:\n\n{}", command);

        // Create LLM request
        let llm_request = LlmGenerationRequest {
            system_prompt: system_prompt.to_string(),
            user_prompt,
            temperature: 0.3,
            max_tokens: 2000,
            schema: None,
        };

        // Generate response from LLM
        let response = self.llm_client.generate(&llm_request).await?;

        // Parse the response into ParsedRealityContinuum
        let response_str = serde_json::to_string(&response).unwrap_or_default();
        let parsed: ParsedRealityContinuum = serde_json::from_value(response).map_err(|e| {
            crate::Error::generic(format!(
                "Failed to parse LLM response as ParsedRealityContinuum: {}. Response: {}",
                e, response_str
            ))
        })?;

        Ok(parsed)
    }

    /// Parse a drift budget configuration command
    ///
    /// This method extracts drift budget preferences from natural language,
    /// such as "strict drift budget" or "moderate tolerance for changes".
    pub async fn parse_drift_budget_command(&self, command: &str) -> Result<ParsedDriftBudget> {
        // Build system prompt for drift budget parsing
        let system_prompt = r#"You are an expert at parsing natural language descriptions of drift budget
configuration and extracting structured budget settings.

Extract the following information from the command:
1. Strictness level (strict, moderate, lenient)
2. Breaking change tolerance
3. Non-breaking change tolerance
4. Per-service/endpoint preferences

Return your response as a JSON object with this structure:
{
  "strictness": "string (strict|moderate|lenient)",
  "enabled": true/false,
  "max_breaking_changes": number (0 for strict, higher for lenient),
  "max_non_breaking_changes": number,
  "max_field_churn_percent": number (0.0-100.0, optional),
  "time_window_days": number (optional, for percentage-based budgets),
  "per_service_budgets": {
    "service_name": {
      "max_breaking_changes": number,
      "max_non_breaking_changes": number
    }
  },
  "description": "string"
}

Examples:
- "strict drift budget" → strictness: "strict", max_breaking_changes: 0, max_non_breaking_changes: 5
- "moderate tolerance" → strictness: "moderate", max_breaking_changes: 1, max_non_breaking_changes: 10
- "lenient, allow up to 5 breaking changes" → strictness: "lenient", max_breaking_changes: 5"#;

        // Build user prompt with the command
        let user_prompt = format!("Parse this drift budget configuration command:\n\n{}", command);

        // Create LLM request
        let llm_request = LlmGenerationRequest {
            system_prompt: system_prompt.to_string(),
            user_prompt,
            temperature: 0.3,
            max_tokens: 2000,
            schema: None,
        };

        // Generate response from LLM
        let response = self.llm_client.generate(&llm_request).await?;

        // Parse the response into ParsedDriftBudget
        let response_str = serde_json::to_string(&response).unwrap_or_default();
        let parsed: ParsedDriftBudget = serde_json::from_value(response).map_err(|e| {
            crate::Error::generic(format!(
                "Failed to parse LLM response as ParsedDriftBudget: {}. Response: {}",
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

/// Parsed workspace creation structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedWorkspaceCreation {
    /// Workspace name
    pub workspace_name: String,
    /// Workspace description
    pub workspace_description: String,
    /// List of entities
    #[serde(default)]
    pub entities: Vec<EntityRequirement>,
    /// List of personas
    #[serde(default)]
    pub personas: Vec<PersonaRequirement>,
    /// List of behavioral scenarios
    #[serde(default)]
    pub scenarios: Vec<ScenarioRequirement>,
    /// Reality continuum preferences
    #[serde(default)]
    pub reality_continuum: Option<ParsedRealityContinuum>,
    /// Drift budget preferences
    #[serde(default)]
    pub drift_budget: Option<ParsedDriftBudget>,
}

/// Entity requirement for workspace creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRequirement {
    /// Entity name (e.g., Customer, Order, Payment)
    pub name: String,
    /// Entity description
    pub description: String,
    /// Endpoints for this entity
    #[serde(default)]
    pub endpoints: Vec<EntityEndpointRequirement>,
    /// Fields for this entity
    #[serde(default)]
    pub fields: Vec<FieldRequirement>,
}

/// Endpoint requirement for an entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityEndpointRequirement {
    /// Path (e.g., /api/customers)
    pub path: String,
    /// HTTP method
    pub method: String,
    /// Description
    pub description: String,
}

/// Persona requirement for workspace creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaRequirement {
    /// Persona name (e.g., premium-customer, regular-customer)
    pub name: String,
    /// Persona description
    pub description: String,
    /// Persona traits
    #[serde(default)]
    pub traits: HashMap<String, String>,
    /// Relationships to other entities
    #[serde(default)]
    pub relationships: Vec<PersonaRelationship>,
}

/// Persona relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaRelationship {
    /// Relationship type (e.g., owns, belongs_to, has)
    pub r#type: String,
    /// Target entity name
    pub target_entity: String,
}

/// Scenario requirement for workspace creation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioRequirement {
    /// Scenario name (e.g., happy-path-checkout, failed-payment)
    pub name: String,
    /// Scenario type (happy_path, failure, slow_path)
    pub r#type: String,
    /// Scenario description
    pub description: String,
    /// Steps in the scenario
    #[serde(default)]
    pub steps: Vec<ScenarioStepRequirement>,
}

/// Scenario step requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioStepRequirement {
    /// Step description
    pub description: String,
    /// Endpoint for this step (e.g., POST /api/orders)
    pub endpoint: String,
    /// Expected outcome
    pub expected_outcome: String,
}

/// Parsed reality continuum configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedRealityContinuum {
    /// Default blend ratio (0.0 = 100% mock, 1.0 = 100% real)
    #[serde(default = "default_blend_ratio")]
    pub default_ratio: f64,
    /// Whether reality continuum is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Route-specific rules
    #[serde(default)]
    pub route_rules: Vec<ParsedContinuumRule>,
    /// Transition mode
    #[serde(default)]
    pub transition_mode: String,
    /// Merge strategy
    #[serde(default)]
    pub merge_strategy: String,
}

fn default_blend_ratio() -> f64 {
    0.0
}

/// Parsed continuum rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedContinuumRule {
    /// Path pattern (e.g., /api/catalog/*)
    pub pattern: String,
    /// Blend ratio for this route
    pub ratio: f64,
    /// Description
    #[serde(default)]
    pub description: String,
}

/// Parsed drift budget configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDriftBudget {
    /// Strictness level (strict, moderate, lenient)
    pub strictness: String,
    /// Whether drift budget is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Maximum breaking changes allowed
    #[serde(default)]
    pub max_breaking_changes: u32,
    /// Maximum non-breaking changes allowed
    #[serde(default)]
    pub max_non_breaking_changes: u32,
    /// Maximum field churn percentage (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_field_churn_percent: Option<f64>,
    /// Time window in days (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_window_days: Option<u32>,
    /// Per-service budgets
    #[serde(default)]
    pub per_service_budgets: HashMap<String, ParsedServiceBudget>,
    /// Description
    #[serde(default)]
    pub description: String,
}

/// Parsed service budget
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedServiceBudget {
    /// Maximum breaking changes for this service
    #[serde(default)]
    pub max_breaking_changes: u32,
    /// Maximum non-breaking changes for this service
    #[serde(default)]
    pub max_non_breaking_changes: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligent_behavior::config::IntelligentBehaviorConfig;
    use serde_json::json;

    #[test]
    fn test_voice_command_parser_new() {
        let config = IntelligentBehaviorConfig::default();
        let _parser = VoiceCommandParser::new(config);
        // Just verify it doesn't panic and creates the parser
        // The llm_client is private, so we can't directly test it
    }

    #[test]
    fn test_parsed_command_creation() {
        let command = ParsedCommand {
            api_type: "e-commerce".to_string(),
            title: "Shop API".to_string(),
            description: "An e-commerce API".to_string(),
            endpoints: vec![],
            models: vec![],
            relationships: vec![],
            sample_counts: HashMap::new(),
            flows: vec![],
        };

        assert_eq!(command.api_type, "e-commerce");
        assert_eq!(command.title, "Shop API");
        assert_eq!(command.description, "An e-commerce API");
    }

    #[test]
    fn test_endpoint_requirement_creation() {
        let endpoint = EndpointRequirement {
            path: "/api/products".to_string(),
            method: "GET".to_string(),
            description: "Get all products".to_string(),
            request_body: None,
            response: None,
        };

        assert_eq!(endpoint.path, "/api/products");
        assert_eq!(endpoint.method, "GET");
        assert_eq!(endpoint.description, "Get all products");
    }

    #[test]
    fn test_endpoint_requirement_with_body() {
        let request_body = RequestBodyRequirement {
            schema: Some(json!({"type": "object"})),
            required: vec!["name".to_string(), "price".to_string()],
        };

        let response = ResponseRequirement {
            status: 201,
            schema: Some(json!({"type": "object"})),
            is_array: false,
            count: None,
        };

        let endpoint = EndpointRequirement {
            path: "/api/products".to_string(),
            method: "POST".to_string(),
            description: "Create a product".to_string(),
            request_body: Some(request_body),
            response: Some(response),
        };

        assert!(endpoint.request_body.is_some());
        assert!(endpoint.response.is_some());
        assert_eq!(endpoint.response.unwrap().status, 201);
    }

    #[test]
    fn test_request_body_requirement_creation() {
        let body = RequestBodyRequirement {
            schema: Some(json!({"type": "object", "properties": {"name": {"type": "string"}}})),
            required: vec!["name".to_string()],
        };

        assert!(body.schema.is_some());
        assert_eq!(body.required.len(), 1);
    }

    #[test]
    fn test_response_requirement_creation() {
        let response = ResponseRequirement {
            status: 200,
            schema: Some(json!({"type": "array", "items": {"type": "object"}})),
            is_array: true,
            count: Some(10),
        };

        assert_eq!(response.status, 200);
        assert!(response.is_array);
        assert_eq!(response.count, Some(10));
    }

    #[test]
    fn test_response_requirement_default_status() {
        let response = ResponseRequirement {
            status: default_status(),
            schema: None,
            is_array: false,
            count: None,
        };

        assert_eq!(response.status, 200);
    }

    #[test]
    fn test_model_requirement_creation() {
        let field = FieldRequirement {
            name: "id".to_string(),
            r#type: "integer".to_string(),
            description: "Product ID".to_string(),
            required: true,
        };

        let model = ModelRequirement {
            name: "Product".to_string(),
            fields: vec![field],
        };

        assert_eq!(model.name, "Product");
        assert_eq!(model.fields.len(), 1);
        assert_eq!(model.fields[0].name, "id");
    }

    #[test]
    fn test_field_requirement_creation() {
        let field = FieldRequirement {
            name: "name".to_string(),
            r#type: "string".to_string(),
            description: "Product name".to_string(),
            required: true,
        };

        assert_eq!(field.name, "name");
        assert_eq!(field.r#type, "string");
        assert!(field.required);
    }

    #[test]
    fn test_field_requirement_default_required() {
        let field = FieldRequirement {
            name: "optional_field".to_string(),
            r#type: "string".to_string(),
            description: "".to_string(),
            required: default_true(),
        };

        assert!(field.required);
    }

    #[test]
    fn test_relationship_requirement_creation() {
        let relationship = RelationshipRequirement {
            from: "Product".to_string(),
            to: "Category".to_string(),
            r#type: "many-to-one".to_string(),
        };

        assert_eq!(relationship.from, "Product");
        assert_eq!(relationship.to, "Category");
        assert_eq!(relationship.r#type, "many-to-one");
    }

    #[test]
    fn test_flow_requirement_creation() {
        let flow = FlowRequirement {
            name: "checkout".to_string(),
            description: "Checkout flow".to_string(),
            steps: vec!["Add to cart".to_string(), "Payment".to_string()],
        };

        assert_eq!(flow.name, "checkout");
        assert_eq!(flow.steps.len(), 2);
    }

    #[test]
    fn test_parsed_workspace_scenario_creation() {
        let scenario = ParsedWorkspaceScenario {
            domain: "e-commerce".to_string(),
            title: "Shop Workspace".to_string(),
            description: "E-commerce workspace".to_string(),
            chaos_characteristics: vec![],
            initial_data: InitialDataRequirements::default(),
            api_requirements: ApiRequirements::default(),
            behavioral_rules: vec![],
        };

        assert_eq!(scenario.domain, "e-commerce");
        assert_eq!(scenario.title, "Shop Workspace");
    }

    #[test]
    fn test_chaos_characteristic_creation() {
        let chaos = ChaosCharacteristic {
            r#type: "latency".to_string(),
            description: "High latency on checkout".to_string(),
            config: json!({"delay_ms": 1000}),
        };

        assert_eq!(chaos.r#type, "latency");
        assert_eq!(chaos.description, "High latency on checkout");
    }

    #[test]
    fn test_initial_data_requirements_creation() {
        let mut custom = HashMap::new();
        custom.insert("products".to_string(), 50);

        let data = InitialDataRequirements {
            users: Some(100),
            disputes: Some(5),
            orders: Some(200),
            custom,
        };

        assert_eq!(data.users, Some(100));
        assert_eq!(data.disputes, Some(5));
        assert_eq!(data.orders, Some(200));
        assert_eq!(data.custom.get("products"), Some(&50));
    }

    #[test]
    fn test_initial_data_requirements_default() {
        let data = InitialDataRequirements::default();
        assert!(data.users.is_none());
        assert!(data.disputes.is_none());
        assert!(data.orders.is_none());
        assert!(data.custom.is_empty());
    }

    #[test]
    fn test_api_requirements_creation() {
        let endpoint = EndpointRequirement {
            path: "/api/products".to_string(),
            method: "GET".to_string(),
            description: "Get products".to_string(),
            request_body: None,
            response: None,
        };

        let model = ModelRequirement {
            name: "Product".to_string(),
            fields: vec![],
        };

        let api_req = ApiRequirements {
            endpoints: vec![endpoint],
            models: vec![model],
        };

        assert_eq!(api_req.endpoints.len(), 1);
        assert_eq!(api_req.models.len(), 1);
    }

    #[test]
    fn test_api_requirements_default() {
        let api_req = ApiRequirements::default();
        assert!(api_req.endpoints.is_empty());
        assert!(api_req.models.is_empty());
    }

    #[test]
    fn test_behavioral_rule_creation() {
        let rule = BehavioralRule {
            description: "Slow response on checkout".to_string(),
            r#type: "latency".to_string(),
            config: json!({"delay_ms": 2000}),
        };

        assert_eq!(rule.description, "Slow response on checkout");
        assert_eq!(rule.r#type, "latency");
    }

    #[test]
    fn test_parsed_workspace_creation_creation() {
        let creation = ParsedWorkspaceCreation {
            workspace_name: "New Workspace".to_string(),
            workspace_description: "A new workspace".to_string(),
            entities: vec![],
            personas: vec![],
            scenarios: vec![],
            reality_continuum: None,
            drift_budget: None,
        };

        assert_eq!(creation.workspace_name, "New Workspace");
        assert_eq!(creation.workspace_description, "A new workspace");
        assert!(creation.entities.is_empty());
    }

    #[test]
    fn test_entity_requirement_creation() {
        let entity = EntityRequirement {
            name: "Product".to_string(),
            description: "Product entity".to_string(),
            endpoints: vec![],
            fields: vec![],
        };

        assert_eq!(entity.name, "Product");
        assert_eq!(entity.description, "Product entity");
        assert!(entity.fields.is_empty());
    }

    #[test]
    fn test_entity_endpoint_requirement_creation() {
        let endpoint = EntityEndpointRequirement {
            path: "/api/products".to_string(),
            method: "GET".to_string(),
            description: "Get products".to_string(),
        };

        assert_eq!(endpoint.path, "/api/products");
        assert_eq!(endpoint.method, "GET");
    }

    #[test]
    fn test_persona_requirement_creation() {
        let persona = PersonaRequirement {
            name: "Customer".to_string(),
            description: "Regular customer".to_string(),
            traits: HashMap::new(),
            relationships: vec![],
        };

        assert_eq!(persona.name, "Customer");
        assert_eq!(persona.description, "Regular customer");
        assert!(persona.traits.is_empty());
    }

    #[test]
    fn test_persona_relationship_creation() {
        let relationship = PersonaRelationship {
            r#type: "one-to-many".to_string(),
            target_entity: "Order".to_string(),
        };

        assert_eq!(relationship.r#type, "one-to-many");
        assert_eq!(relationship.target_entity, "Order");
    }

    #[test]
    fn test_parsed_reality_continuum_creation() {
        let continuum = ParsedRealityContinuum {
            default_ratio: 0.2,
            enabled: true,
            route_rules: vec![],
            transition_mode: "manual".to_string(),
            merge_strategy: "field_level".to_string(),
        };

        assert_eq!(continuum.default_ratio, 0.2);
        assert!(continuum.enabled);
        assert_eq!(continuum.transition_mode, "manual");
        assert_eq!(continuum.merge_strategy, "field_level");
    }

    #[test]
    fn test_parsed_continuum_rule_creation() {
        let rule = ParsedContinuumRule {
            pattern: "/api/catalog/*".to_string(),
            ratio: 0.5,
            description: "Catalog route".to_string(),
        };

        assert_eq!(rule.pattern, "/api/catalog/*");
        assert_eq!(rule.ratio, 0.5);
    }

    #[test]
    fn test_parsed_drift_budget_creation() {
        let mut per_service_budgets = HashMap::new();
        per_service_budgets.insert(
            "catalog".to_string(),
            ParsedServiceBudget {
                max_breaking_changes: 5,
                max_non_breaking_changes: 20,
            },
        );

        let budget = ParsedDriftBudget {
            strictness: "moderate".to_string(),
            enabled: true,
            max_breaking_changes: 10,
            max_non_breaking_changes: 50,
            max_field_churn_percent: Some(5.0),
            time_window_days: Some(30),
            per_service_budgets,
            description: "Drift budget config".to_string(),
        };

        assert_eq!(budget.strictness, "moderate");
        assert!(budget.enabled);
        assert_eq!(budget.max_breaking_changes, 10);
        assert_eq!(budget.max_non_breaking_changes, 50);
        assert_eq!(budget.per_service_budgets.len(), 1);
    }

    #[test]
    fn test_parsed_service_budget_creation() {
        let budget = ParsedServiceBudget {
            max_breaking_changes: 3,
            max_non_breaking_changes: 15,
        };

        assert_eq!(budget.max_breaking_changes, 3);
        assert_eq!(budget.max_non_breaking_changes, 15);
    }

    #[test]
    fn test_parsed_command_clone() {
        let command1 = ParsedCommand {
            api_type: "test".to_string(),
            title: "Test API".to_string(),
            description: "Test".to_string(),
            endpoints: vec![],
            models: vec![],
            relationships: vec![],
            sample_counts: HashMap::new(),
            flows: vec![],
        };
        let command2 = command1.clone();
        assert_eq!(command1.api_type, command2.api_type);
    }

    #[test]
    fn test_parsed_command_debug() {
        let command = ParsedCommand {
            api_type: "test".to_string(),
            title: "Test".to_string(),
            description: "Test".to_string(),
            endpoints: vec![],
            models: vec![],
            relationships: vec![],
            sample_counts: HashMap::new(),
            flows: vec![],
        };
        let debug_str = format!("{:?}", command);
        assert!(debug_str.contains("ParsedCommand"));
    }

    #[test]
    fn test_endpoint_requirement_clone() {
        let endpoint1 = EndpointRequirement {
            path: "/test".to_string(),
            method: "GET".to_string(),
            description: "Test".to_string(),
            request_body: None,
            response: None,
        };
        let endpoint2 = endpoint1.clone();
        assert_eq!(endpoint1.path, endpoint2.path);
    }

    #[test]
    fn test_endpoint_requirement_debug() {
        let endpoint = EndpointRequirement {
            path: "/test".to_string(),
            method: "POST".to_string(),
            description: "Test".to_string(),
            request_body: None,
            response: None,
        };
        let debug_str = format!("{:?}", endpoint);
        assert!(debug_str.contains("EndpointRequirement"));
    }

    #[test]
    fn test_request_body_requirement_clone() {
        let body1 = RequestBodyRequirement {
            schema: None,
            required: vec!["field".to_string()],
        };
        let body2 = body1.clone();
        assert_eq!(body1.required, body2.required);
    }

    #[test]
    fn test_request_body_requirement_debug() {
        let body = RequestBodyRequirement {
            schema: Some(json!({})),
            required: vec![],
        };
        let debug_str = format!("{:?}", body);
        assert!(debug_str.contains("RequestBodyRequirement"));
    }

    #[test]
    fn test_response_requirement_clone() {
        let response1 = ResponseRequirement {
            status: 200,
            schema: None,
            is_array: false,
            count: None,
        };
        let response2 = response1.clone();
        assert_eq!(response1.status, response2.status);
    }

    #[test]
    fn test_response_requirement_debug() {
        let response = ResponseRequirement {
            status: 201,
            schema: Some(json!({})),
            is_array: true,
            count: Some(10),
        };
        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("ResponseRequirement"));
    }

    #[test]
    fn test_model_requirement_clone() {
        let model1 = ModelRequirement {
            name: "User".to_string(),
            fields: vec![],
        };
        let model2 = model1.clone();
        assert_eq!(model1.name, model2.name);
    }

    #[test]
    fn test_model_requirement_debug() {
        let model = ModelRequirement {
            name: "Product".to_string(),
            fields: vec![],
        };
        let debug_str = format!("{:?}", model);
        assert!(debug_str.contains("ModelRequirement"));
    }

    #[test]
    fn test_field_requirement_clone() {
        let field1 = FieldRequirement {
            name: "id".to_string(),
            r#type: "integer".to_string(),
            description: "ID".to_string(),
            required: true,
        };
        let field2 = field1.clone();
        assert_eq!(field1.name, field2.name);
    }

    #[test]
    fn test_field_requirement_debug() {
        let field = FieldRequirement {
            name: "name".to_string(),
            r#type: "string".to_string(),
            description: "Name".to_string(),
            required: false,
        };
        let debug_str = format!("{:?}", field);
        assert!(debug_str.contains("FieldRequirement"));
    }

    #[test]
    fn test_relationship_requirement_clone() {
        let rel1 = RelationshipRequirement {
            from: "User".to_string(),
            to: "Order".to_string(),
            r#type: "one-to-many".to_string(),
        };
        let rel2 = rel1.clone();
        assert_eq!(rel1.from, rel2.from);
    }

    #[test]
    fn test_relationship_requirement_debug() {
        let rel = RelationshipRequirement {
            from: "Product".to_string(),
            to: "Category".to_string(),
            r#type: "many-to-one".to_string(),
        };
        let debug_str = format!("{:?}", rel);
        assert!(debug_str.contains("RelationshipRequirement"));
    }

    #[test]
    fn test_flow_requirement_clone() {
        let flow1 = FlowRequirement {
            name: "checkout".to_string(),
            description: "Checkout".to_string(),
            steps: vec![],
        };
        let flow2 = flow1.clone();
        assert_eq!(flow1.name, flow2.name);
    }

    #[test]
    fn test_flow_requirement_debug() {
        let flow = FlowRequirement {
            name: "auth".to_string(),
            description: "Auth flow".to_string(),
            steps: vec!["step1".to_string()],
        };
        let debug_str = format!("{:?}", flow);
        assert!(debug_str.contains("FlowRequirement"));
    }

    #[test]
    fn test_parsed_workspace_scenario_clone() {
        let scenario1 = ParsedWorkspaceScenario {
            domain: "e-commerce".to_string(),
            title: "Shop".to_string(),
            description: "Shop".to_string(),
            chaos_characteristics: vec![],
            initial_data: InitialDataRequirements::default(),
            api_requirements: ApiRequirements::default(),
            behavioral_rules: vec![],
        };
        let scenario2 = scenario1.clone();
        assert_eq!(scenario1.domain, scenario2.domain);
    }

    #[test]
    fn test_parsed_workspace_scenario_debug() {
        let scenario = ParsedWorkspaceScenario {
            domain: "social".to_string(),
            title: "Social".to_string(),
            description: "Social".to_string(),
            chaos_characteristics: vec![],
            initial_data: InitialDataRequirements::default(),
            api_requirements: ApiRequirements::default(),
            behavioral_rules: vec![],
        };
        let debug_str = format!("{:?}", scenario);
        assert!(debug_str.contains("ParsedWorkspaceScenario"));
    }

    #[test]
    fn test_chaos_characteristic_clone() {
        let chaos1 = ChaosCharacteristic {
            r#type: "latency".to_string(),
            description: "High latency".to_string(),
            config: json!({}),
        };
        let chaos2 = chaos1.clone();
        assert_eq!(chaos1.r#type, chaos2.r#type);
    }

    #[test]
    fn test_chaos_characteristic_debug() {
        let chaos = ChaosCharacteristic {
            r#type: "failure".to_string(),
            description: "Failures".to_string(),
            config: json!({"rate": 0.1}),
        };
        let debug_str = format!("{:?}", chaos);
        assert!(debug_str.contains("ChaosCharacteristic"));
    }

    #[test]
    fn test_initial_data_requirements_clone() {
        let data1 = InitialDataRequirements::default();
        let data2 = data1.clone();
        // Just verify it doesn't panic
        assert_eq!(data1.users, data2.users);
    }

    #[test]
    fn test_initial_data_requirements_debug() {
        let data = InitialDataRequirements::default();
        let debug_str = format!("{:?}", data);
        assert!(debug_str.contains("InitialDataRequirements"));
    }

    #[test]
    fn test_api_requirements_clone() {
        let api1 = ApiRequirements::default();
        let api2 = api1.clone();
        assert_eq!(api1.endpoints.len(), api2.endpoints.len());
    }

    #[test]
    fn test_api_requirements_debug() {
        let api = ApiRequirements::default();
        let debug_str = format!("{:?}", api);
        assert!(debug_str.contains("ApiRequirements"));
    }

    #[test]
    fn test_behavioral_rule_clone() {
        let rule1 = BehavioralRule {
            description: "Rule".to_string(),
            r#type: "failure".to_string(),
            config: json!({}),
        };
        let rule2 = rule1.clone();
        assert_eq!(rule1.description, rule2.description);
    }

    #[test]
    fn test_behavioral_rule_debug() {
        let rule = BehavioralRule {
            description: "Test rule".to_string(),
            r#type: "latency".to_string(),
            config: json!({"delay": 100}),
        };
        let debug_str = format!("{:?}", rule);
        assert!(debug_str.contains("BehavioralRule"));
    }

    #[test]
    fn test_parsed_workspace_creation_clone() {
        let creation1 = ParsedWorkspaceCreation {
            workspace_name: "Test".to_string(),
            workspace_description: "Test".to_string(),
            entities: vec![],
            personas: vec![],
            scenarios: vec![],
            reality_continuum: None,
            drift_budget: None,
        };
        let creation2 = creation1.clone();
        assert_eq!(creation1.workspace_name, creation2.workspace_name);
    }

    #[test]
    fn test_parsed_workspace_creation_debug() {
        let creation = ParsedWorkspaceCreation {
            workspace_name: "Workspace".to_string(),
            workspace_description: "Description".to_string(),
            entities: vec![],
            personas: vec![],
            scenarios: vec![],
            reality_continuum: None,
            drift_budget: None,
        };
        let debug_str = format!("{:?}", creation);
        assert!(debug_str.contains("ParsedWorkspaceCreation"));
    }

    #[test]
    fn test_parsed_reality_continuum_clone() {
        let continuum1 = ParsedRealityContinuum {
            default_ratio: 0.5,
            enabled: true,
            route_rules: vec![],
            transition_mode: "manual".to_string(),
            merge_strategy: "field_level".to_string(),
        };
        let continuum2 = continuum1.clone();
        assert_eq!(continuum1.default_ratio, continuum2.default_ratio);
    }

    #[test]
    fn test_parsed_reality_continuum_debug() {
        let continuum = ParsedRealityContinuum {
            default_ratio: 0.2,
            enabled: true,
            route_rules: vec![],
            transition_mode: "time_based".to_string(),
            merge_strategy: "weighted".to_string(),
        };
        let debug_str = format!("{:?}", continuum);
        assert!(debug_str.contains("ParsedRealityContinuum"));
    }

    #[test]
    fn test_parsed_continuum_rule_clone() {
        let rule1 = ParsedContinuumRule {
            pattern: "/api/*".to_string(),
            ratio: 0.3,
            description: "Test".to_string(),
        };
        let rule2 = rule1.clone();
        assert_eq!(rule1.pattern, rule2.pattern);
    }

    #[test]
    fn test_parsed_continuum_rule_debug() {
        let rule = ParsedContinuumRule {
            pattern: "/catalog/*".to_string(),
            ratio: 0.5,
            description: "Catalog".to_string(),
        };
        let debug_str = format!("{:?}", rule);
        assert!(debug_str.contains("ParsedContinuumRule"));
    }

    #[test]
    fn test_parsed_drift_budget_clone() {
        let budget1 = ParsedDriftBudget {
            strictness: "moderate".to_string(),
            enabled: true,
            max_breaking_changes: 10,
            max_non_breaking_changes: 50,
            max_field_churn_percent: None,
            time_window_days: None,
            per_service_budgets: HashMap::new(),
            description: "Budget".to_string(),
        };
        let budget2 = budget1.clone();
        assert_eq!(budget1.strictness, budget2.strictness);
    }

    #[test]
    fn test_parsed_drift_budget_debug() {
        let budget = ParsedDriftBudget {
            strictness: "strict".to_string(),
            enabled: true,
            max_breaking_changes: 5,
            max_non_breaking_changes: 20,
            max_field_churn_percent: Some(3.0),
            time_window_days: Some(7),
            per_service_budgets: HashMap::new(),
            description: "Strict budget".to_string(),
        };
        let debug_str = format!("{:?}", budget);
        assert!(debug_str.contains("ParsedDriftBudget"));
    }

    #[test]
    fn test_parsed_service_budget_clone() {
        let budget1 = ParsedServiceBudget {
            max_breaking_changes: 3,
            max_non_breaking_changes: 15,
        };
        let budget2 = budget1.clone();
        assert_eq!(budget1.max_breaking_changes, budget2.max_breaking_changes);
    }

    #[test]
    fn test_parsed_service_budget_debug() {
        let budget = ParsedServiceBudget {
            max_breaking_changes: 5,
            max_non_breaking_changes: 25,
        };
        let debug_str = format!("{:?}", budget);
        assert!(debug_str.contains("ParsedServiceBudget"));
    }
}
