//! "What-If" Failure Designer
//!
//! This module provides a UI-friendly way to design failure scenarios with conditional rules.
//! Users can specify "break all webhooks for 10% of users on Chrome only" and the system
//! generates the appropriate chaos rules and hooks.

use crate::config::{ChaosConfig, FaultInjectionConfig, LatencyConfig};
use crate::scenarios::ChaosScenario;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Failure design rule
///
/// Specifies a failure scenario with target conditions and failure type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureDesignRule {
    /// Rule name
    pub name: String,
    /// Target specification (endpoints, user agents, IP ranges, etc.)
    pub target: FailureTarget,
    /// Type of failure to inject
    pub failure_type: FailureType,
    /// Additional conditions for matching
    #[serde(default)]
    pub conditions: Vec<FailureCondition>,
    /// Probability/percentage of requests to affect (0.0 to 1.0)
    pub probability: f64,
    /// Rule description
    #[serde(default)]
    pub description: Option<String>,
}

/// Target specification for failure injection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureTarget {
    /// Endpoint patterns to match (supports wildcards)
    #[serde(default)]
    pub endpoints: Vec<String>,
    /// User agent patterns (regex patterns)
    #[serde(default)]
    pub user_agents: Option<Vec<String>>,
    /// IP address ranges (CIDR notation or specific IPs)
    #[serde(default)]
    pub ip_ranges: Option<Vec<String>>,
    /// Header matching rules
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,
    /// HTTP methods to match
    #[serde(default)]
    pub methods: Option<Vec<String>>,
}

/// Type of failure to inject
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FailureType {
    /// Webhook failure - break webhook execution
    WebhookFailure {
        /// Webhook URL pattern to match (supports wildcards)
        webhook_pattern: String,
    },
    /// HTTP status code injection
    StatusCode {
        /// Status code to return
        code: u16,
    },
    /// Latency injection
    Latency {
        /// Delay in milliseconds
        delay_ms: u64,
    },
    /// Timeout error
    Timeout {
        /// Timeout duration in milliseconds
        timeout_ms: u64,
    },
    /// Connection error
    ConnectionError,
    /// Partial response (incomplete data)
    PartialResponse {
        /// Percentage of response to truncate (0.0 to 1.0)
        truncate_percentage: f64,
    },
}

/// Condition for matching requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureCondition {
    /// Condition type
    pub condition_type: ConditionType,
    /// Field to check (e.g., "header.User-Agent", "query.param", "body.field")
    pub field: String,
    /// Operator for comparison
    pub operator: ConditionOperator,
    /// Value to compare against
    pub value: Value,
}

/// Type of condition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionType {
    /// Match header value
    Header,
    /// Match query parameter
    Query,
    /// Match request body field
    Body,
    /// Match path parameter
    Path,
}

/// Condition operator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConditionOperator {
    /// Equals
    Equals,
    /// Not equals
    NotEquals,
    /// Contains (for strings)
    Contains,
    /// Matches regex
    Matches,
    /// Greater than (for numbers)
    GreaterThan,
    /// Less than (for numbers)
    LessThan,
}

/// Failure designer
///
/// Converts failure design rules into chaos configurations and hooks.
pub struct FailureDesigner;

impl FailureDesigner {
    /// Create a new failure designer
    pub fn new() -> Self {
        Self
    }

    /// Validate a failure design rule
    pub fn validate_rule(&self, rule: &FailureDesignRule) -> Result<(), String> {
        // Validate probability
        if rule.probability < 0.0 || rule.probability > 1.0 {
            return Err("Probability must be between 0.0 and 1.0".to_string());
        }

        // Validate target
        if rule.target.endpoints.is_empty() {
            return Err("At least one endpoint must be specified".to_string());
        }

        // Validate failure type
        match &rule.failure_type {
            FailureType::WebhookFailure { webhook_pattern } => {
                if webhook_pattern.is_empty() {
                    return Err("Webhook pattern cannot be empty".to_string());
                }
            }
            FailureType::StatusCode { code } => {
                if *code < 100 || *code > 599 {
                    return Err("Status code must be between 100 and 599".to_string());
                }
            }
            FailureType::Latency { delay_ms } => {
                if *delay_ms == 0 {
                    return Err("Delay must be greater than 0".to_string());
                }
            }
            FailureType::Timeout { timeout_ms } => {
                if *timeout_ms == 0 {
                    return Err("Timeout must be greater than 0".to_string());
                }
            }
            FailureType::PartialResponse {
                truncate_percentage,
            } => {
                if *truncate_percentage < 0.0 || *truncate_percentage > 1.0 {
                    return Err("Truncate percentage must be between 0.0 and 1.0".to_string());
                }
            }
            FailureType::ConnectionError => {
                // No validation needed
            }
        }

        Ok(())
    }

    /// Convert a failure design rule to a chaos scenario
    pub fn rule_to_scenario(&self, rule: &FailureDesignRule) -> Result<ChaosScenario, String> {
        // Validate rule first
        self.validate_rule(rule)?;

        // Create chaos config based on failure type
        let chaos_config = match &rule.failure_type {
            FailureType::StatusCode { code } => {
                let fault_config = FaultInjectionConfig {
                    enabled: true,
                    http_errors: vec![*code],
                    http_error_probability: rule.probability,
                    connection_errors: false,
                    connection_error_probability: 0.0,
                    timeout_errors: false,
                    timeout_ms: 5000,
                    timeout_probability: 0.0,
                    partial_responses: false,
                    partial_response_probability: 0.0,
                    payload_corruption: false,
                    payload_corruption_probability: 0.0,
                    corruption_type: crate::config::CorruptionType::None,
                    error_pattern: None,
                    mockai_enabled: false,
                };

                ChaosConfig {
                    enabled: true,
                    latency: None,
                    fault_injection: Some(fault_config),
                    rate_limit: None,
                    traffic_shaping: None,
                    circuit_breaker: None,
                    bulkhead: None,
                }
            }
            FailureType::Latency { delay_ms } => {
                let latency_config = LatencyConfig {
                    enabled: true,
                    fixed_delay_ms: Some(*delay_ms),
                    random_delay_range_ms: None,
                    jitter_percent: 0.0,
                    probability: rule.probability,
                };

                ChaosConfig {
                    enabled: true,
                    latency: Some(latency_config),
                    fault_injection: None,
                    rate_limit: None,
                    traffic_shaping: None,
                    circuit_breaker: None,
                    bulkhead: None,
                }
            }
            FailureType::Timeout { timeout_ms } => {
                let fault_config = FaultInjectionConfig {
                    enabled: true,
                    http_errors: vec![],
                    http_error_probability: 0.0,
                    connection_errors: false,
                    connection_error_probability: 0.0,
                    timeout_errors: true,
                    timeout_ms: *timeout_ms,
                    timeout_probability: rule.probability,
                    partial_responses: false,
                    partial_response_probability: 0.0,
                    payload_corruption: false,
                    payload_corruption_probability: 0.0,
                    corruption_type: crate::config::CorruptionType::None,
                    error_pattern: None,
                    mockai_enabled: false,
                };

                ChaosConfig {
                    enabled: true,
                    latency: None,
                    fault_injection: Some(fault_config),
                    rate_limit: None,
                    traffic_shaping: None,
                    circuit_breaker: None,
                    bulkhead: None,
                }
            }
            FailureType::ConnectionError => {
                let fault_config = FaultInjectionConfig {
                    enabled: true,
                    http_errors: vec![],
                    http_error_probability: 0.0,
                    connection_errors: true,
                    connection_error_probability: rule.probability,
                    timeout_errors: false,
                    timeout_ms: 5000,
                    timeout_probability: 0.0,
                    partial_responses: false,
                    partial_response_probability: 0.0,
                    payload_corruption: false,
                    payload_corruption_probability: 0.0,
                    corruption_type: crate::config::CorruptionType::None,
                    error_pattern: None,
                    mockai_enabled: false,
                };

                ChaosConfig {
                    enabled: true,
                    latency: None,
                    fault_injection: Some(fault_config),
                    rate_limit: None,
                    traffic_shaping: None,
                    circuit_breaker: None,
                    bulkhead: None,
                }
            }
            FailureType::PartialResponse {
                truncate_percentage,
            } => {
                let fault_config = FaultInjectionConfig {
                    enabled: true,
                    http_errors: vec![],
                    http_error_probability: 0.0,
                    connection_errors: false,
                    connection_error_probability: 0.0,
                    timeout_errors: false,
                    timeout_ms: 5000,
                    timeout_probability: 0.0,
                    partial_responses: true,
                    partial_response_probability: rule.probability,
                    payload_corruption: false,
                    payload_corruption_probability: 0.0,
                    corruption_type: crate::config::CorruptionType::None,
                    error_pattern: None,
                    mockai_enabled: false,
                };

                ChaosConfig {
                    enabled: true,
                    latency: None,
                    fault_injection: Some(fault_config),
                    rate_limit: None,
                    traffic_shaping: None,
                    circuit_breaker: None,
                    bulkhead: None,
                }
            }
            FailureType::WebhookFailure { .. } => {
                // Webhook failures are handled via hooks, not chaos config
                // Return a minimal config
                ChaosConfig::default()
            }
        };

        // Create scenario
        let scenario = ChaosScenario::new(rule.name.clone(), chaos_config)
            .with_description(rule.description.clone().unwrap_or_default());

        Ok(scenario)
    }

    /// Generate webhook failure hook configuration
    ///
    /// For webhook failure rules, generates the hook configuration that will
    /// intercept and fail webhook executions.
    pub fn generate_webhook_hook(&self, rule: &FailureDesignRule) -> Result<Value, String> {
        if let FailureType::WebhookFailure { webhook_pattern } = &rule.failure_type {
            // Generate hook configuration for webhook failure
            Ok(json!({
                "type": "webhook_failure",
                "name": rule.name,
                "webhook_pattern": webhook_pattern,
                "probability": rule.probability,
                "target": rule.target,
                "conditions": rule.conditions,
            }))
        } else {
            Err("Rule is not a webhook failure type".to_string())
        }
    }

    /// Generate route chaos configuration
    ///
    /// Converts failure design rules into route-specific chaos configurations
    /// that can be used with RouteChaosInjector.
    pub fn generate_route_chaos_config(&self, rule: &FailureDesignRule) -> Result<Value, String> {
        // Validate rule
        self.validate_rule(rule)?;

        // Build route config
        let mut route_configs = Vec::new();

        for endpoint in &rule.target.endpoints {
            let mut route_config = json!({
                "path": endpoint,
                "probability": rule.probability,
            });

            // Add method filter if specified
            if let Some(methods) = &rule.target.methods {
                route_config["methods"] = json!(methods);
            }

            // Add failure type configuration
            match &rule.failure_type {
                FailureType::StatusCode { code } => {
                    route_config["fault_injection"] = json!({
                        "enabled": true,
                        "status_code": code,
                    });
                }
                FailureType::Latency { delay_ms } => {
                    route_config["latency"] = json!({
                        "enabled": true,
                        "delay_ms": delay_ms,
                    });
                }
                FailureType::Timeout { timeout_ms } => {
                    route_config["fault_injection"] = json!({
                        "enabled": true,
                        "timeout": true,
                        "timeout_ms": timeout_ms,
                    });
                }
                FailureType::ConnectionError => {
                    route_config["fault_injection"] = json!({
                        "enabled": true,
                        "connection_error": true,
                    });
                }
                FailureType::PartialResponse {
                    truncate_percentage,
                } => {
                    route_config["fault_injection"] = json!({
                        "enabled": true,
                        "partial_response": true,
                        "truncate_percentage": truncate_percentage,
                    });
                }
                FailureType::WebhookFailure { .. } => {
                    // Webhook failures are handled separately
                    continue;
                }
            }

            // Add condition matching
            if !rule.conditions.is_empty() {
                route_config["conditions"] = json!(rule.conditions);
            }

            // Add user agent filter
            if let Some(user_agents) = &rule.target.user_agents {
                route_config["user_agent_patterns"] = json!(user_agents);
            }

            // Add IP range filter
            if let Some(ip_ranges) = &rule.target.ip_ranges {
                route_config["ip_ranges"] = json!(ip_ranges);
            }

            // Add header filters
            if let Some(headers) = &rule.target.headers {
                route_config["header_filters"] = json!(headers);
            }

            route_configs.push(route_config);
        }

        Ok(json!({
            "routes": route_configs,
        }))
    }
}

impl Default for FailureDesigner {
    fn default() -> Self {
        Self::new()
    }
}
