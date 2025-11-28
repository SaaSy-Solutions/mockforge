//! Natural language to hook transpiler
//!
//! This module converts natural language descriptions of hook logic into
//! structured Hook definitions that can be used in chaos orchestration scenarios.
//!
//! # Example
//!
//! ```rust,no_run
//! use mockforge_core::voice::HookTranspiler;
//! use mockforge_core::intelligent_behavior::IntelligentBehaviorConfig;
//!
//! # async fn example() -> mockforge_core::Result<()> {
//! let config = IntelligentBehaviorConfig::default();
//! let transpiler = HookTranspiler::new(config);
//!
//! let description = "For users flagged as VIP, webhooks should fire instantly but payments fail 5% of the time";
//! let hook = transpiler.transpile(description).await?;
//! # Ok(())
//! # }
//! ```

use crate::intelligent_behavior::{
    config::IntelligentBehaviorConfig, llm_client::LlmClient, types::LlmGenerationRequest,
};
use crate::Result;
// Hook types are defined in mockforge-chaos, but we use serde_json::Value to avoid circular dependency
// When used, they should be deserialized from JSON
type Hook = serde_json::Value;
type Condition = serde_json::Value;
type HookAction = serde_json::Value;
type HookType = serde_json::Value;
type LogLevel = serde_json::Value;

/// Transpiler that converts natural language hook descriptions to Hook structs
pub struct HookTranspiler {
    /// LLM client for parsing descriptions
    llm_client: LlmClient,
    /// Configuration
    config: IntelligentBehaviorConfig,
}

impl HookTranspiler {
    /// Create a new hook transpiler
    pub fn new(config: IntelligentBehaviorConfig) -> Self {
        let behavior_model = config.behavior_model.clone();
        let llm_client = LlmClient::new(behavior_model);

        Self { llm_client, config }
    }

    /// Transpile a natural language hook description to a Hook struct
    ///
    /// # Arguments
    ///
    /// * `description` - Natural language description of the hook logic
    ///
    /// # Example
    ///
    /// ```
    /// "For users flagged as VIP, webhooks should fire instantly but payments fail 5% of the time"
    /// ```
    pub async fn transpile(&self, description: &str) -> Result<Hook> {
        // Build system prompt for hook parsing
        let system_prompt = r#"You are an expert at parsing natural language descriptions of hook logic
and converting them to structured hook configurations.

A hook consists of:
1. **Name**: A descriptive name for the hook
2. **Hook Type**: When the hook executes (pre_step, post_step, pre_orchestration, post_orchestration)
3. **Condition**: Optional condition that must be met for the hook to execute
4. **Actions**: List of actions to perform when the hook executes

Available hook types:
- pre_step: Execute before a step
- post_step: Execute after a step
- pre_orchestration: Execute before orchestration starts
- post_orchestration: Execute after orchestration completes

Available conditions:
- equals: Variable equals value
- not_equals: Variable not equals value
- greater_than: Variable greater than numeric value
- less_than: Variable less than numeric value
- exists: Variable exists
- and: All conditions must be true
- or: At least one condition must be true
- not: Condition must be false

Available actions:
- set_variable: Set a variable to a value
- log: Log a message at a level (trace, debug, info, warn, error)
- http_request: Make an HTTP request (webhook)
- command: Execute a command
- record_metric: Record a metric value

For probability-based failures (e.g., "fail 5% of the time"), you should:
1. Use a condition that checks a random variable or metric
2. Use set_variable to set a failure flag
3. The actual failure injection should be handled by the chaos configuration

For timing constraints (e.g., "instantly", "with delay"), use appropriate hook types or add delay actions.

Return your response as a JSON object with this structure:
{
  "name": "string (descriptive hook name)",
  "hook_type": "pre_step | post_step | pre_orchestration | post_orchestration",
  "condition": {
    "type": "condition type",
    ...condition-specific fields
  } or null,
  "actions": [
    {
      "type": "action type",
      ...action-specific fields
    }
  ]
}

Be specific and extract all details from the description. If timing is mentioned (instantly, with delay),
choose the appropriate hook_type. If conditions are mentioned (for users flagged as VIP), create
appropriate condition structures."#;

        // Build user prompt with the description
        let user_prompt = format!(
            "Parse this hook description and convert it to a hook configuration:\n\n{}",
            description
        );

        // Create LLM request
        let llm_request = LlmGenerationRequest {
            system_prompt: system_prompt.to_string(),
            user_prompt,
            temperature: 0.2, // Lower temperature for more consistent parsing
            max_tokens: 2000,
            schema: None,
        };

        // Generate response from LLM
        let response = self.llm_client.generate(&llm_request).await?;

        // Since Hook is now serde_json::Value, we can return the response directly
        // Just validate it's a valid JSON object
        if !response.is_object() {
            return Err(crate::Error::generic(format!(
                "LLM response is not a JSON object. Response: {}",
                serde_json::to_string(&response).unwrap_or_default()
            )));
        }

        Ok(response)
    }

    // Note: convert_to_hook and related functions removed since Hook is now serde_json::Value
    // The LLM response is returned directly as JSON
}
