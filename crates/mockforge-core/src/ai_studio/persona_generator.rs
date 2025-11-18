//! AI-powered persona generator
//!
//! This module provides functionality to generate and tweak personas using AI.
//! It creates personas with realistic traits, backstories, and lifecycle configurations
//! based on natural language descriptions.

use crate::intelligent_behavior::llm_client::LlmClient;
use crate::intelligent_behavior::types::LlmGenerationRequest;
use crate::intelligent_behavior::IntelligentBehaviorConfig;
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Persona generator for creating personas from descriptions
pub struct PersonaGenerator {
    /// LLM client for generating persona details
    llm_client: LlmClient,
}

impl PersonaGenerator {
    /// Create a new persona generator with default configuration
    pub fn new() -> Self {
        let config = IntelligentBehaviorConfig::default();
        Self {
            llm_client: LlmClient::new(config.behavior_model),
        }
    }

    /// Create a new persona generator with custom configuration
    pub fn with_config(config: IntelligentBehaviorConfig) -> Self {
        Self {
            llm_client: LlmClient::new(config.behavior_model),
        }
    }

    /// Generate a persona from natural language description
    ///
    /// This method uses AI to generate a complete persona profile including:
    /// - Realistic traits based on the description
    /// - A narrative backstory
    /// - Appropriate lifecycle configuration
    /// - Domain-specific characteristics
    pub async fn generate(
        &self,
        request: &PersonaGenerationRequest,
    ) -> Result<PersonaGenerationResponse> {
        // Build system prompt for persona generation
        let system_prompt = r#"You are an expert at creating realistic user personas for API testing.
Generate a complete persona profile from a natural language description.

For the persona, provide:
1. A unique ID (e.g., "user:premium-001", "customer:churned-002")
2. A descriptive name
3. A business domain (e.g., "ecommerce", "saas", "banking", "healthcare")
4. Realistic traits as key-value pairs (e.g., "subscription_tier": "premium", "spending_level": "high")
5. A narrative backstory explaining the persona's characteristics
6. Optional lifecycle state (e.g., "active", "trial", "churned", "premium")

Return your response as a JSON object with this structure:
{
  "id": "string (unique persona ID)",
  "name": "string (descriptive name)",
  "domain": "string (business domain)",
  "traits": {
    "trait_name": "trait_value",
    ...
  },
  "backstory": "string (narrative description)",
  "lifecycle_state": "string (optional, e.g., active, trial, churned)",
  "metadata": {
    "additional": "metadata fields"
  }
}

Make the persona realistic and consistent. Traits should align with the description."#;

        let user_prompt =
            format!("Generate a persona from this description:\n\n{}", request.description);

        let llm_request = LlmGenerationRequest {
            system_prompt: system_prompt.to_string(),
            user_prompt,
            temperature: 0.7, // Higher temperature for more creative personas
            max_tokens: 1500,
            schema: None,
        };

        // Generate persona from LLM
        let response = self.llm_client.generate(&llm_request).await?;

        // Parse the response into a persona structure
        let persona_json = if let Some(id) = response.get("id") {
            // Full persona structure
            response.clone()
        } else {
            // Fallback: create a basic persona structure
            let uuid_str = uuid::Uuid::new_v4().to_string();
            let short_id = uuid_str.split('-').next().unwrap_or("generated");
            serde_json::json!({
                "id": format!("user:generated-{}", short_id),
                "name": response.get("name").and_then(|v| v.as_str()).unwrap_or("Generated Persona"),
                "domain": response.get("domain").and_then(|v| v.as_str()).unwrap_or("general"),
                "traits": response.get("traits").cloned().unwrap_or_else(|| serde_json::json!({})),
                "backstory": response.get("backstory").and_then(|v| v.as_str()).unwrap_or("AI-generated persona"),
                "lifecycle_state": response.get("lifecycle_state").and_then(|v| v.as_str()).unwrap_or("active"),
            })
        };

        // Convert to the simpler Persona format for response
        let persona_name = persona_json
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Generated Persona")
            .to_string();

        let traits: HashMap<String, String> = persona_json
            .get("traits")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        // Build response persona (using the simpler Persona struct format)
        let persona_value = serde_json::json!({
            "name": persona_name,
            "traits": traits,
            "id": persona_json.get("id"),
            "domain": persona_json.get("domain"),
            "backstory": persona_json.get("backstory"),
            "lifecycle_state": persona_json.get("lifecycle_state"),
        });

        Ok(PersonaGenerationResponse {
            persona: Some(persona_value),
            message: format!(
                "Successfully generated persona '{}' with {} traits",
                persona_name,
                traits.len()
            ),
        })
    }

    /// Tweak an existing persona based on a description
    ///
    /// This method modifies an existing persona by adjusting traits, adding new ones,
    /// or updating the backstory based on the provided description.
    pub async fn tweak(
        &self,
        base_persona: &serde_json::Value,
        description: &str,
    ) -> Result<PersonaGenerationResponse> {
        // Build system prompt for persona tweaking
        let system_prompt = r#"You are an expert at modifying user personas for API testing.
Given an existing persona and a description of desired changes, update the persona accordingly.

You can:
- Modify existing traits
- Add new traits
- Update the backstory
- Change lifecycle state
- Adjust domain if needed

Return the updated persona in the same JSON structure as the input."#;

        let user_prompt = format!(
            "Base persona:\n{}\n\nDesired changes: {}\n\nProvide the updated persona.",
            serde_json::to_string_pretty(base_persona)?,
            description
        );

        let llm_request = LlmGenerationRequest {
            system_prompt: system_prompt.to_string(),
            user_prompt,
            temperature: 0.5,
            max_tokens: 1500,
            schema: None,
        };

        // Generate updated persona
        let response = self.llm_client.generate(&llm_request).await?;

        Ok(PersonaGenerationResponse {
            persona: Some(response),
            message: "Successfully updated persona".to_string(),
        })
    }
}

impl Default for PersonaGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Request for persona generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaGenerationRequest {
    /// Natural language description
    pub description: String,

    /// Optional base persona to tweak
    pub base_persona_id: Option<String>,

    /// Workspace ID for context
    pub workspace_id: Option<String>,
}

/// Response from persona generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaGenerationResponse {
    /// Generated persona (if any)
    pub persona: Option<serde_json::Value>,

    /// Status message
    pub message: String,
}
