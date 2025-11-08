//! Confidence scoring system for AI contract diff recommendations
//!
//! This module provides confidence scoring for AI-generated recommendations and corrections.
//! Confidence scores help users understand how reliable each suggestion is.

use super::types::ConfidenceLevel;
use super::types::{Mismatch, MismatchSeverity, Recommendation};
use std::collections::HashMap;

/// Confidence scorer for contract diff recommendations
pub struct ConfidenceScorer;

impl ConfidenceScorer {
    /// Calculate confidence score for a mismatch based on various factors
    ///
    /// Factors considered:
    /// - Mismatch severity (critical = lower confidence, info = higher confidence)
    /// - Type of mismatch (structural vs semantic)
    /// - Consistency across multiple requests
    /// - Schema clarity
    pub fn score_mismatch(mismatch: &Mismatch, context: &ScoringContext) -> f64 {
        let mut score = 0.5; // Base score

        // Adjust based on severity (critical issues are more certain)
        let severity_factor = match mismatch.severity {
            MismatchSeverity::Critical => 0.9,
            MismatchSeverity::High => 0.8,
            MismatchSeverity::Medium => 0.7,
            MismatchSeverity::Low => 0.6,
            MismatchSeverity::Info => 0.5,
        };
        score = (score + severity_factor) / 2.0;

        // Adjust based on mismatch type (structural mismatches are more certain)
        let type_factor = match mismatch.mismatch_type {
            super::types::MismatchType::MissingRequiredField => 0.95,
            super::types::MismatchType::TypeMismatch => 0.9,
            super::types::MismatchType::SchemaMismatch => 0.85,
            super::types::MismatchType::FormatMismatch => 0.8,
            super::types::MismatchType::ConstraintViolation => 0.75,
            super::types::MismatchType::UnexpectedField => 0.7,
            super::types::MismatchType::EndpointNotFound => 0.9,
            super::types::MismatchType::MethodNotAllowed => 0.9,
            super::types::MismatchType::HeaderMismatch => 0.7,
            super::types::MismatchType::QueryParamMismatch => 0.65,
        };
        score = (score + type_factor) / 2.0;

        // Adjust based on consistency (if seen multiple times, higher confidence)
        if context.occurrence_count > 1 {
            let consistency_boost = (context.occurrence_count as f64).min(10.0) / 10.0 * 0.2;
            score = (score + consistency_boost).min(1.0);
        }

        // Adjust based on schema clarity (if schema is well-defined, higher confidence)
        if context.schema_clarity > 0.7 {
            score = (score + 0.1).min(1.0);
        } else if context.schema_clarity < 0.3 {
            score = (score - 0.1).max(0.0);
        }

        // Ensure score is in valid range
        score.max(0.0).min(1.0)
    }

    /// Calculate confidence score for a recommendation
    ///
    /// Factors considered:
    /// - Base mismatch confidence
    /// - Recommendation clarity
    /// - Example quality (if provided)
    /// - Reasoning quality
    pub fn score_recommendation(recommendation: &Recommendation, mismatch_confidence: f64) -> f64 {
        let mut score = mismatch_confidence;

        // Boost if recommendation has clear suggested fix
        if recommendation.suggested_fix.is_some() {
            score = (score + 0.1).min(1.0);
        }

        // Boost if recommendation has reasoning
        if recommendation.reasoning.is_some() {
            score = (score + 0.05).min(1.0);
        }

        // Boost if recommendation has example
        if recommendation.example.is_some() {
            score = (score + 0.05).min(1.0);
        }

        // Use the recommendation's own confidence if it's lower (more conservative)
        score.min(recommendation.confidence)
    }

    /// Calculate confidence score for a correction proposal
    ///
    /// Factors considered:
    /// - Base mismatch confidence
    /// - Patch operation validity
    /// - Before/after comparison clarity
    /// - Affected endpoints count
    pub fn score_correction(
        correction: &super::types::CorrectionProposal,
        mismatch_confidence: f64,
    ) -> f64 {
        let mut score = mismatch_confidence;

        // Boost if correction has before/after comparison
        if correction.before.is_some() && correction.after.is_some() {
            score = (score + 0.1).min(1.0);
        }

        // Boost if correction has reasoning
        if correction.reasoning.is_some() {
            score = (score + 0.05).min(1.0);
        }

        // Slight penalty if affects many endpoints (more risky)
        if correction.affected_endpoints.len() > 5 {
            score = (score - 0.05).max(0.0);
        }

        // Use the correction's own confidence if it's lower (more conservative)
        score.min(correction.confidence)
    }

    /// Get confidence level category from score
    pub fn get_confidence_level(score: f64) -> ConfidenceLevel {
        ConfidenceLevel::from_score(score)
    }

    /// Calculate overall confidence for a diff result
    ///
    /// Takes the average of all mismatch confidences, weighted by severity
    pub fn calculate_overall_confidence(mismatches: &[Mismatch]) -> f64 {
        if mismatches.is_empty() {
            return 1.0; // No mismatches = perfect match
        }

        let mut total_weighted_score = 0.0;
        let mut total_weight = 0.0;

        for mismatch in mismatches {
            let weight = match mismatch.severity {
                MismatchSeverity::Critical => 5.0,
                MismatchSeverity::High => 4.0,
                MismatchSeverity::Medium => 3.0,
                MismatchSeverity::Low => 2.0,
                MismatchSeverity::Info => 1.0,
            };

            total_weighted_score += mismatch.confidence * weight;
            total_weight += weight;
        }

        if total_weight > 0.0 {
            total_weighted_score / total_weight
        } else {
            0.0
        }
    }
}

/// Context for confidence scoring
#[derive(Debug, Clone)]
pub struct ScoringContext {
    /// Number of times this mismatch has been observed
    pub occurrence_count: usize,

    /// Schema clarity score (0.0-1.0) - how well-defined the schema is
    pub schema_clarity: f64,

    /// Additional context factors
    pub factors: HashMap<String, f64>,
}

impl Default for ScoringContext {
    fn default() -> Self {
        Self {
            occurrence_count: 1,
            schema_clarity: 0.5,
            factors: HashMap::new(),
        }
    }
}

impl ScoringContext {
    /// Create a new scoring context
    pub fn new(occurrence_count: usize, schema_clarity: f64) -> Self {
        Self {
            occurrence_count,
            schema_clarity,
            factors: HashMap::new(),
        }
    }

    /// Add a custom factor for scoring
    pub fn with_factor(mut self, key: impl Into<String>, value: f64) -> Self {
        self.factors.insert(key.into(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai_contract_diff::types::{Mismatch, MismatchSeverity, MismatchType};

    #[test]
    fn test_confidence_level_from_score() {
        assert_eq!(ConfidenceLevel::from_score(0.9), ConfidenceLevel::High);
        assert_eq!(ConfidenceLevel::from_score(0.7), ConfidenceLevel::Medium);
        assert_eq!(ConfidenceLevel::from_score(0.3), ConfidenceLevel::Low);
    }

    #[test]
    fn test_score_mismatch() {
        let mismatch = Mismatch {
            mismatch_type: MismatchType::MissingRequiredField,
            path: "/user/email".to_string(),
            method: None,
            expected: Some("string".to_string()),
            actual: None,
            description: "Missing required field".to_string(),
            severity: MismatchSeverity::Critical,
            confidence: 0.0, // Will be calculated
            context: HashMap::new(),
        };

        let context = ScoringContext::new(1, 0.8);
        let score = ConfidenceScorer::score_mismatch(&mismatch, &context);

        assert!(score >= 0.0 && score <= 1.0);
        assert!(score > 0.7); // Critical missing field should have high confidence
    }

    #[test]
    fn test_calculate_overall_confidence() {
        let mismatches = vec![
            Mismatch {
                mismatch_type: MismatchType::TypeMismatch,
                path: "/user/age".to_string(),
                method: None,
                expected: Some("integer".to_string()),
                actual: Some("string".to_string()),
                description: "Type mismatch".to_string(),
                severity: MismatchSeverity::High,
                confidence: 0.9,
                context: HashMap::new(),
            },
            Mismatch {
                mismatch_type: MismatchType::UnexpectedField,
                path: "/user/extra".to_string(),
                method: None,
                expected: None,
                actual: Some("value".to_string()),
                description: "Unexpected field".to_string(),
                severity: MismatchSeverity::Low,
                confidence: 0.6,
                context: HashMap::new(),
            },
        ];

        let overall = ConfidenceScorer::calculate_overall_confidence(&mismatches);
        assert!(overall >= 0.0 && overall <= 1.0);
        assert!(overall > 0.6); // Should be weighted average
    }

    #[test]
    fn test_empty_mismatches_confidence() {
        let mismatches = vec![];
        let overall = ConfidenceScorer::calculate_overall_confidence(&mismatches);
        assert_eq!(overall, 1.0); // No mismatches = perfect match
    }
}
