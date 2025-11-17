//! Consumer breaking change detector
//!
//! This module provides functionality for detecting breaking changes that affect
//! specific consumers based on their actual usage patterns.

use crate::ai_contract_diff::{ContractDiffResult, Mismatch, MismatchType};
use crate::consumer_contracts::types::{ConsumerUsage, ConsumerViolation};
use crate::consumer_contracts::usage_recorder::UsageRecorder;
use std::sync::Arc;
use uuid::Uuid;

/// Detector for consumer-specific breaking changes
#[derive(Debug, Clone)]
pub struct ConsumerBreakingChangeDetector {
    usage_recorder: Arc<UsageRecorder>,
}

impl ConsumerBreakingChangeDetector {
    /// Create a new consumer breaking change detector
    pub fn new(usage_recorder: Arc<UsageRecorder>) -> Self {
        Self { usage_recorder }
    }

    /// Detect violations for a consumer based on contract diff result
    pub async fn detect_violations(
        &self,
        consumer_id: &str,
        endpoint: &str,
        method: &str,
        diff_result: &ContractDiffResult,
        incident_id: Option<String>,
    ) -> Vec<ConsumerViolation> {
        // Get consumer usage for this endpoint
        let usage = self.usage_recorder.get_endpoint_usage(consumer_id, endpoint, method).await;

        if usage.is_none() {
            // No usage recorded, can't detect violations
            return vec![];
        }

        let usage = usage.unwrap();
        let mut violations = Vec::new();

        // Check each mismatch to see if it affects fields the consumer uses
        for mismatch in &diff_result.mismatches {
            if self.is_violation_for_consumer(&usage, mismatch) {
                let violated_fields = self.extract_violated_fields(&usage, mismatch);

                if !violated_fields.is_empty() {
                    violations.push(ConsumerViolation {
                        id: Uuid::new_v4().to_string(),
                        consumer_id: consumer_id.to_string(),
                        incident_id: incident_id.clone(),
                        endpoint: endpoint.to_string(),
                        method: method.to_string(),
                        violated_fields,
                        detected_at: chrono::Utc::now().timestamp(),
                    });
                }
            }
        }

        violations
    }

    /// Check if a mismatch is a violation for a specific consumer
    fn is_violation_for_consumer(&self, usage: &ConsumerUsage, mismatch: &Mismatch) -> bool {
        // Check if the mismatch affects fields the consumer uses
        match mismatch.mismatch_type {
            MismatchType::MissingRequiredField => {
                // If a required field is missing and the consumer uses it, it's a violation
                Self::field_in_usage(&mismatch.path, &usage.fields_used)
            }
            MismatchType::TypeMismatch => {
                // Type mismatch affects the consumer if they use this field
                Self::field_in_usage(&mismatch.path, &usage.fields_used)
            }
            MismatchType::UnexpectedField => {
                // Unexpected fields are usually not violations (they're additions)
                false
            }
            MismatchType::FormatMismatch => {
                // Format mismatch affects the consumer if they use this field
                Self::field_in_usage(&mismatch.path, &usage.fields_used)
            }
            MismatchType::ConstraintViolation => {
                // Constraint violation affects the consumer if they use this field
                Self::field_in_usage(&mismatch.path, &usage.fields_used)
            }
            _ => {
                // Other mismatch types might affect the consumer
                // For now, check if path matches any used field
                Self::field_in_usage(&mismatch.path, &usage.fields_used)
            }
        }
    }

    /// Extract violated fields from a mismatch
    fn extract_violated_fields(&self, usage: &ConsumerUsage, mismatch: &Mismatch) -> Vec<String> {
        let mut violated = Vec::new();

        // Check if the mismatch path matches any used field
        if Self::field_in_usage(&mismatch.path, &usage.fields_used) {
            violated.push(mismatch.path.clone());

            // Also check for nested fields
            for field in &usage.fields_used {
                if field.starts_with(&mismatch.path) {
                    violated.push(field.clone());
                }
            }
        }

        violated
    }

    /// Check if a field path is in the usage list
    fn field_in_usage(field_path: &str, fields_used: &[String]) -> bool {
        // Exact match
        if fields_used.contains(&field_path.to_string()) {
            return true;
        }

        // Check if any used field is a child of the mismatch path
        for used_field in fields_used {
            if used_field.starts_with(field_path) {
                return true;
            }
        }

        // Check if mismatch path is a parent of any used field
        for used_field in fields_used {
            if field_path.starts_with(used_field) {
                return true;
            }
        }

        false
    }
}
