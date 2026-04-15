//! AI Behavioral Simulation Engine
//!
//! This module provides functionality to model users as narrative agents that:
//! - React to app state (e.g., "cart is empty" → intention: "browse products")
//! - Form intentions (shop, browse, buy, abandon)
//! - Respond to errors (rage clicking on 500 errors, retry logic, cart abandonment on payment failure)
//! - Trigger multi-step interactions automatically
//! - Maintain session context across interactions
//!
//! # Persona Integration Strategy
//!
//! - **Primary: Augment existing personas** - Attach behavior policies to existing Smart Personas
//! - **Secondary: Generate new personas** - When system description introduces roles that don't exist
//!
//! # Example Usage
//!
//! ```rust,ignore
//! use mockforge_core::ai_studio::behavioral_simulator::{BehavioralSimulator, CreateAgentRequest};
//! use mockforge_core::intelligent_behavior::IntelligentBehaviorConfig;
//!
//! async fn example() -> mockforge_core::Result<()> {
//!     let config = IntelligentBehaviorConfig::default();
//!     let simulator = BehavioralSimulator::new(config);
//!
//!     let request = CreateAgentRequest {
//!         persona_id: Some("existing-persona-123".to_string()),
//!         behavior_policy: Some("bargain-hunter".to_string()),
//!         generate_persona: false,
//!     };
//!
//!     let agent = simulator.create_agent(&request).await?;
//!     Ok(())
//! }
//! ```

use crate::intelligent_behavior::{
    config::IntelligentBehaviorConfig,
    llm_client::{LlmClient, LlmUsage},
    types::LlmGenerationRequest,
};
use crate::Result;
use chrono::Utc;
// Data types re-exported from foundation.
pub use mockforge_foundation::ai_studio_types::{
    AppState, BehaviorPolicy, BehavioralTraits, CartItem, CartState, CreateAgentRequest,
    ErrorEncounter, Intention, Interaction, NarrativeAgent, NextAction, PolicyRule,
    SimulateBehaviorRequest, SimulateBehaviorResponse,
};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Behavioral Simulator Engine
pub struct BehavioralSimulator {
    /// LLM client for behavior generation
    llm_client: LlmClient,

    /// Configuration
    config: IntelligentBehaviorConfig,

    /// Active agents (in-memory storage - in production, use database)
    agents: HashMap<String, NarrativeAgent>,

    /// Configuration for persona integration
    /// Whether to use existing personas when creating agents (primary mode)
    pub use_existing_personas: bool,
    /// Whether to allow generating new personas when needed (secondary mode)
    pub allow_new_personas: bool,
    /// Maximum number of new personas that can be generated
    pub max_new_personas: usize,
}

impl BehavioralSimulator {
    /// Create a new behavioral simulator
    pub fn new(config: IntelligentBehaviorConfig) -> Self {
        let llm_client = LlmClient::new(config.behavior_model.clone());
        Self {
            llm_client,
            config,
            agents: HashMap::new(),
            use_existing_personas: true,
            allow_new_personas: true,
            max_new_personas: 5,
        }
    }

    /// Create with persona integration settings
    pub fn with_persona_settings(
        config: IntelligentBehaviorConfig,
        use_existing_personas: bool,
        allow_new_personas: bool,
        max_new_personas: usize,
    ) -> Self {
        let llm_client = LlmClient::new(config.behavior_model.clone());
        Self {
            llm_client,
            config,
            agents: HashMap::new(),
            use_existing_personas,
            allow_new_personas,
            max_new_personas,
        }
    }

    /// Create a new narrative agent
    pub async fn create_agent(&mut self, request: &CreateAgentRequest) -> Result<NarrativeAgent> {
        let agent_id = format!("agent-{}", Uuid::new_v4());

        // Determine persona ID
        let persona_id = if let Some(ref existing_id) = request.persona_id {
            // Use existing persona if provided
            if self.use_existing_personas {
                existing_id.clone()
            } else {
                return Err(crate::Error::internal(
                    "Using existing personas is disabled".to_string(),
                ));
            }
        } else if request.generate_persona {
            // Generate new persona if allowed
            if !self.allow_new_personas {
                return Err(crate::Error::internal(
                    "Generating new personas is disabled".to_string(),
                ));
            }

            // Check limit
            let new_persona_count =
                self.agents.values().filter(|a| !a.persona_id.starts_with("existing-")).count();

            if new_persona_count >= self.max_new_personas {
                return Err(crate::Error::internal(format!(
                    "Maximum new personas limit ({}) reached",
                    self.max_new_personas
                )));
            }

            // Generate new persona ID (in production, would call persona generator)
            format!("persona-{}", Uuid::new_v4())
        } else {
            return Err(crate::Error::internal(
                "Either persona_id or generate_persona must be provided".to_string(),
            ));
        };

        // Generate behavior policy
        let behavior_policy = if let Some(ref policy_type) = request.behavior_policy {
            self.generate_behavior_policy(policy_type).await?
        } else {
            // Default policy
            BehaviorPolicy {
                policy_type: "default".to_string(),
                description: "Default user behavior".to_string(),
                rules: vec![],
            }
        };

        // Create agent
        let agent = NarrativeAgent {
            agent_id: agent_id.clone(),
            persona_id,
            current_intention: Intention::Browse,
            session_history: Vec::new(),
            behavioral_traits: BehavioralTraits {
                patience: 0.7,
                price_sensitivity: 0.5,
                risk_tolerance: 0.5,
                technical_proficiency: 0.5,
                engagement_level: 0.7,
            },
            state_awareness: AppState::default(),
            behavior_policy,
            created_at: Utc::now().to_rfc3339(),
        };

        self.agents.insert(agent_id.clone(), agent.clone());
        Ok(agent)
    }

    /// Simulate behavior based on current state and trigger event
    pub async fn simulate_behavior(
        &mut self,
        request: &SimulateBehaviorRequest,
    ) -> Result<SimulateBehaviorResponse> {
        // Get or create agent (clone to avoid borrow conflicts)
        let mut agent = if let Some(ref agent_id) = request.agent_id {
            self.agents
                .get(agent_id)
                .ok_or_else(|| crate::Error::internal("Agent not found".to_string()))?
                .clone()
        } else if let Some(ref persona_id) = request.persona_id {
            // Find existing agent for persona or create new one
            let existing_agent =
                self.agents.values().find(|a| a.persona_id == *persona_id).cloned();

            if let Some(mut agent) = existing_agent {
                // Update state
                agent.state_awareness = request.current_state.clone();
                agent
            } else {
                // Create new agent for persona
                let create_request = CreateAgentRequest {
                    persona_id: Some(persona_id.clone()),
                    behavior_policy: None,
                    generate_persona: false,
                    workspace_id: request.workspace_id.clone(),
                };
                self.create_agent(&create_request).await?
            }
        } else {
            return Err(crate::Error::internal(
                "Either agent_id or persona_id must be provided".to_string(),
            ));
        };

        // Update agent state
        agent.state_awareness = request.current_state.clone();

        // Extract values needed for LLM call
        let behavior_policy = agent.behavior_policy.clone();
        let agent_clone = agent.clone();
        let trigger_event_clone = request.trigger_event.clone();

        // Generate next action using LLM
        let system_prompt = self.build_system_prompt(&behavior_policy);
        let user_prompt = self.build_user_prompt(&agent_clone, &trigger_event_clone)?;

        let llm_request = LlmGenerationRequest {
            system_prompt,
            user_prompt,
            temperature: 0.8, // Higher temperature for more varied behavior
            max_tokens: 1000,
            schema: None,
        };

        let (response_json, usage) = self.llm_client.generate_with_usage(&llm_request).await?;

        // Parse response (clone response_json since we need it multiple times)
        let response_json_clone = response_json.clone();
        let next_action = self.parse_action_response(response_json)?;
        let intention = self.determine_intention(&next_action, &trigger_event_clone)?;
        let reasoning = self.extract_reasoning(&response_json_clone)?;

        // Record interaction
        let interaction = Interaction {
            timestamp: Utc::now().to_rfc3339(),
            action: next_action.action_type.clone(),
            intention: intention.clone(),
            request: next_action.body.clone(),
            response: None,
            result: "pending".to_string(),
        };
        agent.session_history.push(interaction);
        agent.current_intention = intention.clone();

        // Update agent in storage
        self.agents.insert(agent.agent_id.clone(), agent.clone());

        // Calculate cost
        let cost_usd = self.estimate_cost(&usage);

        Ok(SimulateBehaviorResponse {
            next_action,
            intention,
            reasoning,
            agent: Some(agent.clone()),
            tokens_used: Some(usage.total_tokens),
            cost_usd: Some(cost_usd),
        })
    }

    /// Generate behavior policy for a policy type
    async fn generate_behavior_policy(&self, policy_type: &str) -> Result<BehaviorPolicy> {
        // In a full implementation, this would use LLM to generate policy
        // For now, return a template based on policy type
        let (description, rules) = match policy_type {
            "bargain-hunter" => (
                "Price-sensitive user who looks for deals and discounts".to_string(),
                vec![
                    PolicyRule {
                        condition: "price > threshold".to_string(),
                        action: "abandon".to_string(),
                        priority: 10,
                    },
                    PolicyRule {
                        condition: "discount_available".to_string(),
                        action: "buy".to_string(),
                        priority: 9,
                    },
                ],
            ),
            "power-user" => (
                "Highly engaged user with advanced features".to_string(),
                vec![
                    PolicyRule {
                        condition: "error_encountered".to_string(),
                        action: "retry".to_string(),
                        priority: 10,
                    },
                    PolicyRule {
                        condition: "feature_available".to_string(),
                        action: "explore".to_string(),
                        priority: 8,
                    },
                ],
            ),
            "churn-risk" => (
                "User showing signs of churn".to_string(),
                vec![
                    PolicyRule {
                        condition: "error_encountered".to_string(),
                        action: "abandon".to_string(),
                        priority: 10,
                    },
                    PolicyRule {
                        condition: "slow_response".to_string(),
                        action: "abandon".to_string(),
                        priority: 9,
                    },
                ],
            ),
            _ => ("Default user behavior".to_string(), vec![]),
        };

        Ok(BehaviorPolicy {
            policy_type: policy_type.to_string(),
            description,
            rules,
        })
    }

    /// Build system prompt for behavior simulation
    fn build_system_prompt(&self, behavior_policy: &BehaviorPolicy) -> String {
        format!(
            r#"You are modeling a user's behavior in a web application. Your task is to determine what action the user would take next based on:

1. Current app state (cart, authentication, recent errors, etc.)
2. User's current intention (browse, shop, buy, abandon, retry, navigate)
3. Behavioral traits (patience, price sensitivity, risk tolerance, etc.)
4. Behavior policy: {}

Return a JSON object with:
{{
  "action_type": "GET|POST|navigate|abandon",
  "target": "/api/endpoint or page name",
  "body": {{ ... }} (optional, for POST requests),
  "query_params": {{ ... }} (optional),
  "delay_ms": 1000 (optional, delay before action),
  "reasoning": "Why this action makes sense for this user"
}}

Consider:
- User's patience level when encountering errors
- Price sensitivity when making purchase decisions
- Engagement level for exploration vs. quick actions
- Recent errors may trigger retry or abandon
- Empty cart may trigger browse intention
- Payment failures may trigger abandon or retry based on patience"#,
            behavior_policy.description
        )
    }

    /// Build user prompt with current state and trigger
    fn build_user_prompt(
        &self,
        agent: &NarrativeAgent,
        trigger_event: &Option<String>,
    ) -> Result<String> {
        let state_json = serde_json::to_string_pretty(&agent.state_awareness)
            .map_err(|e| crate::Error::internal(format!("Failed to serialize state: {}", e)))?;

        let trigger_text = trigger_event
            .as_ref()
            .map(|e| format!("Trigger event: {}", e))
            .unwrap_or_else(|| "No specific trigger".to_string());

        Ok(format!(
            r#"Current user state:
{}

Current intention: {:?}
Behavioral traits: patience={:.2}, price_sensitivity={:.2}, risk_tolerance={:.2}
Session history: {} interactions
{}

What should the user do next?"#,
            state_json,
            agent.current_intention,
            agent.behavioral_traits.patience,
            agent.behavioral_traits.price_sensitivity,
            agent.behavioral_traits.risk_tolerance,
            agent.session_history.len(),
            trigger_text
        ))
    }

    /// Parse LLM response into NextAction
    fn parse_action_response(&self, response: Value) -> Result<NextAction> {
        // Try to extract action from response
        let action_json = if let Some(action) = response.get("action") {
            action.clone()
        } else if response.is_object() {
            response
        } else {
            return Err(crate::Error::internal(
                "LLM response is not a valid JSON object".to_string(),
            ));
        };

        let action_type = action_json
            .get("action_type")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_string();

        let target = action_json.get("target").and_then(|v| v.as_str()).unwrap_or("/").to_string();

        let body = action_json.get("body").cloned();
        let query_params = action_json
            .get("query_params")
            .and_then(|v| serde_json::from_value(v.clone()).ok());

        let delay_ms = action_json.get("delay_ms").and_then(|v| v.as_u64());

        Ok(NextAction {
            action_type,
            target,
            body,
            query_params,
            delay_ms,
        })
    }

    /// Determine intention from action and trigger
    fn determine_intention(
        &self,
        action: &NextAction,
        trigger_event: &Option<String>,
    ) -> Result<Intention> {
        // Determine intention based on action and trigger
        if let Some(ref trigger) = trigger_event {
            if trigger.contains("error") || trigger.contains("500") || trigger.contains("timeout") {
                // Check if user would retry or abandon based on context
                // For now, default to retry
                return Ok(Intention::Retry);
            }
            if trigger.contains("payment_failed") {
                return Ok(Intention::Abandon);
            }
            if trigger.contains("cart_empty") {
                return Ok(Intention::Browse);
            }
        }

        // Determine from action type
        match action.action_type.as_str() {
            "GET" if action.target.contains("/products") || action.target.contains("/browse") => {
                Ok(Intention::Browse)
            }
            "GET" if action.target.contains("/search") => Ok(Intention::Search),
            "POST" if action.target.contains("/cart") || action.target.contains("/add") => {
                Ok(Intention::Shop)
            }
            "POST"
                if action.target.contains("/checkout") || action.target.contains("/purchase") =>
            {
                Ok(Intention::Buy)
            }
            "navigate" => Ok(Intention::Navigate),
            "abandon" => Ok(Intention::Abandon),
            _ => Ok(Intention::Browse),
        }
    }

    /// Extract reasoning from LLM response
    fn extract_reasoning(&self, response: &Value) -> Result<String> {
        if let Some(reasoning) = response.get("reasoning").and_then(|v| v.as_str()) {
            Ok(reasoning.to_string())
        } else {
            Ok("User behavior determined based on current state and traits".to_string())
        }
    }

    /// Estimate cost in USD based on token usage
    fn estimate_cost(&self, usage: &LlmUsage) -> f64 {
        let cost_per_1k_tokens =
            match self.config.behavior_model.llm_provider.to_lowercase().as_str() {
                "openai" => match self.config.behavior_model.model.to_lowercase().as_str() {
                    model if model.contains("gpt-4") => 0.03,
                    model if model.contains("gpt-3.5") => 0.002,
                    _ => 0.002,
                },
                "anthropic" => 0.008,
                "ollama" => 0.0,
                _ => 0.002,
            };

        (usage.total_tokens as f64 / 1000.0) * cost_per_1k_tokens
    }

    /// Get agent by ID
    pub fn get_agent(&self, agent_id: &str) -> Option<&NarrativeAgent> {
        self.agents.get(agent_id)
    }

    /// List all agents
    pub fn list_agents(&self) -> Vec<&NarrativeAgent> {
        self.agents.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intelligent_behavior::config::BehaviorModelConfig;

    fn create_test_config() -> IntelligentBehaviorConfig {
        IntelligentBehaviorConfig {
            behavior_model: BehaviorModelConfig {
                llm_provider: "ollama".to_string(),
                model: "llama2".to_string(),
                api_endpoint: Some("http://localhost:11434/api/chat".to_string()),
                api_key: None,
                temperature: 0.7,
                max_tokens: 2000,
                rules: crate::intelligent_behavior::types::BehaviorRules::default(),
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_behavioral_simulator_creation() {
        let config = create_test_config();
        let simulator = BehavioralSimulator::new(config);
        assert!(simulator.use_existing_personas);
        assert!(simulator.allow_new_personas);
    }

    #[test]
    fn test_intention_determination() {
        let config = create_test_config();
        let simulator = BehavioralSimulator::new(config);

        let action = NextAction {
            action_type: "GET".to_string(),
            target: "/api/products".to_string(),
            body: None,
            query_params: None,
            delay_ms: None,
        };

        let intention = simulator.determine_intention(&action, &None).unwrap();
        assert_eq!(intention, Intention::Browse);
    }
}
