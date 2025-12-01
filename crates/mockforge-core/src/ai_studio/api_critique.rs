//! API Architecture Critique Engine
//!
//! This module provides LLM-powered analysis of API schemas (OpenAPI, GraphQL, Protobuf)
//! to detect anti-patterns, redundancies, naming issues, emotional tone problems,
//! and provide restructuring recommendations.
//!
//! # Features
//!
//! - **Anti-pattern Detection**: REST violations, inconsistent naming, poor resource modeling
//! - **Redundancy Detection**: Duplicate endpoints, overlapping functionality
//! - **Naming Quality Assessment**: Inconsistent conventions, unclear names
//! - **Emotional Tone Analysis**: Error messages, user-facing text quality
//! - **Restructuring Recommendations**: Better resource hierarchy, consolidation opportunities
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use mockforge_core::ai_studio::api_critique::{ApiCritique, ApiCritiqueEngine, CritiqueRequest};
//! use mockforge_core::intelligent_behavior::IntelligentBehaviorConfig;
//!
//! # async fn example() -> mockforge_core::Result<()> {
//! let config = IntelligentBehaviorConfig::default();
//! let engine = ApiCritiqueEngine::new(config);
//!
//! let request = CritiqueRequest {
//!     schema: serde_json::json!({"openapi": "3.0.0", ...}),
//!     schema_type: "openapi".to_string(),
//!     focus_areas: vec!["anti-patterns".to_string(), "naming".to_string()],
//! };
//!
//! let critique = engine.analyze(&request).await?;
//! # Ok(())
//! # }
//! ```

use crate::intelligent_behavior::{
    config::IntelligentBehaviorConfig,
    llm_client::{LlmClient, LlmUsage},
    types::LlmGenerationRequest,
};
use crate::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Request for API critique analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CritiqueRequest {
    /// API schema (OpenAPI JSON, GraphQL SDL, or Protobuf)
    pub schema: Value,

    /// Schema type: "openapi", "graphql", or "protobuf"
    pub schema_type: String,

    /// Optional focus areas for analysis
    /// Valid values: "anti-patterns", "redundancy", "naming", "tone", "restructuring"
    #[serde(default)]
    pub focus_areas: Vec<String>,

    /// Optional workspace ID for context
    pub workspace_id: Option<String>,
}

/// API critique result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiCritique {
    /// Detected anti-patterns
    pub anti_patterns: Vec<AntiPattern>,

    /// Detected redundancies
    pub redundancies: Vec<Redundancy>,

    /// Naming quality issues
    pub naming_issues: Vec<NamingIssue>,

    /// Emotional tone analysis
    pub tone_analysis: ToneAnalysis,

    /// Restructuring recommendations
    pub restructuring: RestructuringRecommendations,

    /// Overall score (0-100, higher is better)
    pub overall_score: f64,

    /// Summary of findings
    pub summary: String,

    /// Token usage for this critique
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<u64>,

    /// Estimated cost in USD
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
}

/// Detected anti-pattern in API design
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AntiPattern {
    /// Type of anti-pattern (e.g., "rest_violation", "inconsistent_naming", "poor_resource_modeling")
    pub pattern_type: String,

    /// Severity: "low", "medium", "high", "critical"
    pub severity: String,

    /// Location in schema (path, endpoint, etc.)
    pub location: String,

    /// Description of the issue
    pub description: String,

    /// Suggested fix
    pub suggestion: String,

    /// Example of the problem
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<String>,
}

/// Detected redundancy in API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Redundancy {
    /// Type of redundancy (e.g., "duplicate_endpoint", "overlapping_functionality")
    pub redundancy_type: String,

    /// Severity: "low", "medium", "high"
    pub severity: String,

    /// Affected endpoints/resources
    pub affected_items: Vec<String>,

    /// Description of the redundancy
    pub description: String,

    /// Suggested consolidation
    pub suggestion: String,
}

/// Naming quality issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamingIssue {
    /// Type of naming issue (e.g., "inconsistent_convention", "unclear_name", "abbreviation")
    pub issue_type: String,

    /// Severity: "low", "medium", "high"
    pub severity: String,

    /// Location (field name, endpoint name, etc.)
    pub location: String,

    /// Current name
    pub current_name: String,

    /// Description of the issue
    pub description: String,

    /// Suggested improvement
    pub suggestion: String,
}

/// Emotional tone analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneAnalysis {
    /// Overall tone assessment
    pub overall_tone: String,

    /// Issues found in error messages
    pub error_message_issues: Vec<ToneIssue>,

    /// Issues found in user-facing text
    pub user_facing_issues: Vec<ToneIssue>,

    /// Recommendations for improvement
    pub recommendations: Vec<String>,
}

/// Tone issue in API text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToneIssue {
    /// Type of tone issue (e.g., "too_vague", "too_technical", "unfriendly")
    pub issue_type: String,

    /// Severity: "low", "medium", "high"
    pub severity: String,

    /// Location (error message, description, etc.)
    pub location: String,

    /// Current text
    pub current_text: String,

    /// Description of the issue
    pub description: String,

    /// Suggested improvement
    pub suggestion: String,
}

/// Restructuring recommendations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestructuringRecommendations {
    /// Recommended resource hierarchy improvements
    pub hierarchy_improvements: Vec<HierarchyImprovement>,

    /// Consolidation opportunities
    pub consolidation_opportunities: Vec<ConsolidationOpportunity>,

    /// Resource modeling suggestions
    pub resource_modeling: Vec<ResourceModelingSuggestion>,

    /// Overall restructuring priority: "low", "medium", "high"
    pub priority: String,
}

/// Hierarchy improvement suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchyImprovement {
    /// Current structure
    pub current: String,

    /// Suggested structure
    pub suggested: String,

    /// Rationale
    pub rationale: String,

    /// Impact: "low", "medium", "high"
    pub impact: String,
}

/// Consolidation opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationOpportunity {
    /// Items that can be consolidated
    pub items: Vec<String>,

    /// Description of the opportunity
    pub description: String,

    /// Suggested consolidation approach
    pub suggestion: String,

    /// Benefits of consolidation
    pub benefits: Vec<String>,
}

/// Resource modeling suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceModelingSuggestion {
    /// Current modeling approach
    pub current: String,

    /// Suggested modeling approach
    pub suggested: String,

    /// Rationale
    pub rationale: String,
}

/// API Critique Engine
pub struct ApiCritiqueEngine {
    /// LLM client for analysis
    llm_client: LlmClient,

    /// Configuration
    config: IntelligentBehaviorConfig,
}

impl ApiCritiqueEngine {
    /// Create a new API critique engine
    pub fn new(config: IntelligentBehaviorConfig) -> Self {
        let llm_client = LlmClient::new(config.behavior_model.clone());
        Self { llm_client, config }
    }

    /// Analyze an API schema and generate critique
    pub async fn analyze(&self, request: &CritiqueRequest) -> Result<ApiCritique> {
        // Build the analysis prompt
        let system_prompt = self.build_system_prompt();
        let user_prompt = self.build_user_prompt(request)?;

        // Generate critique using LLM
        let llm_request = LlmGenerationRequest {
            system_prompt,
            user_prompt,
            temperature: 0.3, // Lower temperature for more consistent analysis
            max_tokens: 4000,
            schema: None,
        };

        let (response_json, usage) = self.llm_client.generate_with_usage(&llm_request).await?;

        // Parse the response
        let critique = self.parse_critique_response(response_json)?;

        // Calculate cost
        let cost_usd = self.estimate_cost(&usage);

        Ok(ApiCritique {
            tokens_used: Some(usage.total_tokens),
            cost_usd: Some(cost_usd),
            ..critique
        })
    }

    /// Build system prompt for API critique
    fn build_system_prompt(&self) -> String {
        r#"You are an expert API architect and design reviewer. Your task is to analyze API schemas
(OpenAPI, GraphQL, or Protobuf) and provide comprehensive critique covering:

1. **Anti-patterns**: REST violations, inconsistent naming, poor resource modeling
2. **Redundancy**: Duplicate endpoints, overlapping functionality
3. **Naming Quality**: Inconsistent conventions, unclear names, abbreviations
4. **Emotional Tone**: Error messages that are too vague, technical, or unfriendly
5. **Restructuring**: Better resource hierarchy, consolidation opportunities

Return your analysis as a JSON object with the following structure:
{
  "anti_patterns": [
    {
      "pattern_type": "rest_violation|inconsistent_naming|poor_resource_modeling",
      "severity": "low|medium|high|critical",
      "location": "path/to/endpoint or field name",
      "description": "Clear description of the issue",
      "suggestion": "How to fix it",
      "example": "Optional example of the problem"
    }
  ],
  "redundancies": [
    {
      "redundancy_type": "duplicate_endpoint|overlapping_functionality",
      "severity": "low|medium|high",
      "affected_items": ["endpoint1", "endpoint2"],
      "description": "Description of redundancy",
      "suggestion": "How to consolidate"
    }
  ],
  "naming_issues": [
    {
      "issue_type": "inconsistent_convention|unclear_name|abbreviation",
      "severity": "low|medium|high",
      "location": "field or endpoint name",
      "current_name": "actual name",
      "description": "What's wrong with it",
      "suggestion": "Better name"
    }
  ],
  "tone_analysis": {
    "overall_tone": "friendly|neutral|technical|unfriendly",
    "error_message_issues": [
      {
        "issue_type": "too_vague|too_technical|unfriendly",
        "severity": "low|medium|high",
        "location": "error code or endpoint",
        "current_text": "actual error message",
        "description": "What's wrong",
        "suggestion": "Improved message"
      }
    ],
    "user_facing_issues": [],
    "recommendations": ["list of recommendations"]
  },
  "restructuring": {
    "hierarchy_improvements": [
      {
        "current": "current structure",
        "suggested": "suggested structure",
        "rationale": "why this is better",
        "impact": "low|medium|high"
      }
    ],
    "consolidation_opportunities": [
      {
        "items": ["item1", "item2"],
        "description": "what can be consolidated",
        "suggestion": "how to consolidate",
        "benefits": ["benefit1", "benefit2"]
      }
    ],
    "resource_modeling": [
      {
        "current": "current approach",
        "suggested": "suggested approach",
        "rationale": "why this is better"
      }
    ],
    "priority": "low|medium|high"
  },
  "overall_score": 75.5,
  "summary": "Overall assessment summary"
}

Be thorough but practical. Focus on actionable recommendations."#
            .to_string()
    }

    /// Build user prompt with schema and focus areas
    fn build_user_prompt(&self, request: &CritiqueRequest) -> Result<String> {
        let schema_str = serde_json::to_string_pretty(&request.schema)
            .map_err(|e| crate::Error::generic(format!("Failed to serialize schema: {}", e)))?;

        let focus_areas_text = if request.focus_areas.is_empty() {
            "all areas".to_string()
        } else {
            request.focus_areas.join(", ")
        };

        Ok(format!(
            r#"Analyze this {} API schema and provide critique focusing on: {}

Schema:
{}

Please provide a comprehensive analysis covering all requested areas. Be specific with locations, examples, and actionable suggestions."#,
            request.schema_type, focus_areas_text, schema_str
        ))
    }

    /// Parse LLM response into ApiCritique struct
    fn parse_critique_response(&self, response: Value) -> Result<ApiCritique> {
        // Try to extract the critique from the response
        let critique_json = if response.is_object() && response.get("critique").is_some() {
            response.get("critique").unwrap().clone()
        } else if response.is_object() {
            response
        } else {
            return Err(crate::Error::generic(
                "LLM response is not a valid JSON object".to_string(),
            ));
        };

        // Parse into ApiCritique struct
        let critique: ApiCritique = serde_json::from_value(critique_json.clone()).map_err(|e| {
            crate::Error::generic(format!(
                "Failed to parse critique response: {}. Response was: {}",
                e,
                serde_json::to_string_pretty(&critique_json).unwrap_or_default()
            ))
        })?;

        Ok(critique)
    }

    /// Estimate cost in USD based on token usage
    fn estimate_cost(&self, usage: &LlmUsage) -> f64 {
        // Rough cost estimates per 1K tokens
        // These are approximate and should be updated based on actual provider pricing
        let cost_per_1k_tokens =
            match self.config.behavior_model.llm_provider.to_lowercase().as_str() {
                "openai" => match self.config.behavior_model.model.to_lowercase().as_str() {
                    model if model.contains("gpt-4") => 0.03,
                    model if model.contains("gpt-3.5") => 0.002,
                    _ => 0.002,
                },
                "anthropic" => 0.008,
                "ollama" => 0.0, // Local models are free
                _ => 0.002,
            };

        (usage.total_tokens as f64 / 1000.0) * cost_per_1k_tokens
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

    #[tokio::test]
    #[ignore] // Requires LLM service
    async fn test_api_critique_engine_creation() {
        let config = create_test_config();
        let engine = ApiCritiqueEngine::new(config);
        // Engine should be created successfully
        assert!(true);
    }

    #[test]
    fn test_critique_request_serialization() {
        let request = CritiqueRequest {
            schema: serde_json::json!({"openapi": "3.0.0"}),
            schema_type: "openapi".to_string(),
            focus_areas: vec!["anti-patterns".to_string()],
            workspace_id: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("openapi"));
        assert!(json.contains("anti-patterns"));
    }
}
