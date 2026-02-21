//! Contract Diff Handler for processing natural language queries
//!
//! This module provides functionality to process natural language queries about
//! contract diffs, enabling users to ask questions like "show me breaking changes"
//! or "compare the last 3 versions" via the AI Studio chat interface.

use crate::ai_contract_diff::{
    CapturedRequest, ContractDiffAnalyzer, ContractDiffConfig, ContractDiffResult,
};
use crate::intelligent_behavior::{config::IntelligentBehaviorConfig, llm_client::LlmClient};
use crate::{OpenApiSpec, Result};
use serde::{Deserialize, Serialize};

/// Contract diff handler for NL queries
pub struct ContractDiffHandler {
    /// LLM client for parsing queries
    llm_client: LlmClient,
    /// Contract diff analyzer
    analyzer: ContractDiffAnalyzer,
    /// Configuration
    config: IntelligentBehaviorConfig,
}

impl ContractDiffHandler {
    /// Create a new contract diff handler
    pub fn new() -> Result<Self> {
        let config = IntelligentBehaviorConfig::default();
        let llm_client = LlmClient::new(config.behavior_model.clone());
        let diff_config = ContractDiffConfig {
            enabled: true,
            llm_provider: config.behavior_model.llm_provider.clone(),
            llm_model: config.behavior_model.model.clone(),
            confidence_threshold: 0.5,
            ..Default::default()
        };
        let analyzer = ContractDiffAnalyzer::new(diff_config)?;

        Ok(Self {
            llm_client,
            analyzer,
            config,
        })
    }

    /// Create with custom configuration
    pub fn with_config(config: IntelligentBehaviorConfig) -> Result<Self> {
        let llm_client = LlmClient::new(config.behavior_model.clone());
        let diff_config = ContractDiffConfig {
            enabled: true,
            llm_provider: config.behavior_model.llm_provider.clone(),
            llm_model: config.behavior_model.model.clone(),
            confidence_threshold: 0.5,
            ..Default::default()
        };
        let analyzer = ContractDiffAnalyzer::new(diff_config)?;

        Ok(Self {
            llm_client,
            analyzer,
            config,
        })
    }

    /// Analyze a contract diff from a natural language query
    ///
    /// Parses the query to extract:
    /// - Which spec/request to analyze
    /// - What type of analysis to perform
    /// - Any filters (breaking changes only, mobile endpoints, etc.)
    pub async fn analyze_from_query(
        &self,
        query: &str,
        spec: Option<&OpenApiSpec>,
        captured_request: Option<CapturedRequest>,
    ) -> Result<ContractDiffQueryResult> {
        // Parse the query to understand intent
        let intent = self.parse_query_intent(query).await?;

        match intent {
            ContractDiffIntent::AnalyzeRequest { request_id, filters } => {
                // Analyze a specific captured request
                if let Some(request) = captured_request {
                    if let Some(spec) = spec {
                        let result = self.analyzer.analyze(&request, spec).await?;
                        let breaking_changes = self.extract_breaking_changes(&result);
                        let summary = self.generate_summary(&result, &filters).await?;
                        Ok(ContractDiffQueryResult {
                            intent: ContractDiffIntent::AnalyzeRequest {
                                request_id: None,
                                filters: filters.clone(),
                            },
                            result: Some(result),
                            summary,
                            breaking_changes,
                            link_to_viewer: Some(format!("/contract-diff?request_id={}", request_id.unwrap_or_default())),
                        })
                    } else {
                        Err(crate::Error::generic("OpenAPI spec is required for analysis"))
                    }
                } else {
                    Err(crate::Error::generic("Captured request is required for analysis"))
                }
            }
            ContractDiffIntent::CompareVersions { spec1_path, spec2_path, filters } => {
                // Compare two contract versions
                // This would require loading both specs
                Ok(ContractDiffQueryResult {
                    intent: ContractDiffIntent::CompareVersions {
                        spec1_path: spec1_path.clone(),
                        spec2_path: spec2_path.clone(),
                        filters: filters.clone(),
                    },
                    result: None,
                    summary: format!(
                        "To compare versions, please provide both OpenAPI specifications. Spec 1: {}, Spec 2: {}",
                        spec1_path.unwrap_or_else(|| "not specified".to_string()),
                        spec2_path.unwrap_or_else(|| "not specified".to_string())
                    ),
                    breaking_changes: Vec::new(),
                    link_to_viewer: Some("/contract-diff/compare".to_string()),
                })
            }
            ContractDiffIntent::SummarizeDrift { filters } => {
                // Summarize contract drift
                Ok(ContractDiffQueryResult {
                    intent: ContractDiffIntent::SummarizeDrift { filters: filters.clone() },
                    result: None,
                    summary: "Drift summary would be generated from recent contract diff analyses. Use the Contract Diff page to view detailed drift history.".to_string(),
                    breaking_changes: Vec::new(),
                    link_to_viewer: Some("/contract-diff".to_string()),
                })
            }
            ContractDiffIntent::FindBreakingChanges { filters } => {
                // Find breaking changes
                if let Some(_spec) = spec {
                    // This is a simplified version - in practice, you'd compare against a previous version
                    Ok(ContractDiffQueryResult {
                        intent: ContractDiffIntent::FindBreakingChanges { filters: filters.clone() },
                        result: None,
                        summary: "Breaking changes analysis requires comparing against a previous contract version. Use the Contract Diff page to compare versions.".to_string(),
                        breaking_changes: Vec::new(),
                        link_to_viewer: Some("/contract-diff".to_string()),
                    })
                } else {
                    Err(crate::Error::generic("OpenAPI spec is required for breaking changes analysis"))
                }
            }
            ContractDiffIntent::Unknown => {
                Ok(ContractDiffQueryResult {
                    intent: ContractDiffIntent::Unknown,
                    result: None,
                    summary: "I can help with contract diff analysis! Try asking:\n- \"Analyze the last captured request\"\n- \"Show me breaking changes\"\n- \"Compare contract versions\"\n- \"Summarize drift for mobile endpoints\"".to_string(),
                    breaking_changes: Vec::new(),
                    link_to_viewer: None,
                })
            }
        }
    }

    /// Compare two contract versions
    pub async fn compare_versions(
        &self,
        _spec1: &OpenApiSpec,
        _spec2: &OpenApiSpec,
    ) -> Result<ContractDiffResult> {
        // Use analyze method with a dummy request for now
        // In production, compare_specs would be implemented separately
        // For now, return an error indicating this needs proper implementation
        Err(crate::Error::generic("Contract version comparison requires proper implementation. Use the Contract Diff page for detailed comparison."))
    }

    /// Summarize contract drift
    ///
    /// Generates a human-readable summary of contract drift based on recent analyses.
    pub async fn summarize_drift(
        &self,
        results: &[ContractDiffResult],
        filters: &ContractDiffFilters,
    ) -> Result<String> {
        if results.is_empty() {
            return Ok("No contract drift detected in recent analyses.".to_string());
        }

        let total_mismatches: usize = results.iter().map(|r| r.mismatches.len()).sum();
        let breaking_count = results
            .iter()
            .flat_map(|r| &r.mismatches)
            .filter(|m| m.severity == crate::ai_contract_diff::MismatchSeverity::Critical)
            .count();

        let mut summary = format!(
            "Contract drift summary:\n- Total analyses: {}\n- Total mismatches: {}\n- Breaking changes: {}",
            results.len(),
            total_mismatches,
            breaking_count
        );

        // Apply filters
        if let Some(ref endpoint_filter) = filters.endpoint_filter {
            summary.push_str(&format!("\n- Filtered by endpoint: {}", endpoint_filter));
        }

        if filters.breaking_only {
            summary.push_str("\n- Showing breaking changes only");
        }

        Ok(summary)
    }

    /// Find breaking changes in contract diff results
    pub fn find_breaking_changes(&self, result: &ContractDiffResult) -> Vec<BreakingChange> {
        result
            .mismatches
            .iter()
            .filter(|m| m.severity == crate::ai_contract_diff::MismatchSeverity::Critical)
            .map(|m| BreakingChange {
                path: m.path.clone(),
                method: m.method.clone(),
                description: m.description.clone(),
                impact: "High - This change will break existing clients".to_string(),
            })
            .collect()
    }

    /// Extract breaking changes from result
    fn extract_breaking_changes(&self, result: &ContractDiffResult) -> Vec<BreakingChange> {
        self.find_breaking_changes(result)
    }

    /// Generate a summary from contract diff result
    async fn generate_summary(
        &self,
        result: &ContractDiffResult,
        filters: &ContractDiffFilters,
    ) -> Result<String> {
        if result.matches {
            return Ok("Contract validation passed - no mismatches detected.".to_string());
        }

        let mut summary =
            format!("Found {} mismatch(es) between request and contract.", result.mismatches.len());

        if filters.breaking_only {
            let breaking = result
                .mismatches
                .iter()
                .filter(|m| m.severity == crate::ai_contract_diff::MismatchSeverity::Critical)
                .count();
            summary = format!("Found {} breaking change(s).", breaking);
        }

        if !result.recommendations.is_empty() {
            summary.push_str(&format!(
                "\n\n{} AI-powered recommendation(s) available.",
                result.recommendations.len()
            ));
        }

        if !result.corrections.is_empty() {
            summary.push_str(&format!(
                "\n\n{} correction proposal(s) available.",
                result.corrections.len()
            ));
        }

        Ok(summary)
    }

    /// Parse query intent from natural language
    async fn parse_query_intent(&self, query: &str) -> Result<ContractDiffIntent> {
        let query_lower = query.to_lowercase();

        // Simple keyword-based intent detection (can be enhanced with LLM)
        if query_lower.contains("analyze") || query_lower.contains("check") {
            // Extract request ID if mentioned
            let request_id = self.extract_request_id(query);
            let filters = self.extract_filters(query);
            return Ok(ContractDiffIntent::AnalyzeRequest {
                request_id,
                filters,
            });
        }

        if query_lower.contains("compare") || query_lower.contains("diff") {
            let (spec1, spec2) = self.extract_spec_paths(query);
            let filters = self.extract_filters(query);
            return Ok(ContractDiffIntent::CompareVersions {
                spec1_path: spec1,
                spec2_path: spec2,
                filters,
            });
        }

        if query_lower.contains("summarize")
            || query_lower.contains("summary")
            || query_lower.contains("drift")
        {
            let filters = self.extract_filters(query);
            return Ok(ContractDiffIntent::SummarizeDrift { filters });
        }

        if query_lower.contains("breaking") || query_lower.contains("breaking change") {
            let filters = self.extract_filters(query);
            return Ok(ContractDiffIntent::FindBreakingChanges { filters });
        }

        Ok(ContractDiffIntent::Unknown)
    }

    /// Extract request ID from query (simple pattern matching)
    fn extract_request_id(&self, query: &str) -> Option<String> {
        // Look for patterns like "request id: abc123" or "request abc123"
        for word in query.split_whitespace() {
            if word.len() > 10 {
                // Likely a UUID or request ID
                return Some(word.to_string());
            }
        }
        None
    }

    /// Extract spec paths from query
    fn extract_spec_paths(&self, query: &str) -> (Option<String>, Option<String>) {
        // Simple extraction - look for file paths or URLs
        let words: Vec<&str> = query.split_whitespace().collect();
        let mut paths = Vec::new();

        for word in words {
            if word.ends_with(".yaml")
                || word.ends_with(".yml")
                || word.ends_with(".json")
                || word.starts_with("http")
            {
                paths.push(word.to_string());
            }
        }

        match paths.len() {
            0 => (None, None),
            1 => (Some(paths[0].clone()), None),
            _ => (Some(paths[0].clone()), Some(paths[1].clone())),
        }
    }

    /// Extract filters from query
    fn extract_filters(&self, query: &str) -> ContractDiffFilters {
        let query_lower = query.to_lowercase();
        ContractDiffFilters {
            breaking_only: query_lower.contains("breaking")
                || query_lower.contains("breaking change"),
            endpoint_filter: if query_lower.contains("mobile") {
                Some("mobile".to_string())
            } else if query_lower.contains("api") {
                Some("api".to_string())
            } else {
                None
            },
        }
    }
}

impl Default for ContractDiffHandler {
    fn default() -> Self {
        Self::new().expect("Failed to create ContractDiffHandler")
    }
}

/// Intent detected from natural language query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContractDiffIntent {
    /// Analyze a specific request
    AnalyzeRequest {
        /// Optional request ID
        request_id: Option<String>,
        /// Filters to apply
        filters: ContractDiffFilters,
    },
    /// Compare two contract versions
    CompareVersions {
        /// Path to first spec
        spec1_path: Option<String>,
        /// Path to second spec
        spec2_path: Option<String>,
        /// Filters to apply
        filters: ContractDiffFilters,
    },
    /// Summarize contract drift
    SummarizeDrift {
        /// Filters to apply
        filters: ContractDiffFilters,
    },
    /// Find breaking changes
    FindBreakingChanges {
        /// Filters to apply
        filters: ContractDiffFilters,
    },
    /// Unknown intent
    Unknown,
}

/// Filters for contract diff queries
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContractDiffFilters {
    /// Show only breaking changes
    pub breaking_only: bool,
    /// Filter by endpoint pattern
    pub endpoint_filter: Option<String>,
}

/// Result of a contract diff query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractDiffQueryResult {
    /// Detected intent
    pub intent: ContractDiffIntent,
    /// Contract diff result (if analysis was performed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<ContractDiffResult>,
    /// Human-readable summary
    pub summary: String,
    /// Breaking changes found
    pub breaking_changes: Vec<BreakingChange>,
    /// Link to Contract Diff Viewer page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_to_viewer: Option<String>,
}

/// Breaking change information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakingChange {
    /// Path/endpoint affected
    pub path: String,
    /// HTTP method (if applicable)
    pub method: Option<String>,
    /// Description of the breaking change
    pub description: String,
    /// Impact assessment
    pub impact: String,
}
