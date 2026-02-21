//! AI-powered recommendation engine for contract diff analysis
//!
//! This module uses LLM to generate contextual recommendations for fixing contract mismatches,
//! going beyond structural diffs to provide intelligent suggestions.

use super::types::{ContractDiffConfig, Mismatch, Recommendation};
use crate::intelligent_behavior::config::BehaviorModelConfig;
use crate::intelligent_behavior::llm_client::LlmClient;
use crate::intelligent_behavior::types::LlmGenerationRequest;
use crate::Result;
use std::collections::HashMap;

/// AI-powered recommendation engine
pub struct RecommendationEngine {
    /// LLM client for generating recommendations
    llm_client: Option<LlmClient>,

    /// Configuration
    config: ContractDiffConfig,
}

impl RecommendationEngine {
    /// Create a new recommendation engine
    pub fn new(config: ContractDiffConfig) -> Result<Self> {
        let llm_client = if config.use_ai_recommendations {
            // Create LLM client configuration
            let llm_config = BehaviorModelConfig {
                llm_provider: config.llm_provider.clone(),
                model: config.llm_model.clone(),
                api_key: config.api_key.clone(),
                api_endpoint: None,
                temperature: 0.7, // Lower temperature for more focused recommendations
                max_tokens: 2000,
                rules: crate::intelligent_behavior::BehaviorRules::default(), // No specific rules for contract diff recommendations
            };

            Some(LlmClient::new(llm_config))
        } else {
            None
        };

        Ok(Self { llm_client, config })
    }

    /// Generate recommendations for mismatches
    pub async fn generate_recommendations(
        &self,
        mismatches: &[Mismatch],
        request_context: &RequestContext,
    ) -> Result<Vec<Recommendation>> {
        if !self.config.use_ai_recommendations || self.llm_client.is_none() {
            // Return basic recommendations without AI
            return Ok(self.generate_basic_recommendations(mismatches));
        }

        let mut recommendations = Vec::new();

        // Group mismatches by type for batch processing
        let mut grouped: HashMap<String, Vec<&Mismatch>> = HashMap::new();
        for mismatch in mismatches {
            let key = format!("{:?}", mismatch.mismatch_type);
            grouped.entry(key).or_default().push(mismatch);
        }

        // Generate recommendations for each group
        for (_group_key, group_mismatches) in grouped {
            if group_mismatches.len() > self.config.max_recommendations {
                // Limit to max_recommendations
                let limited = group_mismatches
                    .iter()
                    .take(self.config.max_recommendations)
                    .copied()
                    .collect::<Vec<_>>();
                let group_recs =
                    self.generate_ai_recommendations_for_group(&limited, request_context).await?;
                recommendations.extend(group_recs);
            } else {
                let group_recs = self
                    .generate_ai_recommendations_for_group(&group_mismatches, request_context)
                    .await?;
                recommendations.extend(group_recs);
            }
        }

        Ok(recommendations)
    }

    /// Generate AI-powered recommendations for a group of mismatches
    async fn generate_ai_recommendations_for_group(
        &self,
        mismatches: &[&Mismatch],
        context: &RequestContext,
    ) -> Result<Vec<Recommendation>> {
        let llm_client = self
            .llm_client
            .as_ref()
            .ok_or_else(|| crate::Error::generic("LLM client not initialized"))?;

        // Build prompt for LLM
        let prompt = self.build_recommendation_prompt(mismatches, context);

        // Generate recommendation using LLM
        let request = LlmGenerationRequest::new(self.get_system_prompt(), prompt)
            .with_temperature(0.7)
            .with_max_tokens(2000);

        let response = llm_client.generate(&request).await?;

        // Parse LLM response into recommendations
        self.parse_llm_recommendations(response, mismatches)
    }

    /// Build prompt for LLM recommendation generation
    fn build_recommendation_prompt(
        &self,
        mismatches: &[&Mismatch],
        context: &RequestContext,
    ) -> String {
        let mut prompt = String::from(
            "You are analyzing API contract mismatches between front-end requests and backend specifications.\n\n",
        );

        prompt.push_str("## Request Context\n");
        prompt.push_str(&format!("Endpoint: {} {}\n", context.method, context.path));
        if let Some(body) = &context.request_body {
            prompt.push_str(&format!(
                "Request Body: {}\n",
                serde_json::to_string(body).unwrap_or_default()
            ));
        }
        prompt.push_str(&format!("Contract Format: {}\n\n", context.contract_format));

        prompt.push_str("## Detected Mismatches\n\n");
        for (idx, mismatch) in mismatches.iter().enumerate() {
            prompt.push_str(&format!("### Mismatch {}: {:?}\n", idx + 1, mismatch.mismatch_type));
            prompt.push_str(&format!("Path: {}\n", mismatch.path));
            prompt.push_str(&format!("Description: {}\n", mismatch.description));
            if let Some(expected) = &mismatch.expected {
                prompt.push_str(&format!("Expected: {}\n", expected));
            }
            if let Some(actual) = &mismatch.actual {
                prompt.push_str(&format!("Actual: {}\n", actual));
            }
            prompt.push_str(&format!("Severity: {:?}\n\n", mismatch.severity));
        }

        prompt.push_str("## Task\n\n");
        prompt.push_str("For each mismatch, provide:\n");
        prompt.push_str("1. A clear, actionable recommendation for fixing the issue\n");
        prompt.push_str("2. A suggested fix (code or configuration change)\n");
        prompt.push_str("3. Reasoning explaining why this fix is appropriate\n");
        if self.config.include_examples {
            prompt.push_str("4. An example showing the fix applied\n");
        }
        prompt.push_str(
            "\nReturn your response as a JSON array of recommendation objects with the following structure:\n",
        );
        prompt.push_str(
            r#"[
  {
    "mismatch_index": 0,
    "recommendation": "Clear recommendation text",
    "suggested_fix": "Specific fix or action",
    "reasoning": "Why this fix is appropriate",
    "example": { "before": "...", "after": "..." }
  }
]"#,
        );

        prompt
    }

    /// Get system prompt for LLM
    fn get_system_prompt(&self) -> String {
        String::from(
            "You are an expert API contract analyst. Your role is to analyze mismatches between \
            front-end API requests and backend contract specifications, and provide clear, \
            actionable recommendations for fixing these issues. Your recommendations should be \
            practical, well-reasoned, and include specific examples when helpful. Always consider \
            the context of the API and the severity of the mismatch when making recommendations.",
        )
    }

    /// Parse LLM response into recommendation objects
    fn parse_llm_recommendations(
        &self,
        response: serde_json::Value,
        mismatches: &[&Mismatch],
    ) -> Result<Vec<Recommendation>> {
        let mut recommendations = Vec::new();

        // Try to extract recommendations array from response
        let recommendations_array = if response.is_array() {
            Some(response.as_array().unwrap())
        } else if let Some(arr) = response.get("recommendations") {
            arr.as_array()
        } else if let Some(arr) = response.get("data") {
            arr.as_array()
        } else {
            None
        };

        if let Some(recs) = recommendations_array {
            for (idx, rec_json) in recs.iter().enumerate() {
                let mismatch_index =
                    rec_json.get("mismatch_index").and_then(|v| v.as_u64()).unwrap_or(idx as u64)
                        as usize;

                if mismatch_index < mismatches.len() {
                    let mismatch = mismatches[mismatch_index];
                    let recommendation = Recommendation {
                        id: format!("rec_{}_{}", mismatch.path, idx),
                        mismatch_id: format!("mismatch_{}", mismatch_index),
                        recommendation: rec_json
                            .get("recommendation")
                            .and_then(|v| v.as_str())
                            .unwrap_or("No recommendation provided")
                            .to_string(),
                        suggested_fix: rec_json
                            .get("suggested_fix")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        confidence: mismatch.confidence, // Use mismatch confidence as base
                        reasoning: rec_json
                            .get("reasoning")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        example: rec_json.get("example").cloned(),
                    };

                    recommendations.push(recommendation);
                }
            }
        } else {
            // Fallback: try to parse as text and extract JSON
            if let Some(text) = response.as_str() {
                // Try to find JSON in text
                if let Some(start) = text.find('[') {
                    if let Some(end) = text.rfind(']') {
                        let json_str = &text[start..=end];
                        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(json_str) {
                            return self.parse_llm_recommendations(parsed, mismatches);
                        }
                    }
                }
            }

            // If all else fails, generate basic recommendations
            return Ok(self.generate_basic_recommendations(
                &mismatches.iter().map(|m| (*m).clone()).collect::<Vec<_>>(),
            ));
        }

        Ok(recommendations)
    }

    /// Generate basic recommendations without AI
    fn generate_basic_recommendations(&self, mismatches: &[Mismatch]) -> Vec<Recommendation> {
        mismatches
            .iter()
            .enumerate()
            .map(|(idx, mismatch)| {
                let (recommendation, suggested_fix) = match mismatch.mismatch_type {
                    super::types::MismatchType::MissingRequiredField => (
                        format!("Add the required field '{}' to the request", mismatch.path),
                        format!("Add field: {}", mismatch.path),
                    ),
                    super::types::MismatchType::TypeMismatch => (
                        format!(
                            "Change the type of '{}' from {} to {}",
                            mismatch.path,
                            mismatch.actual.as_ref().unwrap_or(&"unknown".to_string()),
                            mismatch.expected.as_ref().unwrap_or(&"unknown".to_string())
                        ),
                        format!(
                            "Update field type: {} -> {}",
                            mismatch.path,
                            mismatch.expected.as_ref().unwrap_or(&"unknown".to_string())
                        ),
                    ),
                    super::types::MismatchType::UnexpectedField => (
                        format!("Remove the unexpected field '{}' from the request", mismatch.path),
                        format!("Remove field: {}", mismatch.path),
                    ),
                    _ => (mismatch.description.clone(), "Review and fix the mismatch".to_string()),
                };

                Recommendation {
                    id: format!("rec_{}_{}", mismatch.path, idx),
                    mismatch_id: format!("mismatch_{}", idx),
                    recommendation,
                    suggested_fix: Some(suggested_fix),
                    confidence: mismatch.confidence,
                    reasoning: Some(format!(
                        "Based on mismatch type: {:?}",
                        mismatch.mismatch_type
                    )),
                    example: None,
                }
            })
            .collect()
    }
}

/// Context for recommendation generation
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// HTTP method
    pub method: String,

    /// Request path
    pub path: String,

    /// Request body
    pub request_body: Option<serde_json::Value>,

    /// Contract format
    pub contract_format: String,

    /// Additional context
    pub additional_context: HashMap<String, serde_json::Value>,
}

impl RequestContext {
    /// Create a new request context
    pub fn new(method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            method: method.into(),
            path: path.into(),
            request_body: None,
            contract_format: "openapi-3.0".to_string(),
            additional_context: HashMap::new(),
        }
    }

    /// Add request body
    pub fn with_body(mut self, body: serde_json::Value) -> Self {
        self.request_body = Some(body);
        self
    }

    /// Set contract format
    pub fn with_contract_format(mut self, format: impl Into<String>) -> Self {
        self.contract_format = format.into();
        self
    }
}
