//! Pillars: [Contracts][AI]
//!
//! AI-powered contract diff analysis
//!
//! This module provides intelligent contract diff analysis that compares front-end requests
//! against backend API contract specifications, detects mismatches, and generates AI-powered
//! recommendations and correction proposals.
//!
//! # Features
//!
//! - **Structural Diff Analysis**: Detects mismatches between requests and contract specs
//! - **AI-Powered Recommendations**: Uses LLM to generate contextual recommendations
//! - **Correction Proposals**: Generates JSON Patch files for schema corrections
//! - **Confidence Scoring**: Provides confidence scores for all suggestions
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use mockforge_core::ai_contract_diff::{
//!     ContractDiffAnalyzer, ContractDiffConfig, CapturedRequest,
//! };
//! use mockforge_core::openapi::OpenApiSpec;
//!
//! # async fn example() -> mockforge_core::Result<()> {
//! // Load contract specification
//! let spec = OpenApiSpec::from_file("api.yaml").await?;
//!
//! // Configure contract diff
//! let config = ContractDiffConfig {
//!     enabled: true,
//!     llm_provider: "openai".to_string(),
//!     llm_model: "gpt-4".to_string(),
//!     confidence_threshold: 0.5,
//!     ..Default::default()
//! };
//!
//! // Create analyzer
//! let analyzer = ContractDiffAnalyzer::new(config)?;
//!
//! // Capture a request
//! let request = CapturedRequest::new("POST", "/api/users", "browser_extension")
//!     .with_body(serde_json::json!({"name": "Alice", "email": "alice@example.com"}));
//!
//! // Analyze request against contract
//! let result = analyzer.analyze(request, &spec).await?;
//!
//! // Check results
//! if !result.matches {
//!     println!("Found {} mismatches", result.mismatches.len());
//!     for mismatch in &result.mismatches {
//!         println!("  - {}: {}", mismatch.path, mismatch.description);
//!     }
//!
//!     // Generate recommendations
//!     for recommendation in &result.recommendations {
//!         println!("  Recommendation: {}", recommendation.recommendation);
//!     }
//!
//!     // Generate correction proposals
//!     for correction in &result.corrections {
//!         println!("  Correction: {}", correction.description);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

pub mod confidence_scorer;
pub mod correction_proposer;
pub mod diff_analyzer;
pub mod recommendation_engine;
pub mod semantic_analyzer;
pub mod types;

// Re-export main types
pub use confidence_scorer::{ConfidenceScorer, ScoringContext};
pub use correction_proposer::CorrectionProposer;
pub use diff_analyzer::DiffAnalyzer;
pub use recommendation_engine::{RecommendationEngine, RequestContext};
pub use semantic_analyzer::{SemanticAnalyzer, SemanticChangeType, SemanticDriftResult};
pub use types::ConfidenceLevel;
pub use types::{
    CapturedRequest, ContractDiffConfig, ContractDiffResult, CorrectionProposal, DiffMetadata,
    Mismatch, MismatchSeverity, MismatchType, PatchOperation, Recommendation,
};

/// Main contract diff analyzer that orchestrates all components
pub struct ContractDiffAnalyzer {
    /// Diff analyzer for structural comparison
    diff_analyzer: DiffAnalyzer,

    /// Recommendation engine for AI-powered suggestions
    recommendation_engine: RecommendationEngine,

    /// Semantic analyzer for Layer 2 semantic drift detection
    semantic_analyzer: SemanticAnalyzer,

    /// Correction proposer for generating patches
    correction_proposer: CorrectionProposer,

    /// Configuration
    config: ContractDiffConfig,
}

impl ContractDiffAnalyzer {
    /// Create a new contract diff analyzer
    pub fn new(config: ContractDiffConfig) -> crate::Result<Self> {
        let diff_analyzer = DiffAnalyzer::new(config.clone());
        let recommendation_engine = RecommendationEngine::new(config.clone())?;
        let semantic_analyzer = SemanticAnalyzer::new(config.clone())?;
        let correction_proposer = CorrectionProposer;

        Ok(Self {
            diff_analyzer,
            recommendation_engine,
            semantic_analyzer,
            correction_proposer,
            config,
        })
    }

    /// Analyze a captured request against a contract specification
    pub async fn analyze(
        &self,
        request: &CapturedRequest,
        spec: &crate::openapi::OpenApiSpec,
    ) -> crate::Result<ContractDiffResult> {
        // Step 1: Perform structural diff analysis
        let mut result = self.diff_analyzer.analyze_request(request, spec).await?;

        // Step 2: Generate AI-powered recommendations if enabled
        if self.config.use_ai_recommendations && !result.mismatches.is_empty() {
            let mut request_context = RequestContext::new(&request.method, &request.path);
            if let Some(body) = &request.body {
                request_context = request_context.with_body(body.clone());
            }
            request_context =
                request_context.with_contract_format(&result.metadata.contract_format);

            let recommendations = self
                .recommendation_engine
                .generate_recommendations(&result.mismatches, &request_context)
                .await?;

            // Filter by confidence threshold
            result.recommendations = recommendations
                .into_iter()
                .filter(|r| r.confidence >= self.config.confidence_threshold)
                .collect();
        }

        // Step 3: Generate correction proposals if enabled
        if self.config.generate_corrections && !result.mismatches.is_empty() {
            result.corrections = CorrectionProposer::generate_proposals(
                &result.mismatches,
                &result.recommendations,
                spec,
            );

            // Filter by confidence threshold
            result.corrections.retain(|c| c.confidence >= self.config.confidence_threshold);
        }

        // Recalculate overall confidence with recommendations and corrections
        result.confidence = ConfidenceScorer::calculate_overall_confidence(&result.mismatches);

        Ok(result)
    }

    /// Compare two contract specifications and detect semantic drift
    ///
    /// This method performs Layer 1 (structural) and Layer 2 (semantic) analysis
    /// to detect both structural and meaning changes between contract versions.
    pub async fn compare_specs(
        &self,
        before_spec: &crate::openapi::OpenApiSpec,
        after_spec: &crate::openapi::OpenApiSpec,
        endpoint_path: &str,
        method: &str,
    ) -> crate::Result<Option<SemanticDriftResult>> {
        // Layer 2: Semantic analysis
        if self.config.semantic_analysis_enabled {
            let semantic_result = self
                .semantic_analyzer
                .analyze_semantic_drift(before_spec, after_spec, endpoint_path, method)
                .await?;

            // Filter by semantic confidence threshold
            if let Some(ref result) = semantic_result {
                if result.semantic_confidence >= self.config.semantic_confidence_threshold {
                    return Ok(semantic_result);
                }
            }
        }

        Ok(None)
    }

    /// Generate a JSON Patch file from correction proposals
    pub fn generate_patch_file(
        &self,
        corrections: &[CorrectionProposal],
        spec_version: &str,
    ) -> serde_json::Value {
        CorrectionProposer::generate_patch_file(corrections, spec_version)
    }

    /// Get configuration
    pub fn config(&self) -> &ContractDiffConfig {
        &self.config
    }
}
