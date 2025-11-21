//! AI-guided debugging analyzer
//!
//! This module provides functionality to analyze test failures and suggest fixes.
//! It integrates with the existing failure analysis infrastructure to provide
//! AI-powered debugging assistance.

use crate::ai_studio::debug_context::DebugContext as UnifiedDebugContext;
use crate::ai_studio::debug_context_integrator::DebugContextIntegrator;
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
    /// Optional debug context integrator for collecting subsystem context
    context_integrator: Option<DebugContextIntegrator>,
}

impl DebugAnalyzer {
    /// Create a new debug analyzer with default configuration
    pub fn new() -> Self {
        let config = IntelligentBehaviorConfig::default();
        Self {
            context_collector: FailureContextCollector::new(),
            narrative_generator: FailureNarrativeGenerator::new(config.clone()),
            llm_client: LlmClient::new(config.behavior_model),
            context_integrator: None,
        }
    }

    /// Create a new debug analyzer with custom configuration
    pub fn with_config(config: IntelligentBehaviorConfig) -> Self {
        Self {
            context_collector: FailureContextCollector::new(),
            narrative_generator: FailureNarrativeGenerator::new(config.clone()),
            llm_client: LlmClient::new(config.behavior_model),
            context_integrator: None,
        }
    }

    /// Create a new debug analyzer with context integrator
    pub fn with_integrator(integrator: DebugContextIntegrator) -> Self {
        let config = IntelligentBehaviorConfig::default();
        Self {
            context_collector: FailureContextCollector::new(),
            narrative_generator: FailureNarrativeGenerator::new(config.clone()),
            llm_client: LlmClient::new(config.behavior_model),
            context_integrator: Some(integrator),
        }
    }

    /// Create a new debug analyzer with config and integrator
    pub fn with_config_and_integrator(
        config: IntelligentBehaviorConfig,
        integrator: DebugContextIntegrator,
    ) -> Self {
        Self {
            context_collector: FailureContextCollector::new(),
            narrative_generator: FailureNarrativeGenerator::new(config.clone()),
            llm_client: LlmClient::new(config.behavior_model),
            context_integrator: Some(integrator),
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

        // Collect unified debug context from subsystems (if integrator is available)
        let unified_context = if let Some(ref integrator) = self.context_integrator {
            Some(integrator.collect_unified_context(request.workspace_id.as_deref()).await?)
        } else {
            None
        };

        // Generate narrative for root cause
        let narrative = self.narrative_generator.generate_narrative(&context).await?;
        let root_cause = if narrative.summary.is_empty() {
            "Unable to determine root cause from provided logs".to_string()
        } else {
            narrative.summary.clone()
        };

        // Generate AI-powered suggestions with unified context
        let mut suggestions = self
            .generate_suggestions(&context, &narrative, unified_context.as_ref())
            .await?;

        // Generate patch operations for suggestions
        self.generate_patches(&mut suggestions, &context, &narrative, unified_context.as_ref())?;

        // Identify related configurations with unified context
        let related_configs = self.identify_related_configs(&context, unified_context.as_ref());

        Ok(DebugResponse {
            root_cause,
            suggestions,
            related_configs,
            context: Some(context),
            unified_context,
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
        unified_context: Option<&UnifiedDebugContext>,
    ) -> Result<Vec<DebugSuggestion>> {
        // Build prompt for suggestion generation
        let system_prompt = r#"You are an expert at debugging API test failures in mock environments.
Analyze the failure context and provide specific, actionable suggestions for fixing the issue.

For each suggestion, provide:
1. A clear title
2. A detailed description of what to do
3. A specific action to take
4. The configuration path to update (if applicable)
5. Linked artifacts (persona IDs, scenario names, contract paths) that are relevant

Focus on:
- Contract validation issues (suggest tightening validation or updating contracts)
- Persona mismatches (suggest adjusting persona traits or reality settings)
- Mock scenario issues (suggest adding explicit error examples)
- Reality continuum settings (suggest adjusting reality ratios)
- Chaos configuration issues (suggest disabling or adjusting chaos rules)

Return your response as a JSON array of suggestions."#;

        // Build unified context summary
        let unified_summary = if let Some(uc) = unified_context {
            format!(
                r#"
Unified Subsystem Context:
- Reality Level: {} (chaos: {}, latency: {}ms, MockAI: {})
- Contract Validation: {} (enforcement: {})
- Active Scenario: {}
- Active Persona: {}
- Chaos Rules: {} active
"#,
                uc.reality
                    .level_name
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("unknown"),
                uc.reality.chaos_enabled,
                uc.reality.latency_base_ms,
                uc.reality.mockai_enabled,
                uc.contract.validation_enabled,
                uc.contract.enforcement_mode,
                uc.scenario
                    .active_scenario
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("none"),
                uc.persona
                    .active_persona_id
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or("none"),
                uc.chaos.active_rules.len()
            )
        } else {
            String::new()
        };

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
{}

Provide 3-5 specific suggestions for fixing this test failure. Include linked artifacts (persona IDs, scenario names, contract paths) in your suggestions."#,
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
            },
            unified_summary
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
        let mut suggestions: Vec<DebugSuggestion> = if let Some(suggestions_array) =
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
                    patch: None,
                    linked_artifacts: Vec::new(),
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
                    patch: None,
                    linked_artifacts: Vec::new(),
                },
                DebugSuggestion {
                    title: "Review Persona Settings".to_string(),
                    description: "The failure might be related to persona configuration. Check if the active persona matches your test expectations.".to_string(),
                    action: "Review persona configuration".to_string(),
                    config_path: Some("consistency.personas".to_string()),
                    patch: None,
                    linked_artifacts: Vec::new(),
                },
            ]
        };

        // Enhance suggestions with linked artifacts from unified context
        if let Some(uc) = unified_context {
            for suggestion in &mut suggestions {
                // Add persona link if relevant
                if suggestion.title.to_lowercase().contains("persona")
                    || suggestion.description.to_lowercase().contains("persona")
                {
                    if let Some(ref persona_id) = uc.persona.active_persona_id {
                        suggestion.linked_artifacts.push(LinkedArtifact {
                            artifact_type: "persona".to_string(),
                            artifact_id: persona_id.clone(),
                            artifact_name: uc.persona.active_persona_name.clone(),
                        });
                    }
                }

                // Add scenario link if relevant
                if suggestion.title.to_lowercase().contains("scenario")
                    || suggestion.description.to_lowercase().contains("scenario")
                {
                    if let Some(ref scenario_id) = uc.scenario.active_scenario {
                        suggestion.linked_artifacts.push(LinkedArtifact {
                            artifact_type: "scenario".to_string(),
                            artifact_id: scenario_id.clone(),
                            artifact_name: None,
                        });
                    }
                }

                // Add contract links if relevant
                if suggestion.title.to_lowercase().contains("contract")
                    || suggestion.description.to_lowercase().contains("contract")
                {
                    for contract_path in &uc.contract.active_contracts {
                        suggestion.linked_artifacts.push(LinkedArtifact {
                            artifact_type: "contract".to_string(),
                            artifact_id: contract_path.clone(),
                            artifact_name: None,
                        });
                    }
                }

                // Add reality level link if relevant
                if suggestion.title.to_lowercase().contains("reality")
                    || suggestion.description.to_lowercase().contains("reality")
                {
                    if let Some(ref level_name) = uc.reality.level_name {
                        suggestion.linked_artifacts.push(LinkedArtifact {
                            artifact_type: "reality".to_string(),
                            artifact_id: uc.reality.level.map(|l| l.value().to_string()).unwrap_or_default(),
                            artifact_name: Some(level_name.clone()),
                        });
                    }
                }
            }
        }

        Ok(suggestions)
    }

    /// Generate JSON Patch operations for suggestions
    fn generate_patches(
        &self,
        suggestions: &mut [DebugSuggestion],
        context: &FailureContext,
        narrative: &crate::failure_analysis::types::FailureNarrative,
        unified_context: Option<&UnifiedDebugContext>,
    ) -> Result<()> {
        for suggestion in suggestions.iter_mut() {
            // Generate patch based on suggestion type and context
            if let Some(config_path) = &suggestion.config_path {
                // Generate appropriate patch based on the suggestion
                let patch = self.create_patch_for_suggestion(suggestion, config_path, context)?;
                suggestion.patch = patch;
            }
        }
        Ok(())
    }

    /// Create a JSON Patch operation for a specific suggestion
    fn create_patch_for_suggestion(
        &self,
        suggestion: &DebugSuggestion,
        config_path: &str,
        context: &FailureContext,
    ) -> Result<Option<DebugPatch>> {
        // Determine patch operation based on suggestion content
        let patch = if suggestion.action.contains("add") || suggestion.action.contains("Add") {
            // Add operation - typically for adding new examples or configurations
            Some(DebugPatch {
                op: "add".to_string(),
                path: self.build_patch_path(config_path, &suggestion.title),
                value: self.infer_patch_value(suggestion, context),
                from: None,
            })
        } else if suggestion.action.contains("remove") || suggestion.action.contains("Remove") {
            // Remove operation
            Some(DebugPatch {
                op: "remove".to_string(),
                path: self.build_patch_path(config_path, &suggestion.title),
                value: None,
                from: None,
            })
        } else {
            // Replace operation (default)
            Some(DebugPatch {
                op: "replace".to_string(),
                path: self.build_patch_path(config_path, &suggestion.title),
                value: self.infer_patch_value(suggestion, context),
                from: None,
            })
        };

        Ok(patch)
    }

    /// Build JSON Pointer path from config path and suggestion context
    fn build_patch_path(&self, config_path: &str, suggestion_title: &str) -> String {
        // Convert config path to JSON Pointer format
        // Example: "consistency.personas" -> "/consistency/personas"
        // Example: "contract_validation" -> "/contract_validation"
        let mut path = config_path.replace('.', "/");
        if !path.starts_with('/') {
            path = format!("/{}", path);
        }

        // If suggestion mentions a specific field, append it
        if suggestion_title.to_lowercase().contains("error rate") {
            path = format!("{}/error_rate", path);
        } else if suggestion_title.to_lowercase().contains("schema") {
            path = format!("{}/schema", path);
        } else if suggestion_title.to_lowercase().contains("example") {
            path = format!("{}/examples", path);
        }

        path
    }

    /// Infer patch value from suggestion and context
    fn infer_patch_value(
        &self,
        suggestion: &DebugSuggestion,
        context: &FailureContext,
    ) -> Option<serde_json::Value> {
        // Generate appropriate value based on suggestion type
        if suggestion.title.contains("422") || suggestion.description.contains("422") {
            // Add 422 validation error example
            Some(serde_json::json!({
                "status": 422,
                "body": {
                    "error": "Validation failed",
                    "message": context.error_message.clone().unwrap_or_else(|| "Invalid request".to_string())
                }
            }))
        } else if suggestion.title.contains("schema") || suggestion.description.contains("schema") {
            // Schema tightening - suggest number type for amount fields
            if suggestion.description.contains("amount") {
                Some(serde_json::json!({
                    "type": "number",
                    "format": "float"
                }))
            } else {
                Some(serde_json::json!({
                    "type": "string"
                }))
            }
        } else if suggestion.title.contains("persona") || suggestion.description.contains("persona") {
            // Persona configuration
            Some(serde_json::json!({
                "traits": {},
                "domain": "general"
            }))
        } else {
            // Generic configuration value
            Some(serde_json::json!({
                "enabled": true
            }))
        }
    }

    /// Identify related mock configurations
    fn identify_related_configs(
        &self,
        context: &FailureContext,
        unified_context: Option<&UnifiedDebugContext>,
    ) -> Vec<String> {
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

        // Enhance with unified context information
        if let Some(uc) = unified_context {
            if uc.reality.level.is_some() {
                configs.push(format!(
                    "Reality Level: {}",
                    uc.reality.level_name.as_ref().unwrap_or(&"Unknown".to_string())
                ));
            }
            if uc.scenario.active_scenario.is_some() {
                configs.push(format!(
                    "Active Scenario: {}",
                    uc.scenario.active_scenario.as_ref().unwrap()
                ));
            }
            if uc.persona.active_persona_id.is_some() {
                configs.push(format!(
                    "Active Persona: {}",
                    uc.persona.active_persona_id.as_ref().unwrap()
                ));
            }
            if !uc.contract.active_contracts.is_empty() {
                configs.push(format!(
                    "Active Contracts: {}",
                    uc.contract.active_contracts.join(", ")
                ));
            }
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

    /// Unified debug context from subsystems (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unified_context: Option<UnifiedDebugContext>,
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

    /// JSON Patch operation for applying the fix (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<DebugPatch>,

    /// Linked artifacts (persona IDs, scenario names, contract paths)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub linked_artifacts: Vec<LinkedArtifact>,
}

/// Linked artifact reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedArtifact {
    /// Artifact type (persona, scenario, contract, reality)
    pub artifact_type: String,
    /// Artifact ID or path
    pub artifact_id: String,
    /// Artifact name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_name: Option<String>,
}

/// JSON Patch operation for applying a debug suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugPatch {
    /// Patch operation type: "add", "remove", or "replace"
    pub op: String,

    /// JSON Pointer path to the field to modify
    pub path: String,

    /// Value to add or replace (for "add" and "replace" operations)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,

    /// Source path for "move" or "copy" operations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,
}
