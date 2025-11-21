//! Semantic drift analysis for contract diffs
//!
//! This module provides Layer 2 semantic analysis that detects meaning changes
//! beyond structural diffs, such as description changes, enum narrowing,
//! nullable changes hidden behind oneOf, and error code removals.

use super::types::{ContractDiffConfig, Mismatch, MismatchSeverity, MismatchType};
use crate::intelligent_behavior::config::BehaviorModelConfig;
use crate::intelligent_behavior::llm_client::LlmClient;
use crate::intelligent_behavior::types::LlmGenerationRequest;
use crate::openapi::OpenApiSpec;
use crate::Result;
use chrono::Utc;
use openapiv3;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Semantic drift analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticDriftResult {
    /// Semantic confidence score (0.0-1.0)
    pub semantic_confidence: f64,
    /// Soft-breaking score (0.0-1.0) - likelihood this is a soft-breaking change
    pub soft_breaking_score: f64,
    /// Type of semantic change detected
    pub change_type: SemanticChangeType,
    /// Full LLM analysis and reasoning
    pub llm_analysis: Value,
    /// Before semantic state
    pub before_semantic_state: Value,
    /// After semantic state
    pub after_semantic_state: Value,
    /// Detected semantic mismatches
    pub semantic_mismatches: Vec<Mismatch>,
}

/// Type of semantic change
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SemanticChangeType {
    /// Description meaning changed
    DescriptionChange,
    /// Enum values narrowed (values removed)
    EnumNarrowing,
    /// Nullable → non-nullable change hidden behind oneOf/anyOf
    NullableChange,
    /// Error code removed
    ErrorCodeRemoved,
    /// Semantic constraint changed (e.g., format, pattern)
    SemanticConstraintChange,
    /// General meaning shift
    MeaningShift,
    /// Soft-breaking change
    SoftBreakingChange,
}

/// Semantic analyzer for detecting meaning changes
pub struct SemanticAnalyzer {
    /// LLM client for semantic analysis
    llm_client: Option<LlmClient>,
    /// Configuration
    config: ContractDiffConfig,
}

impl SemanticAnalyzer {
    /// Create a new semantic analyzer
    pub fn new(config: ContractDiffConfig) -> Result<Self> {
        let llm_client = if config.semantic_analysis_enabled {
            let llm_config = BehaviorModelConfig {
                llm_provider: config.llm_provider.clone(),
                model: config.llm_model.clone(),
                api_key: config.api_key.clone(),
                api_endpoint: None,
                temperature: 0.3, // Lower temperature for more precise semantic analysis
                max_tokens: 3000,
                rules: crate::intelligent_behavior::BehaviorRules::default(),
            };

            Some(LlmClient::new(llm_config))
        } else {
            None
        };

        Ok(Self { llm_client, config })
    }

    /// Analyze semantic drift between two contract states
    ///
    /// This is Layer 2 analysis that runs after structural diff to detect
    /// meaning changes that might not be structurally breaking but are
    /// semantically significant.
    pub async fn analyze_semantic_drift(
        &self,
        before_spec: &OpenApiSpec,
        after_spec: &OpenApiSpec,
        endpoint_path: &str,
        method: &str,
    ) -> Result<Option<SemanticDriftResult>> {
        if !self.config.semantic_analysis_enabled {
            return Ok(None);
        }

        // Extract relevant schemas for the endpoint
        let before_schema = self.extract_endpoint_schema(before_spec, endpoint_path, method);
        let after_schema = self.extract_endpoint_schema(after_spec, endpoint_path, method);

        if before_schema.is_none() || after_schema.is_none() {
            return Ok(None);
        }

        let before = before_schema.unwrap();
        let after = after_schema.unwrap();

        // Detect semantic changes using rule-based analysis first
        let rule_based_changes = self.detect_rule_based_changes(&before, &after);

        // If we have an LLM client, use it for deeper semantic analysis
        if let Some(ref llm_client) = self.llm_client {
            let llm_result = self
                .analyze_with_llm(llm_client, &before, &after, endpoint_path, method)
                .await?;

            // Combine rule-based and LLM results
            Ok(Some(self.combine_results(rule_based_changes, llm_result, before, after)))
        } else {
            // Use only rule-based analysis
            if rule_based_changes.is_empty() {
                return Ok(None);
            }

            // Create result from rule-based changes only
            let change_type = self.determine_change_type(&rule_based_changes);
            let semantic_confidence = 0.6; // Lower confidence without LLM
            let soft_breaking_score = self.calculate_soft_breaking_score(&rule_based_changes);

            Ok(Some(SemanticDriftResult {
                semantic_confidence,
                soft_breaking_score,
                change_type,
                llm_analysis: serde_json::json!({}),
                before_semantic_state: before,
                after_semantic_state: after,
                semantic_mismatches: rule_based_changes,
            }))
        }
    }

    /// Extract schema for a specific endpoint
    fn extract_endpoint_schema(
        &self,
        spec: &OpenApiSpec,
        endpoint_path: &str,
        method: &str,
    ) -> Option<Value> {
        // This is a simplified extraction - in practice, you'd properly
        // navigate the OpenAPI spec structure
        spec.spec
            .paths
            .paths
            .get(endpoint_path)
            .and_then(|path_item| {
                path_item.as_item().and_then(|item| {
                    // Get operation based on method
                    let operation = match method.to_uppercase().as_str() {
                        "GET" => item.get.as_ref(),
                        "POST" => item.post.as_ref(),
                        "PUT" => item.put.as_ref(),
                        "DELETE" => item.delete.as_ref(),
                        "PATCH" => item.patch.as_ref(),
                        "HEAD" => item.head.as_ref(),
                        "OPTIONS" => item.options.as_ref(),
                        "TRACE" => item.trace.as_ref(),
                        _ => None,
                    }?;
                    
                    operation.responses
                        .responses
                        .get(&openapiv3::StatusCode::Code(200))
                        .and_then(|resp| {
                            resp.as_item().and_then(|r| {
                                r.content
                                    .get("application/json")
                                    .and_then(|media| {
                                        media.schema.as_ref().map(|s| {
                                            serde_json::to_value(s).unwrap_or_default()
                                        })
                                    })
                            })
                        })
                })
            })
    }

    /// Detect rule-based semantic changes
    fn detect_rule_based_changes(&self, before: &Value, after: &Value) -> Vec<Mismatch> {
        let mut mismatches = Vec::new();

        // Detect description changes
        mismatches.extend(self.detect_description_changes(before, after));

        // Detect enum narrowing
        mismatches.extend(self.detect_enum_narrowing(before, after));

        // Detect nullable changes
        mismatches.extend(self.detect_nullable_changes(before, after));

        // Detect error code changes (if error responses are in schema)
        mismatches.extend(self.detect_error_code_changes(before, after));

        mismatches
    }

    /// Detect description meaning changes
    fn detect_description_changes(&self, before: &Value, after: &Value) -> Vec<Mismatch> {
        let mut mismatches = Vec::new();

        // Compare descriptions at schema level
        if let (Some(before_desc), Some(after_desc)) = (
            before.get("description").and_then(|v| v.as_str()),
            after.get("description").and_then(|v| v.as_str()),
        ) {
            if before_desc != after_desc {
                // Check if it's a significant meaning change (not just wording)
                let is_significant = self.is_description_meaning_change(before_desc, after_desc);

                if is_significant {
                    mismatches.push(Mismatch {
                        mismatch_type: MismatchType::SemanticDescriptionChange,
                        path: "description".to_string(),
                        method: None,
                        expected: Some(before_desc.to_string()),
                        actual: Some(after_desc.to_string()),
                        description: format!(
                            "Description meaning changed: '{}' → '{}'",
                            before_desc, after_desc
                        ),
                        severity: MismatchSeverity::Medium,
                        confidence: 0.7,
                        context: HashMap::new(),
                    });
                }
            }
        }

        mismatches
    }

    /// Check if description change is a meaning change (simplified heuristic)
    fn is_description_meaning_change(&self, before: &str, after: &str) -> bool {
        // Simple heuristic: if more than 30% of words changed, consider it significant
        let before_words: Vec<&str> = before.split_whitespace().collect();
        let after_words: Vec<&str> = after.split_whitespace().collect();

        if before_words.is_empty() || after_words.is_empty() {
            return true; // Empty to non-empty or vice versa is significant
        }

        let common_words: usize = before_words
            .iter()
            .filter(|w| after_words.contains(w))
            .count();

        let change_ratio = 1.0 - (common_words as f64 / before_words.len().max(after_words.len()) as f64);
        change_ratio > 0.3
    }

    /// Detect enum narrowing (values removed)
    fn detect_enum_narrowing(&self, before: &Value, after: &Value) -> Vec<Mismatch> {
        let mut mismatches = Vec::new();

        if let (Some(before_enum), Some(after_enum)) = (
            before.get("enum").and_then(|v| v.as_array()),
            after.get("enum").and_then(|v| v.as_array()),
        ) {
            let before_set: std::collections::HashSet<&Value> =
                before_enum.iter().collect();
            let after_set: std::collections::HashSet<&Value> = after_enum.iter().collect();

            let removed: Vec<_> = before_set.difference(&after_set).collect();

            if !removed.is_empty() {
                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::SemanticEnumNarrowing,
                    path: "enum".to_string(),
                    method: None,
                    expected: Some(format!("{:?}", before_enum)),
                    actual: Some(format!("{:?}", after_enum)),
                    description: format!(
                        "Enum values narrowed: {} value(s) removed",
                        removed.len()
                    ),
                    severity: MismatchSeverity::High,
                    confidence: 1.0, // Structural change is certain
                    context: HashMap::new(),
                });
            }
        }

        mismatches
    }

    /// Detect nullable changes hidden behind oneOf/anyOf
    fn detect_nullable_changes(&self, before: &Value, after: &Value) -> Vec<Mismatch> {
        let mut mismatches = Vec::new();

        // Check if nullable changed
        let before_nullable = before.get("nullable").and_then(|v| v.as_bool()).unwrap_or(false);
        let after_nullable = after.get("nullable").and_then(|v| v.as_bool()).unwrap_or(false);

        if before_nullable && !after_nullable {
            // Check if it's hidden behind oneOf/anyOf
            let is_hidden = after.get("oneOf").is_some() || after.get("anyOf").is_some();

            if is_hidden {
                mismatches.push(Mismatch {
                    mismatch_type: MismatchType::SemanticNullabilityChange,
                    path: "nullable".to_string(),
                    method: None,
                    expected: Some("nullable: true".to_string()),
                    actual: Some("nullable: false (hidden behind oneOf/anyOf)".to_string()),
                    description: "Field became non-nullable but change is hidden behind oneOf/anyOf".to_string(),
                    severity: MismatchSeverity::High,
                    confidence: 0.8,
                    context: HashMap::new(),
                });
            }
        }

        mismatches
    }

    /// Detect error code changes
    fn detect_error_code_changes(&self, _before: &Value, _after: &Value) -> Vec<Mismatch> {
        // This would analyze error response schemas
        // For now, return empty - would need full OpenAPI spec context
        Vec::new()
    }

    /// Analyze with LLM for deeper semantic understanding
    async fn analyze_with_llm(
        &self,
        llm_client: &LlmClient,
        before: &Value,
        after: &Value,
        endpoint_path: &str,
        method: &str,
    ) -> Result<Value> {
        let prompt = self.build_semantic_analysis_prompt(before, after, endpoint_path, method);

        let request = LlmGenerationRequest::new(self.get_system_prompt(), prompt)
            .with_temperature(0.3)
            .with_max_tokens(3000);

        let response = llm_client.generate(&request).await?;

        // Response is already a serde_json::Value, extract fields
        let analysis = response
            .get("analysis")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| serde_json::to_string(&response).unwrap_or_default());
        
        let confidence = response
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);
        
        let soft_breaking_score = response
            .get("soft_breaking_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        Ok(serde_json::json!({
            "analysis": analysis,
            "confidence": confidence,
            "soft_breaking_score": soft_breaking_score
        }))
    }

    /// Build prompt for semantic analysis
    fn build_semantic_analysis_prompt(
        &self,
        before: &Value,
        after: &Value,
        endpoint_path: &str,
        method: &str,
    ) -> String {
        format!(
            r#"Analyze the semantic differences between these two API contract schemas for endpoint {} {}.

Before schema:
{}

After schema:
{}

Please identify:
1. Any changes in meaning or semantics (not just structural changes)
2. Description changes that alter the intended behavior
3. Enum narrowing or constraint tightening
4. Nullable changes that might break clients
5. Error code removals
6. Any "soft-breaking" changes that won't cause immediate failures but will cause issues

Provide your analysis in JSON format with:
- semantic_confidence: 0.0-1.0
- soft_breaking_score: 0.0-1.0
- change_type: one of the semantic change types
- reasoning: detailed explanation
- detected_changes: array of specific changes found"#,
            method,
            endpoint_path,
            serde_json::to_string_pretty(before).unwrap_or_default(),
            serde_json::to_string_pretty(after).unwrap_or_default()
        )
    }

    /// Get system prompt for semantic analysis
    fn get_system_prompt(&self) -> String {
        "You are an expert API contract analyst specializing in detecting semantic drift and soft-breaking changes in API contracts. Your analysis helps teams understand when API changes might break clients even if they're not structurally breaking.".to_string()
    }

    /// Combine rule-based and LLM results
    fn combine_results(
        &self,
        rule_based: Vec<Mismatch>,
        llm_result: Value,
        before: Value,
        after: Value,
    ) -> SemanticDriftResult {
        let semantic_confidence = llm_result
            .get("semantic_confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7);

        let soft_breaking_score = llm_result
            .get("soft_breaking_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.5);

        let change_type_str = llm_result
            .get("change_type")
            .and_then(|v| v.as_str())
            .unwrap_or("meaning_shift");

        let change_type = match change_type_str {
            "description_change" => SemanticChangeType::DescriptionChange,
            "enum_narrowing" => SemanticChangeType::EnumNarrowing,
            "nullable_change" => SemanticChangeType::NullableChange,
            "error_code_removed" => SemanticChangeType::ErrorCodeRemoved,
            "semantic_constraint_change" => SemanticChangeType::SemanticConstraintChange,
            "soft_breaking_change" => SemanticChangeType::SoftBreakingChange,
            _ => SemanticChangeType::MeaningShift,
        };

        // Merge rule-based mismatches with any from LLM
        let mut semantic_mismatches = rule_based;

        SemanticDriftResult {
            semantic_confidence,
            soft_breaking_score,
            change_type,
            llm_analysis: llm_result,
            before_semantic_state: before,
            after_semantic_state: after,
            semantic_mismatches,
        }
    }

    /// Determine change type from mismatches
    fn determine_change_type(&self, mismatches: &[Mismatch]) -> SemanticChangeType {
        for mismatch in mismatches {
            match mismatch.mismatch_type {
                MismatchType::SemanticDescriptionChange => {
                    return SemanticChangeType::DescriptionChange
                }
                MismatchType::SemanticEnumNarrowing => return SemanticChangeType::EnumNarrowing,
                MismatchType::SemanticNullabilityChange => {
                    return SemanticChangeType::NullableChange
                }
                MismatchType::SemanticErrorCodeRemoved => {
                    return SemanticChangeType::ErrorCodeRemoved
                }
                _ => {}
            }
        }

        SemanticChangeType::MeaningShift
    }

    /// Calculate soft-breaking score
    fn calculate_soft_breaking_score(&self, mismatches: &[Mismatch]) -> f64 {
        if mismatches.is_empty() {
            return 0.0;
        }

        // Higher score for more severe mismatches
        let total_score: f64 = mismatches
            .iter()
            .map(|m| {
                let severity_score = match m.severity {
                    MismatchSeverity::Critical => 1.0,
                    MismatchSeverity::High => 0.8,
                    MismatchSeverity::Medium => 0.6,
                    MismatchSeverity::Low => 0.4,
                    MismatchSeverity::Info => 0.2,
                };
                severity_score * m.confidence
            })
            .sum();

        (total_score / mismatches.len() as f64).min(1.0)
    }
}

