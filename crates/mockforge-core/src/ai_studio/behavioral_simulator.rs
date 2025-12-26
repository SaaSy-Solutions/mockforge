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
//! ```rust,no_run
//! use mockforge_core::ai_studio::behavioral_simulator::{BehavioralSimulator, CreateAgentRequest};
//! use mockforge_core::intelligent_behavior::IntelligentBehaviorConfig;
//!
//! # async fn example() -> mockforge_core::Result<()> {
//! let config = IntelligentBehaviorConfig::default();
//! let simulator = BehavioralSimulator::new(config);
//!
//! let request = CreateAgentRequest {
//!     persona_id: Some("existing-persona-123".to_string()),
//!     behavior_policy: Some("bargain-hunter".to_string()),
//!     generate_persona: false,
//! };
//!
//! let agent = simulator.create_agent(&request).await?;
//! # Ok(())
//! # }
//! ```

use crate::intelligent_behavior::{
    config::IntelligentBehaviorConfig,
    llm_client::{LlmClient, LlmUsage},
    types::LlmGenerationRequest,
};
use crate::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

/// Request to create a narrative agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentRequest {
    /// Optional: Attach to existing persona ID
    pub persona_id: Option<String>,

    /// Optional: Behavior policy type (e.g., "bargain-hunter", "power-user", "churn-risk")
    pub behavior_policy: Option<String>,

    /// If true, generate new persona if persona_id is not provided or doesn't exist
    pub generate_persona: bool,

    /// Optional workspace ID
    pub workspace_id: Option<String>,
}

/// Narrative agent that models user behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeAgent {
    /// Agent ID
    pub agent_id: String,

    /// Persona ID (links to existing persona or new generated one)
    pub persona_id: String,

    /// Current intention
    pub current_intention: Intention,

    /// Session history of interactions
    pub session_history: Vec<Interaction>,

    /// Behavioral traits
    pub behavioral_traits: BehavioralTraits,

    /// Current app state awareness
    pub state_awareness: AppState,

    /// Behavior policy attached to persona
    pub behavior_policy: BehaviorPolicy,

    /// Created at timestamp
    pub created_at: String,
}

/// User intention types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Intention {
    /// Explore products/content
    Browse,

    /// Actively looking to purchase
    Shop,

    /// Ready to complete purchase
    Buy,

    /// Leave due to frustration/error
    Abandon,

    /// Retry after error
    Retry,

    /// Move to different section
    Navigate,

    /// Search for something
    Search,

    /// Compare options
    Compare,

    /// Review/read content
    Review,
}

/// Behavioral traits for the agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralTraits {
    /// Patience level (0.0-1.0, higher = more patient)
    pub patience: f64,

    /// Price sensitivity (0.0-1.0, higher = more price-sensitive)
    pub price_sensitivity: f64,

    /// Risk tolerance (0.0-1.0, higher = more risk-tolerant)
    pub risk_tolerance: f64,

    /// Technical proficiency (0.0-1.0, higher = more technical)
    pub technical_proficiency: f64,

    /// Engagement level (0.0-1.0, higher = more engaged)
    pub engagement_level: f64,
}

/// Behavior policy attached to persona
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorPolicy {
    /// Policy type (e.g., "bargain-hunter", "power-user", "churn-risk")
    pub policy_type: String,

    /// Policy description
    pub description: String,

    /// Policy rules/behaviors
    pub rules: Vec<PolicyRule>,
}

/// Policy rule for behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Condition that triggers this rule
    pub condition: String,

    /// Action to take
    pub action: String,

    /// Priority (higher = more important)
    pub priority: i32,
}

/// App state awareness
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppState {
    /// Current page/section
    pub current_page: Option<String>,

    /// Cart state
    pub cart: CartState,

    /// Authentication state
    pub authenticated: bool,

    /// Recent errors encountered
    pub recent_errors: Vec<ErrorEncounter>,

    /// Current context
    pub context: HashMap<String, Value>,
}

/// Cart state
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CartState {
    /// Whether cart is empty
    pub is_empty: bool,

    /// Number of items
    pub item_count: usize,

    /// Total value
    pub total_value: f64,

    /// Items in cart
    pub items: Vec<CartItem>,
}

/// Cart item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CartItem {
    /// Item ID
    pub item_id: String,

    /// Item name
    pub name: String,

    /// Price
    pub price: f64,

    /// Quantity
    pub quantity: usize,
}

/// Error encounter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEncounter {
    /// Error type (e.g., "500", "timeout", "validation_error")
    pub error_type: String,

    /// Error message
    pub message: String,

    /// Timestamp
    pub timestamp: String,

    /// Number of times encountered
    pub count: usize,
}

/// Interaction record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Interaction {
    /// Interaction timestamp
    pub timestamp: String,

    /// Action taken
    pub action: String,

    /// Intention at time of action
    pub intention: Intention,

    /// Request details
    pub request: Option<Value>,

    /// Response details
    pub response: Option<Value>,

    /// Result (success, error, etc.)
    pub result: String,
}

/// Request to simulate behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateBehaviorRequest {
    /// Optional: Use existing agent ID
    pub agent_id: Option<String>,

    /// Optional: Attach to existing persona
    pub persona_id: Option<String>,

    /// Current app state
    pub current_state: AppState,

    /// Trigger event (e.g., "error_500", "cart_empty", "payment_failed")
    pub trigger_event: Option<String>,

    /// Optional workspace ID
    pub workspace_id: Option<String>,
}

/// Response from behavior simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulateBehaviorResponse {
    /// Next action to take
    pub next_action: NextAction,

    /// New intention
    pub intention: Intention,

    /// Reasoning for the action
    pub reasoning: String,

    /// Updated agent state
    pub agent: Option<NarrativeAgent>,

    /// Token usage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<u64>,

    /// Estimated cost
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
}

/// Next action to take
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextAction {
    /// Action type (e.g., "GET", "POST", "navigate", "abandon")
    pub action_type: String,

    /// Target endpoint or page
    pub target: String,

    /// Optional request body
    pub body: Option<Value>,

    /// Optional query parameters
    pub query_params: Option<HashMap<String, String>>,

    /// Delay before action (ms)
    pub delay_ms: Option<u64>,
}

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
                return Err(crate::Error::generic(
                    "Using existing personas is disabled".to_string(),
                ));
            }
        } else if request.generate_persona {
            // Generate new persona if allowed
            if !self.allow_new_personas {
                return Err(crate::Error::generic(
                    "Generating new personas is disabled".to_string(),
                ));
            }

            // Check limit
            let new_persona_count =
                self.agents.values().filter(|a| !a.persona_id.starts_with("existing-")).count();

            if new_persona_count >= self.max_new_personas {
                return Err(crate::Error::generic(format!(
                    "Maximum new personas limit ({}) reached",
                    self.max_new_personas
                )));
            }

            // Generate new persona ID (in production, would call persona generator)
            format!("persona-{}", Uuid::new_v4())
        } else {
            return Err(crate::Error::generic(
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
                .ok_or_else(|| crate::Error::generic("Agent not found".to_string()))?
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
            return Err(crate::Error::generic(
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
            .map_err(|e| crate::Error::generic(format!("Failed to serialize state: {}", e)))?;

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
            return Err(crate::Error::generic(
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
