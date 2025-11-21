//! AI-powered remediation suggestion generator
//!
//! This module generates remediation suggestions for threat findings
//! using LLM analysis.

use super::types::{RemediationSuggestion, ThreatFinding};
use crate::intelligent_behavior::config::BehaviorModelConfig;
use crate::intelligent_behavior::llm_client::LlmClient;
use crate::intelligent_behavior::types::LlmGenerationRequest;
use crate::Result;
use serde_json::json;

/// Remediation generator using AI
pub struct RemediationGenerator {
    /// LLM client
    llm_client: Option<LlmClient>,
    /// Whether AI generation is enabled
    enabled: bool,
}

impl RemediationGenerator {
    /// Create a new remediation generator
    pub fn new(
        enabled: bool,
        llm_provider: String,
        llm_model: String,
        api_key: Option<String>,
    ) -> Result<Self> {
        let llm_client = if enabled {
            let llm_config = BehaviorModelConfig {
                llm_provider: llm_provider.clone(),
                model: llm_model.clone(),
                api_key: api_key.clone(),
                api_endpoint: None,
                temperature: 0.3, // Lower temperature for precise suggestions
                max_tokens: 2000,
                rules: crate::intelligent_behavior::BehaviorRules::default(),
            };

            Some(LlmClient::new(llm_config))
        } else {
            None
        };

        Ok(Self { llm_client, enabled })
    }

    /// Generate remediation suggestions for findings
    pub async fn generate_remediations(
        &self,
        findings: &[ThreatFinding],
    ) -> Result<Vec<RemediationSuggestion>> {
        if !self.enabled || self.llm_client.is_none() {
            return Ok(self.generate_basic_remediations(findings));
        }

        let mut suggestions = Vec::new();

        for finding in findings {
            if let Some(ref llm_client) = self.llm_client {
                match self.generate_ai_remediation(llm_client, finding).await {
                    Ok(suggestion) => suggestions.push(suggestion),
                    Err(e) => {
                        // Fallback to basic remediation on error
                        suggestions.push(self.generate_basic_remediation(finding));
                        tracing::warn!("Failed to generate AI remediation: {}", e);
                    }
                }
            }
        }

        Ok(suggestions)
    }

    /// Generate AI-powered remediation
    async fn generate_ai_remediation(
        &self,
        llm_client: &LlmClient,
        finding: &ThreatFinding,
    ) -> Result<RemediationSuggestion> {
        let prompt = self.build_remediation_prompt(finding);

        let request = LlmGenerationRequest::new(self.get_system_prompt(), prompt)
            .with_temperature(0.3)
            .with_max_tokens(2000);

        let response = llm_client.generate(&request).await?;

        // Response is already a serde_json::Value
        let suggestion_text = response
            .get("suggestion")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                // Fallback: try to extract text from "response" field or use the whole value as string
                response
                    .get("response")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .or_else(|| {
                        // Last resort: serialize the whole response as a string
                        serde_json::to_string(&response).ok()
                    })
            })
            .unwrap_or_else(|| "No remediation suggestion available".to_string());

        let code_example = response
            .get("code_example")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let confidence = response
            .get("confidence")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7);

        Ok(RemediationSuggestion {
            finding_id: format!("finding_{}", finding.field_path.as_deref().unwrap_or("unknown")),
            suggestion: suggestion_text,
            code_example,
            confidence,
            ai_generated: true,
            priority: self.calculate_priority(finding),
        })
    }

    /// Build prompt for remediation generation
    fn build_remediation_prompt(&self, finding: &ThreatFinding) -> String {
        format!(
            r#"Generate a remediation suggestion for this API security finding:

Finding Type: {:?}
Severity: {:?}
Description: {}
Field Path: {}

Provide:
1. A clear, actionable remediation suggestion
2. A code example showing how to fix it (if applicable)
3. Confidence score (0.0-1.0)

Format your response as JSON:
{{
  "suggestion": "detailed suggestion text",
  "code_example": "example code or schema change",
  "confidence": 0.8
}}"#,
            finding.finding_type, finding.severity, finding.description,
            finding.field_path.as_deref().unwrap_or("N/A")
        )
    }

    /// Get system prompt
    fn get_system_prompt(&self) -> String {
        "You are an expert API security analyst specializing in contract security and threat remediation. Provide clear, actionable remediation suggestions with code examples when applicable.".to_string()
    }

    /// Calculate priority based on severity
    fn calculate_priority(&self, finding: &ThreatFinding) -> u32 {
        match finding.severity {
            super::types::ThreatLevel::Critical => 1,
            super::types::ThreatLevel::High => 2,
            super::types::ThreatLevel::Medium => 3,
            super::types::ThreatLevel::Low => 4,
        }
    }

    /// Generate basic remediation without AI
    fn generate_basic_remediations(&self, findings: &[ThreatFinding]) -> Vec<RemediationSuggestion> {
        findings
            .iter()
            .map(|f| self.generate_basic_remediation(f))
            .collect()
    }

    /// Generate a basic remediation for a finding
    fn generate_basic_remediation(&self, finding: &ThreatFinding) -> RemediationSuggestion {
        let (suggestion, code_example) = match finding.finding_type {
            super::types::ThreatCategory::UnboundedArrays => (
                "Add maxItems constraint to array schema to prevent DoS attacks".to_string(),
                Some(r#"{
  "type": "array",
  "items": {...},
  "maxItems": 100
}"#.to_string()),
            ),
            super::types::ThreatCategory::PiiExposure => (
                "Review field name and ensure PII is properly masked or removed from responses".to_string(),
                None,
            ),
            super::types::ThreatCategory::StackTraceLeakage => (
                "Sanitize error messages to remove stack traces and internal details".to_string(),
                Some(r#"{
  "error": {
    "message": "An error occurred",
    "code": "ERROR_CODE"
  }
}"#.to_string()),
            ),
            super::types::ThreatCategory::ExcessiveOptionalFields => (
                "Consider making more fields required or splitting into separate schemas".to_string(),
                None,
            ),
            _ => (
                format!("Address the {} issue in the API contract", finding.finding_type),
                None,
            ),
        };

        RemediationSuggestion {
            finding_id: format!("finding_{}", finding.field_path.as_deref().unwrap_or("unknown")),
            suggestion,
            code_example,
            confidence: 0.6,
            ai_generated: false,
            priority: self.calculate_priority(finding),
        }
    }
}

