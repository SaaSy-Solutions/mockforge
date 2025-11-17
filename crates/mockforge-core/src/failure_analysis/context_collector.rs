//! Failure context collector
//!
//! Collects comprehensive context about a request failure, including
//! active configurations, rules, and execution details.

use crate::Result;
use chrono::Utc;
use serde_json::Value;
use std::collections::HashMap;

use super::types::*;

/// Collector for failure context
#[derive(Debug)]
pub struct FailureContextCollector;

impl FailureContextCollector {
    /// Create a new failure context collector
    pub fn new() -> Self {
        Self
    }

    /// Collect failure context from request execution details
    ///
    /// This is a placeholder implementation. In a real implementation,
    /// this would collect context from:
    /// - Request/response details
    /// - Active chaos configurations
    /// - Consistency rules
    /// - Contract validation results
    /// - Behavioral rules/personas
    /// - Hook execution results
    pub fn collect_context(
        &self,
        method: &str,
        path: &str,
        status_code: Option<u16>,
        error_message: Option<String>,
    ) -> Result<FailureContext> {
        // Build request details
        let request = RequestDetails {
            method: method.to_string(),
            path: path.to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
        };

        // Build response details if status code is available
        let response = status_code.map(|code| ResponseDetails {
            status_code: code,
            headers: HashMap::new(),
            body: None,
            duration_ms: None,
        });

        // For now, return a basic context structure
        // In a full implementation, this would collect from:
        // - Chaos config registry
        // - Consistency rule engine
        // - Contract validation system
        // - Behavioral rule system
        // - Hook execution tracker
        Ok(FailureContext {
            request,
            response,
            chaos_configs: Vec::new(),
            consistency_rules: Vec::new(),
            contract_validation: None,
            behavioral_rules: Vec::new(),
            hook_results: Vec::new(),
            error_message,
            timestamp: Utc::now(),
        })
    }

    /// Collect context with additional details
    pub fn collect_context_with_details(
        &self,
        method: &str,
        path: &str,
        headers: HashMap<String, String>,
        query_params: HashMap<String, String>,
        body: Option<Value>,
        status_code: Option<u16>,
        response_headers: HashMap<String, String>,
        response_body: Option<Value>,
        duration_ms: Option<u64>,
        error_message: Option<String>,
        chaos_configs: Vec<ChaosConfigInfo>,
        consistency_rules: Vec<ConsistencyRuleInfo>,
        contract_validation: Option<ContractValidationInfo>,
        behavioral_rules: Vec<BehavioralRuleInfo>,
        hook_results: Vec<HookExecutionInfo>,
    ) -> Result<FailureContext> {
        let request = RequestDetails {
            method: method.to_string(),
            path: path.to_string(),
            headers,
            query_params,
            body,
        };

        let response = status_code.map(|code| ResponseDetails {
            status_code: code,
            headers: response_headers,
            body: response_body,
            duration_ms,
        });

        Ok(FailureContext {
            request,
            response,
            chaos_configs,
            consistency_rules,
            contract_validation,
            behavioral_rules,
            hook_results,
            error_message,
            timestamp: Utc::now(),
        })
    }
}

impl Default for FailureContextCollector {
    fn default() -> Self {
        Self::new()
    }
}

