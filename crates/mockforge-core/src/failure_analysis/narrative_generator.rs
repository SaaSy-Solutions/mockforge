//! Failure narrative generator
//!
//! Uses LLM to generate human-readable narratives explaining why
//! request failures occurred, with stack traces showing the chain
//! of events.

use crate::intelligent_behavior::{
    config::IntelligentBehaviorConfig, llm_client::LlmClient, types::LlmGenerationRequest,
};
use crate::Result;

use super::types::*;

/// Generator for failure narratives
pub struct FailureNarrativeGenerator {
    /// LLM client for generating narratives
    llm_client: LlmClient,
    /// Configuration
    config: IntelligentBehaviorConfig,
}

impl FailureNarrativeGenerator {
    /// Create a new failure narrative generator
    pub fn new(config: IntelligentBehaviorConfig) -> Self {
        let behavior_model = config.behavior_model.clone();
        let llm_client = LlmClient::new(behavior_model);

        Self { llm_client, config }
    }

    /// Generate a narrative explaining a failure
    pub async fn generate_narrative(&self, context: &FailureContext) -> Result<FailureNarrative> {
        // Build system prompt for narrative generation
        let system_prompt = r#"You are an expert at analyzing system failures and explaining them
in clear, human-readable narratives. Your task is to analyze failure context and generate
a comprehensive explanation of why a request failed.

Generate a narrative that includes:
1. A concise summary of what failed
2. A detailed explanation of why it failed
3. A stack trace showing the chain of events (which rules/personas/contracts triggered)
4. Contributing factors (what made the failure more likely)
5. Suggested fixes

Focus on identifying which specific rules, personas, contracts, or chaos configurations
caused or contributed to the failure. Be specific about conditions that were met.

Return your response as a JSON object with this structure:
{
  "summary": "Brief one-sentence summary of the failure",
  "explanation": "Detailed explanation of why the failure occurred",
  "stack_trace": [
    {
      "description": "What happened in this frame",
      "trigger": "What condition or event triggered this",
      "source": "Name of the rule/persona/contract/chaos config",
      "source_type": "rule|persona|contract|chaos|hook|other"
    }
  ],
  "contributing_factors": [
    {
      "description": "Description of the contributing factor",
      "factor_type": "Type of factor (e.g., chaos_config, consistency_rule, etc.)",
      "impact": "high|medium|low"
    }
  ],
  "suggested_fixes": [
    "List of suggested fixes or improvements"
  ],
  "confidence": 0.0-1.0
}

Be thorough but concise. Focus on actionable insights."#;

        // Build context summary for the LLM
        let context_summary = self.build_context_summary(context);

        // Build user prompt
        let user_prompt = format!(
            "Analyze this failure context and generate a narrative:\n\n{}",
            context_summary
        );

        // Create LLM request
        let llm_request = LlmGenerationRequest {
            system_prompt: system_prompt.to_string(),
            user_prompt,
            temperature: 0.3, // Lower temperature for more consistent analysis
            max_tokens: 2000,
            schema: None,
        };

        // Generate response from LLM
        let response = self.llm_client.generate(&llm_request).await?;

        // Parse the response into FailureNarrative
        let response_str = serde_json::to_string(&response).unwrap_or_default();
        let narrative: FailureNarrative = serde_json::from_value(response).map_err(|e| {
            crate::Error::generic(format!(
                "Failed to parse LLM response as FailureNarrative: {}. Response: {}",
                e, response_str
            ))
        })?;

        Ok(narrative)
    }

    /// Build a human-readable summary of the failure context
    fn build_context_summary(&self, context: &FailureContext) -> String {
        let mut summary = String::new();

        // Request details
        summary.push_str("## Request Details\n");
        summary.push_str(&format!("Method: {}\n", context.request.method));
        summary.push_str(&format!("Path: {}\n", context.request.path));
        if !context.request.headers.is_empty() {
            summary.push_str(&format!("Headers: {:?}\n", context.request.headers));
        }
        if !context.request.query_params.is_empty() {
            summary.push_str(&format!("Query Params: {:?}\n", context.request.query_params));
        }
        if let Some(ref body) = context.request.body {
            summary.push_str(&format!("Body: {}\n", body));
        }
        summary.push('\n');

        // Response details
        if let Some(ref response) = context.response {
            summary.push_str("## Response Details\n");
            summary.push_str(&format!("Status Code: {}\n", response.status_code));
            if let Some(duration) = response.duration_ms {
                summary.push_str(&format!("Duration: {}ms\n", duration));
            }
            if let Some(ref body) = response.body {
                summary.push_str(&format!("Response Body: {}\n", body));
            }
            summary.push('\n');
        }

        // Error message
        if let Some(ref error) = context.error_message {
            summary.push_str("## Error\n");
            summary.push_str(&format!("{}\n", error));
            summary.push('\n');
        }

        // Chaos configs
        if !context.chaos_configs.is_empty() {
            summary.push_str("## Active Chaos Configurations\n");
            for config in &context.chaos_configs {
                summary.push_str(&format!("- {}: enabled={}\n", config.name, config.enabled));
            }
            summary.push('\n');
        }

        // Consistency rules
        if !context.consistency_rules.is_empty() {
            summary.push_str("## Consistency Rules\n");
            for rule in &context.consistency_rules {
                summary.push_str(&format!(
                    "- {}: enabled={}, triggered={}\n",
                    rule.name, rule.enabled, rule.triggered
                ));
                if let Some(ref desc) = rule.description {
                    summary.push_str(&format!("  Description: {}\n", desc));
                }
            }
            summary.push('\n');
        }

        // Contract validation
        if let Some(ref validation) = context.contract_validation {
            summary.push_str("## Contract Validation\n");
            summary.push_str(&format!("Passed: {}\n", validation.passed));
            if !validation.errors.is_empty() {
                summary.push_str("Errors:\n");
                for error in &validation.errors {
                    summary.push_str(&format!("  - {}\n", error));
                }
            }
            summary.push('\n');
        }

        // Behavioral rules
        if !context.behavioral_rules.is_empty() {
            summary.push_str("## Behavioral Rules/Personas\n");
            for rule in &context.behavioral_rules {
                summary.push_str(&format!("- {}: active={}\n", rule.name, rule.active));
                if let Some(ref desc) = rule.description {
                    summary.push_str(&format!("  Description: {}\n", desc));
                }
            }
            summary.push('\n');
        }

        // Hook results
        if !context.hook_results.is_empty() {
            summary.push_str("## Hook Execution Results\n");
            for hook in &context.hook_results {
                summary.push_str(&format!(
                    "- {}: success={}, type={}\n",
                    hook.name, hook.success, hook.hook_type
                ));
                if let Some(ref error) = hook.error {
                    summary.push_str(&format!("  Error: {}\n", error));
                }
            }
            summary.push('\n');
        }

        summary
    }
}
