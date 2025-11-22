//! Main threat analyzer
//!
//! This module orchestrates all threat detection components to provide
//! comprehensive security posture analysis.

use super::dos_analyzer::DosAnalyzer;
use super::error_analyzer::ErrorAnalyzer;
use super::pii_detector::PiiDetector;
use super::remediation_generator::RemediationGenerator;
use super::schema_analyzer::SchemaAnalyzer;
use super::types::{
    AggregationLevel, ThreatAssessment, ThreatCategory, ThreatFinding, ThreatLevel,
    ThreatModelingConfig,
};
use crate::openapi::OpenApiSpec;
use crate::Result;
use chrono::Utc;
use std::collections::HashMap;

/// Main threat analyzer
pub struct ThreatAnalyzer {
    /// PII detector
    pii_detector: PiiDetector,
    /// DoS analyzer
    dos_analyzer: DosAnalyzer,
    /// Error analyzer
    error_analyzer: ErrorAnalyzer,
    /// Schema analyzer
    schema_analyzer: SchemaAnalyzer,
    /// Remediation generator
    remediation_generator: Option<RemediationGenerator>,
    /// Configuration
    config: ThreatModelingConfig,
}

impl ThreatAnalyzer {
    /// Create a new threat analyzer
    pub fn new(config: ThreatModelingConfig) -> Result<Self> {
        let pii_detector = PiiDetector::new(config.pii_patterns.clone());
        let dos_analyzer = DosAnalyzer::default();
        let error_analyzer = ErrorAnalyzer::new(config.detect_error_leakage);
        let schema_analyzer = SchemaAnalyzer::new(config.max_optional_fields_threshold);

        let remediation_generator = if config.generate_remediations {
            // Note: In a real implementation, you'd get LLM config from somewhere
            // For now, we'll create it with defaults
            Some(RemediationGenerator::new(
                true,
                "openai".to_string(),
                "gpt-4".to_string(),
                None,
            )?)
        } else {
            None
        };

        Ok(Self {
            pii_detector,
            dos_analyzer,
            error_analyzer,
            schema_analyzer,
            remediation_generator,
            config,
        })
    }

    /// Analyze a contract for threats
    pub async fn analyze_contract(
        &self,
        spec: &OpenApiSpec,
        workspace_id: Option<String>,
        service_id: Option<String>,
        service_name: Option<String>,
        endpoint: Option<String>,
        method: Option<String>,
    ) -> Result<ThreatAssessment> {
        if !self.config.enabled {
            return Ok(ThreatAssessment {
                workspace_id,
                service_id,
                service_name,
                endpoint: endpoint.clone(),
                method: method.clone(),
                aggregation_level: self.determine_aggregation_level(endpoint.as_ref(), method.as_ref()),
                threat_level: ThreatLevel::Low,
                threat_score: 0.0,
                threat_categories: Vec::new(),
                findings: Vec::new(),
                remediation_suggestions: Vec::new(),
                assessed_at: Utc::now(),
            });
        }

        // Run all analyzers
        let mut all_findings = Vec::new();

        // PII detection
        all_findings.extend(self.pii_detector.detect_pii(spec));

        // DoS analysis
        all_findings.extend(self.dos_analyzer.analyze_dos_risks(spec));

        // Error analysis
        all_findings.extend(self.error_analyzer.analyze_errors(spec));

        // Schema analysis
        all_findings.extend(self.schema_analyzer.analyze_schemas(spec));

        // Generate remediations if enabled
        let remediation_suggestions = if let Some(ref generator) = self.remediation_generator {
            generator.generate_remediations(&all_findings).await.unwrap_or_default()
        } else {
            Vec::new()
        };

        // Calculate threat score and level
        let threat_score = self.calculate_threat_score(&all_findings);
        let threat_level = self.determine_threat_level(threat_score, &all_findings);

        // Extract unique threat categories
        let threat_categories: Vec<ThreatCategory> = all_findings
            .iter()
            .map(|f| f.finding_type.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        Ok(ThreatAssessment {
            workspace_id,
            service_id,
            service_name,
            endpoint: endpoint.clone(),
            method: method.clone(),
            aggregation_level: self.determine_aggregation_level(endpoint.as_ref(), method.as_ref()),
            threat_level,
            threat_score,
            threat_categories,
            findings: all_findings,
            remediation_suggestions,
            assessed_at: Utc::now(),
        })
    }

    /// Determine aggregation level
    fn determine_aggregation_level(
        &self,
        endpoint: Option<&String>,
        method: Option<&String>,
    ) -> AggregationLevel {
        if endpoint.is_some() && method.is_some() {
            AggregationLevel::Endpoint
        } else {
            AggregationLevel::Service
        }
    }

    /// Calculate overall threat score
    fn calculate_threat_score(&self, findings: &[ThreatFinding]) -> f64 {
        if findings.is_empty() {
            return 0.0;
        }

        let total_score: f64 = findings
            .iter()
            .map(|f| {
                let severity_score = match f.severity {
                    ThreatLevel::Critical => 1.0,
                    ThreatLevel::High => 0.75,
                    ThreatLevel::Medium => 0.5,
                    ThreatLevel::Low => 0.25,
                };
                severity_score * f.confidence
            })
            .sum();

        (total_score / findings.len() as f64).min(1.0)
    }

    /// Determine threat level from score and findings
    fn determine_threat_level(&self, score: f64, findings: &[ThreatFinding]) -> ThreatLevel {
        // Check for critical findings
        let has_critical = findings
            .iter()
            .any(|f| matches!(f.severity, ThreatLevel::Critical));

        if has_critical {
            return ThreatLevel::Critical;
        }

        // Use score-based determination
        if score >= 0.75 {
            ThreatLevel::High
        } else if score >= 0.5 {
            ThreatLevel::Medium
        } else if score >= 0.25 {
            ThreatLevel::Low
        } else {
            ThreatLevel::Low
        }
    }
}

impl Default for ThreatAnalyzer {
    fn default() -> Self {
        Self::new(ThreatModelingConfig::default()).unwrap()
    }
}
