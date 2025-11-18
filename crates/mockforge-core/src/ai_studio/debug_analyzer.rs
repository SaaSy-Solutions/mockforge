//! AI-guided debugging analyzer
//!
//! This module provides functionality to analyze test failures and suggest fixes.
//! It integrates with the existing failure analysis infrastructure to provide
//! AI-powered debugging assistance.

use crate::failure_analysis::{
    context_collector::FailureContextCollector, narrative_generator::FailureNarrativeGenerator,
    types::FailureContext,
};
use crate::intelligent_behavior::llm_client::LlmClient;
use crate::intelligent_behavior::types::LlmGenerationRequest;
use crate::intelligent_behavior::IntelligentBehaviorConfig;
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Debug analyzer for test failure analysis
pub struct DebugAnalyzer {
    /// Context collector for gathering failure details
    context_collector: FailureContextCollector,
    /// Narrative generator for root cause analysis
    narrative_generator: FailureNarrativeGenerator,
    /// LLM client for generating suggestions
    llm_client: LlmClient,
}

impl DebugAnalyzer {
    /// Create a new debug analyzer with default configuration
    pub fn new() -> Self {
        let config = IntelligentBehaviorConfig::default();
        Self {
            context_collector: FailureContextCollector::new(),
            narrative_generator: FailureNarrativeGenerator::new(config.clone()),
            llm_client: LlmClient::new(config.behavior_model),
        }
    }

    /// Create a new debug analyzer with custom configuration
    pub fn with_config(config: IntelligentBehaviorConfig) -> Self {
        Self {
            context_collector: FailureContextCollector::new(),
            narrative_generator: FailureNarrativeGenerator::new(config.clone()),
            llm_client: LlmClient::new(config.behavior_model),
        }
    }

    /// Analyze a test failure and suggest fixes
    ///
    /// This method analyzes test failure logs and provides:
    /// - Root cause identification
    /// - Specific suggestions for fixing the issue
    /// - Links to related mock configurations (personas, reality settings, contracts)
    pub async fn analyze(&self, request: &DebugRequest) -> Result<DebugResponse> {
        // Parse test logs to extract failure information
        let failure_info = self.parse_test_logs(&request.test_logs)?;

        // Collect failure context
        let context = self.context_collector.collect_context(
            &failure_info.method.unwrap_or_else(|| "UNKNOWN".to_string()),
            &failure_info.path.unwrap_or_else(|| "/".to_string()),
            failure_info.status_code,
            failure_info.error_message.clone(),
        )?;

        // Generate narrative for root cause
        let narrative = self.narrative_generator.generate_narrative(&context).await?;
        let root_cause = if narrative.summary.is_empty() {
            "Unable to determine root cause from provided logs".to_string()
        } else {
            narrative.summary.clone()
        };

        // Generate AI-powered suggestions
        let suggestions = self.generate_suggestions(&context, &narrative).await?;

        // Identify related configurations
        let related_configs = self.identify_related_configs(&context);

        Ok(DebugResponse {
            root_cause,
            suggestions,
            related_configs,
            context: Some(context),
        })
    }

    /// Parse test logs to extract failure information
    fn parse_test_logs(&self, logs: &str) -> Result<ParsedFailureInfo> {
        // Simple parsing - in a real implementation, this would use more sophisticated
        // log parsing to extract HTTP methods, paths, status codes, etc.
        let mut info = ParsedFailureInfo::default();

        // Try to extract HTTP method
        for method in &["GET", "POST", "PUT", "DELETE", "PATCH"] {
            if logs.contains(method) {
                info.method = Some(method.to_string());
                break;
            }
        }

        // Try to extract status code (simple pattern matching)
        for line in logs.lines() {
            // Look for 3-digit status codes (400-599 for errors)
            for word in line.split_whitespace() {
                if let Ok(status) = word.parse::<u16>() {
                    if status >= 400 && status < 600 {
                        info.status_code = Some(status);
                        break;
                    }
                }
            }
            if info.status_code.is_some() {
                break;
            }
        }

        // Try to extract path (simple pattern matching)
        for line in logs.lines() {
            for method in &["GET", "POST", "PUT", "DELETE", "PATCH"] {
                if let Some(pos) = line.find(method) {
                    let after_method = &line[pos + method.len()..];
                    if let Some(path_start) = after_method.find('/') {
                        let path_part = &after_method[path_start..];
                        if let Some(path_end) =
                            path_part.find(|c: char| c.is_whitespace() || c == '?' || c == '\n')
                        {
                            info.path = Some(path_part[..path_end].to_string());
                        } else {
                            info.path = Some(path_part.trim().to_string());
                        }
                        break;
                    }
                }
            }
            if info.path.is_some() {
                break;
            }
        }

        // Extract error message (look for common error patterns)
        if logs.contains("error") || logs.contains("Error") || logs.contains("ERROR") {
            info.error_message = Some(
                logs.lines()
                    .find(|line| {
                        line.to_lowercase().contains("error")
                            || line.to_lowercase().contains("fail")
                    })
                    .unwrap_or("Test failure detected")
                    .to_string(),
            );
        }

        Ok(info)
    }

    /// Generate AI-powered suggestions for fixing the failure
    async fn generate_suggestions(
        &self,
        context: &FailureContext,
        narrative: &crate::failure_analysis::types::FailureNarrative,
    ) -> Result<Vec<DebugSuggestion>> {
        // Build prompt for suggestion generation
        let system_prompt = r#"You are an expert at debugging API test failures in mock environments.
Analyze the failure context and provide specific, actionable suggestions for fixing the issue.

For each suggestion, provide:
1. A clear title
2. A detailed description of what to do
3. A specific action to take
4. The configuration path to update (if applicable)

Focus on:
- Contract validation issues (suggest tightening validation or updating contracts)
- Persona mismatches (suggest adjusting persona traits or reality settings)
- Mock scenario issues (suggest adding explicit error examples)
- Reality continuum settings (suggest adjusting reality ratios)
- Chaos configuration issues (suggest disabling or adjusting chaos rules)

Return your response as a JSON array of suggestions."#;

        let user_prompt = format!(
            r#"Failure Context:
- Request: {} {}
- Status Code: {:?}
- Error: {:?}
- Active Chaos Configs: {}
- Active Consistency Rules: {}
- Contract Validation: {:?}
- Behavioral Rules: {}

Narrative Summary: {}

Provide 3-5 specific suggestions for fixing this test failure."#,
            context.request.method,
            context.request.path,
            context.response.as_ref().map(|r| r.status_code),
            context.error_message,
            context.chaos_configs.len(),
            context.consistency_rules.len(),
            context.contract_validation.is_some(),
            context.behavioral_rules.len(),
            if narrative.summary.is_empty() {
                "No narrative available"
            } else {
                &narrative.summary
            }
        );

        let llm_request = LlmGenerationRequest {
            system_prompt: system_prompt.to_string(),
            user_prompt,
            temperature: 0.3,
            max_tokens: 1500,
            schema: None,
        };

        // Generate suggestions from LLM
        let response = self.llm_client.generate(&llm_request).await?;

        // Parse suggestions from response
        let suggestions: Vec<DebugSuggestion> = if let Some(suggestions_array) =
            response.get("suggestions")
        {
            serde_json::from_value(suggestions_array.clone()).unwrap_or_else(|_| {
                // Fallback: create a generic suggestion
                vec![DebugSuggestion {
                    title: "Review Mock Configuration".to_string(),
                    description: "Check your mock configuration for issues related to this failure"
                        .to_string(),
                    action: "Review config.yaml and related mock settings".to_string(),
                    config_path: Some("config.yaml".to_string()),
                }]
            })
        } else {
            // Fallback suggestions
            vec![
                DebugSuggestion {
                    title: "Check Contract Validation".to_string(),
                    description: "The failure may be due to contract validation issues. Review your OpenAPI spec and request/response schemas.".to_string(),
                    action: "Review contract validation settings".to_string(),
                    config_path: Some("contract_validation".to_string()),
                },
                DebugSuggestion {
                    title: "Review Persona Settings".to_string(),
                    description: "The failure might be related to persona configuration. Check if the active persona matches your test expectations.".to_string(),
                    action: "Review persona configuration".to_string(),
                    config_path: Some("consistency.personas".to_string()),
                },
            ]
        };

        Ok(suggestions)
    }

    /// Identify related mock configurations
    fn identify_related_configs(&self, context: &FailureContext) -> Vec<String> {
        let mut configs = Vec::new();

        // Add contract validation config if present
        if context.contract_validation.is_some() {
            configs.push("Contract Validation".to_string());
        }

        // Add persona configs if behavioral rules are present
        if !context.behavioral_rules.is_empty() {
            configs.push("Persona Configuration".to_string());
        }

        // Add chaos configs if present
        if !context.chaos_configs.is_empty() {
            configs.push("Chaos Configuration".to_string());
        }

        // Add consistency rules if present
        if !context.consistency_rules.is_empty() {
            configs.push("Consistency Rules".to_string());
        }

        // Add reality continuum if no specific configs found
        if configs.is_empty() {
            configs.push("Reality Continuum Settings".to_string());
        }

        configs
    }
}

impl Default for DebugAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Parsed failure information from test logs
#[derive(Debug, Default)]
struct ParsedFailureInfo {
    method: Option<String>,
    path: Option<String>,
    status_code: Option<u16>,
    error_message: Option<String>,
}

/// Request for debug analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugRequest {
    /// Test failure logs
    pub test_logs: String,

    /// Test name/identifier
    pub test_name: Option<String>,

    /// Workspace ID for context
    pub workspace_id: Option<String>,
}

/// Response from debug analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugResponse {
    /// Identified root cause
    pub root_cause: String,

    /// Suggested fixes
    pub suggestions: Vec<DebugSuggestion>,

    /// Related mock configurations
    pub related_configs: Vec<String>,

    /// Full failure context (optional, for detailed analysis)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<FailureContext>,
}

/// Debug suggestion for fixing a test failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugSuggestion {
    /// Suggestion title
    pub title: String,

    /// Detailed description
    pub description: String,

    /// Suggested action
    pub action: String,

    /// Configuration path to update
    pub config_path: Option<String>,
}
