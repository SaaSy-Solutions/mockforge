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
use mockforge_chaos::advanced_orchestration::{Condition, Hook, HookAction, HookType, LogLevel};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

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

        // Parse the response into ParsedHook
        let response_str = serde_json::to_string(&response).unwrap_or_default();
        let parsed: ParsedHook = serde_json::from_value(response).map_err(|e| {
            crate::Error::generic(format!(
                "Failed to parse LLM response as ParsedHook: {}. Response: {}",
                e, response_str
            ))
        })?;

        // Convert ParsedHook to Hook
        self.convert_to_hook(parsed)
    }

    /// Convert a ParsedHook to a Hook struct
    fn convert_to_hook(&self, parsed: ParsedHook) -> Result<Hook> {
        // Convert hook type
        let hook_type = match parsed.hook_type.as_str() {
            "pre_step" => HookType::PreStep,
            "post_step" => HookType::PostStep,
            "pre_orchestration" => HookType::PreOrchestration,
            "post_orchestration" => HookType::PostOrchestration,
            _ => {
                return Err(crate::Error::generic(format!(
                    "Invalid hook type: {}",
                    parsed.hook_type
                )));
            }
        };

        // Convert condition
        let condition = if let Some(cond) = parsed.condition {
            Some(self.convert_condition(cond)?)
        } else {
            None
        };

        // Convert actions
        let mut actions = Vec::new();
        for action in parsed.actions {
            actions.push(self.convert_action(action)?);
        }

        Ok(Hook {
            name: parsed.name,
            hook_type,
            actions,
            condition,
        })
    }

    /// Convert a ParsedCondition to a Condition enum
    fn convert_condition(&self, parsed: ParsedCondition) -> Result<Condition> {
        match parsed.r#type.as_str() {
            "equals" => {
                let variable = parsed
                    .variable
                    .ok_or_else(|| crate::Error::generic("Missing variable in equals condition"))?;
                let value = parsed
                    .value
                    .ok_or_else(|| crate::Error::generic("Missing value in equals condition"))?;
                Ok(Condition::Equals { variable, value })
            }
            "not_equals" => {
                let variable = parsed.variable.ok_or_else(|| {
                    crate::Error::generic("Missing variable in not_equals condition")
                })?;
                let value = parsed.value.ok_or_else(|| {
                    crate::Error::generic("Missing value in not_equals condition")
                })?;
                Ok(Condition::NotEquals { variable, value })
            }
            "greater_than" => {
                let variable = parsed.variable.ok_or_else(|| {
                    crate::Error::generic("Missing variable in greater_than condition")
                })?;
                let value = parsed
                    .numeric_value
                    .ok_or_else(|| crate::Error::generic("Missing numeric_value in greater_than condition"))?;
                Ok(Condition::GreaterThan { variable, value })
            }
            "less_than" => {
                let variable = parsed.variable.ok_or_else(|| {
                    crate::Error::generic("Missing variable in less_than condition")
                })?;
                let value = parsed
                    .numeric_value
                    .ok_or_else(|| crate::Error::generic("Missing numeric_value in less_than condition"))?;
                Ok(Condition::LessThan { variable, value })
            }
            "exists" => {
                let variable = parsed
                    .variable
                    .ok_or_else(|| crate::Error::generic("Missing variable in exists condition"))?;
                Ok(Condition::Exists { variable })
            }
            "and" => {
                let conditions = parsed
                    .conditions
                    .ok_or_else(|| crate::Error::generic("Missing conditions in and condition"))?;
                let mut converted = Vec::new();
                for cond in conditions {
                    converted.push(self.convert_condition(cond)?);
                }
                Ok(Condition::And {
                    conditions: converted,
                })
            }
            "or" => {
                let conditions = parsed
                    .conditions
                    .ok_or_else(|| crate::Error::generic("Missing conditions in or condition"))?;
                let mut converted = Vec::new();
                for cond in conditions {
                    converted.push(self.convert_condition(cond)?);
                }
                Ok(Condition::Or {
                    conditions: converted,
                })
            }
            "not" => {
                let condition = parsed
                    .condition
                    .ok_or_else(|| crate::Error::generic("Missing condition in not condition"))?;
                Ok(Condition::Not {
                    condition: Box::new(self.convert_condition(*condition)?),
                })
            }
            _ => Err(crate::Error::generic(format!(
                "Unknown condition type: {}",
                parsed.r#type
            ))),
        }
    }

    /// Convert a ParsedAction to a HookAction enum
    fn convert_action(&self, parsed: ParsedAction) -> Result<HookAction> {
        match parsed.r#type.as_str() {
            "set_variable" => {
                let name = parsed
                    .name
                    .ok_or_else(|| crate::Error::generic("Missing name in set_variable action"))?;
                let value = parsed
                    .value
                    .ok_or_else(|| crate::Error::generic("Missing value in set_variable action"))?;
                Ok(HookAction::SetVariable { name, value })
            }
            "log" => {
                let message = parsed
                    .message
                    .ok_or_else(|| crate::Error::generic("Missing message in log action"))?;
                let level = parsed
                    .level
                    .as_deref()
                    .unwrap_or("info")
                    .to_lowercase();
                let log_level = match level.as_str() {
                    "trace" => LogLevel::Trace,
                    "debug" => LogLevel::Debug,
                    "info" => LogLevel::Info,
                    "warn" => LogLevel::Warn,
                    "error" => LogLevel::Error,
                    _ => LogLevel::Info,
                };
                Ok(HookAction::Log { message, level: log_level })
            }
            "http_request" => {
                let url = parsed
                    .url
                    .ok_or_else(|| crate::Error::generic("Missing url in http_request action"))?;
                let method = parsed
                    .method
                    .as_deref()
                    .unwrap_or("POST")
                    .to_uppercase();
                let body = parsed.body.map(|b| {
                    // If body is already a string, use it; otherwise serialize to string
                    if let JsonValue::String(s) = b {
                        s
                    } else {
                        serde_json::to_string(&b).unwrap_or_default()
                    }
                });
                Ok(HookAction::HttpRequest {
                    url,
                    method,
                    body,
                })
            }
            "command" => {
                let command = parsed
                    .command
                    .ok_or_else(|| crate::Error::generic("Missing command in command action"))?;
                let args = parsed.args.unwrap_or_default();
                Ok(HookAction::Command { command, args })
            }
            "record_metric" => {
                let name = parsed
                    .name
                    .ok_or_else(|| crate::Error::generic("Missing name in record_metric action"))?;
                let value = parsed
                    .numeric_value
                    .ok_or_else(|| crate::Error::generic("Missing numeric_value in record_metric action"))?;
                Ok(HookAction::RecordMetric { name, value })
            }
            _ => Err(crate::Error::generic(format!(
                "Unknown action type: {}",
                parsed.r#type
            ))),
        }
    }
}

/// Parsed hook structure from LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParsedHook {
    /// Hook name
    name: String,
    /// Hook type
    hook_type: String,
    /// Optional condition
    condition: Option<ParsedCondition>,
    /// List of actions
    actions: Vec<ParsedAction>,
}

/// Parsed condition structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParsedCondition {
    /// Condition type
    r#type: String,
    /// Variable name (for most conditions)
    variable: Option<String>,
    /// Value (for equals/not_equals)
    value: Option<JsonValue>,
    /// Numeric value (for greater_than/less_than)
    numeric_value: Option<f64>,
    /// Nested conditions (for and/or)
    conditions: Option<Vec<ParsedCondition>>,
    /// Nested condition (for not)
    condition: Option<Box<ParsedCondition>>,
}

/// Parsed action structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParsedAction {
    /// Action type
    r#type: String,
    /// Name (for set_variable, record_metric)
    name: Option<String>,
    /// Value (for set_variable)
    value: Option<JsonValue>,
    /// Message (for log)
    message: Option<String>,
    /// Log level (for log)
    level: Option<String>,
    /// URL (for http_request)
    url: Option<String>,
    /// HTTP method (for http_request)
    method: Option<String>,
    /// Request body (for http_request)
    body: Option<JsonValue>,
    /// Command (for command)
    command: Option<String>,
    /// Command arguments (for command)
    args: Option<Vec<String>>,
    /// Numeric value (for record_metric)
    numeric_value: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsed_condition_conversion() {
        let config = IntelligentBehaviorConfig::default();
        let transpiler = HookTranspiler::new(config);

        // Test equals condition
        let parsed = ParsedCondition {
            r#type: "equals".to_string(),
            variable: Some("user.vip".to_string()),
            value: Some(JsonValue::Bool(true)),
            numeric_value: None,
            conditions: None,
            condition: None,
        };

        let result = transpiler.convert_condition(parsed);
        assert!(result.is_ok());
    }
}

