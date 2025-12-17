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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_basic_target() -> FailureTarget {
        FailureTarget {
            endpoints: vec!["/api/webhooks".to_string()],
            user_agents: None,
            ip_ranges: None,
            headers: None,
            methods: None,
        }
    }

    fn create_basic_rule() -> FailureDesignRule {
        FailureDesignRule {
            name: "test_rule".to_string(),
            target: create_basic_target(),
            failure_type: FailureType::StatusCode { code: 500 },
            conditions: vec![],
            probability: 0.5,
            description: Some("Test rule".to_string()),
        }
    }

    #[test]
    fn test_failure_designer_new() {
        let designer = FailureDesigner::new();
        assert!(true); // Designer created successfully
    }

    #[test]
    fn test_failure_designer_default() {
        let designer = FailureDesigner::default();
        assert!(true); // Designer created successfully
    }

    #[test]
    fn test_validate_rule_valid() {
        let designer = FailureDesigner::new();
        let rule = create_basic_rule();
        let result = designer.validate_rule(&rule);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_rule_probability_too_high() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.probability = 1.5;

        let result = designer.validate_rule(&rule);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Probability must be between"));
    }

    #[test]
    fn test_validate_rule_probability_negative() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.probability = -0.1;

        let result = designer.validate_rule(&rule);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Probability must be between"));
    }

    #[test]
    fn test_validate_rule_no_endpoints() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.target.endpoints.clear();

        let result = designer.validate_rule(&rule);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("At least one endpoint"));
    }

    #[test]
    fn test_validate_rule_invalid_status_code_low() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.failure_type = FailureType::StatusCode { code: 50 };

        let result = designer.validate_rule(&rule);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Status code must be between"));
    }

    #[test]
    fn test_validate_rule_invalid_status_code_high() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.failure_type = FailureType::StatusCode { code: 600 };

        let result = designer.validate_rule(&rule);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Status code must be between"));
    }

    #[test]
    fn test_validate_rule_zero_delay() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.failure_type = FailureType::Latency { delay_ms: 0 };

        let result = designer.validate_rule(&rule);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Delay must be greater than 0"));
    }

    #[test]
    fn test_validate_rule_zero_timeout() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.failure_type = FailureType::Timeout { timeout_ms: 0 };

        let result = designer.validate_rule(&rule);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Timeout must be greater than 0"));
    }

    #[test]
    fn test_validate_rule_invalid_truncate_percentage() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.failure_type = FailureType::PartialResponse {
            truncate_percentage: 1.5,
        };

        let result = designer.validate_rule(&rule);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Truncate percentage must be between"));
    }

    #[test]
    fn test_validate_rule_empty_webhook_pattern() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.failure_type = FailureType::WebhookFailure {
            webhook_pattern: "".to_string(),
        };

        let result = designer.validate_rule(&rule);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Webhook pattern cannot be empty"));
    }

    #[test]
    fn test_validate_rule_connection_error() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.failure_type = FailureType::ConnectionError;

        let result = designer.validate_rule(&rule);
        assert!(result.is_ok());
    }

    #[test]
    fn test_rule_to_scenario_status_code() {
        let designer = FailureDesigner::new();
        let rule = create_basic_rule();

        let result = designer.rule_to_scenario(&rule);
        assert!(result.is_ok());

        let scenario = result.unwrap();
        assert_eq!(scenario.name, "test_rule");
        assert!(scenario.chaos_config.enabled);
        assert!(scenario.chaos_config.fault_injection.is_some());

        let fault = scenario.chaos_config.fault_injection.unwrap();
        assert!(fault.enabled);
        assert_eq!(fault.http_errors, vec![500]);
        assert_eq!(fault.http_error_probability, 0.5);
    }

    #[test]
    fn test_rule_to_scenario_latency() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.failure_type = FailureType::Latency { delay_ms: 3000 };

        let result = designer.rule_to_scenario(&rule);
        assert!(result.is_ok());

        let scenario = result.unwrap();
        assert!(scenario.chaos_config.latency.is_some());

        let latency = scenario.chaos_config.latency.unwrap();
        assert!(latency.enabled);
        assert_eq!(latency.fixed_delay_ms, Some(3000));
        assert_eq!(latency.probability, 0.5);
    }

    #[test]
    fn test_rule_to_scenario_timeout() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.failure_type = FailureType::Timeout { timeout_ms: 5000 };

        let result = designer.rule_to_scenario(&rule);
        assert!(result.is_ok());

        let scenario = result.unwrap();
        assert!(scenario.chaos_config.fault_injection.is_some());

        let fault = scenario.chaos_config.fault_injection.unwrap();
        assert!(fault.timeout_errors);
        assert_eq!(fault.timeout_ms, 5000);
        assert_eq!(fault.timeout_probability, 0.5);
    }

    #[test]
    fn test_rule_to_scenario_connection_error() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.failure_type = FailureType::ConnectionError;

        let result = designer.rule_to_scenario(&rule);
        assert!(result.is_ok());

        let scenario = result.unwrap();
        assert!(scenario.chaos_config.fault_injection.is_some());

        let fault = scenario.chaos_config.fault_injection.unwrap();
        assert!(fault.connection_errors);
        assert_eq!(fault.connection_error_probability, 0.5);
    }

    #[test]
    fn test_rule_to_scenario_partial_response() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.failure_type = FailureType::PartialResponse {
            truncate_percentage: 0.7,
        };

        let result = designer.rule_to_scenario(&rule);
        assert!(result.is_ok());

        let scenario = result.unwrap();
        assert!(scenario.chaos_config.fault_injection.is_some());

        let fault = scenario.chaos_config.fault_injection.unwrap();
        assert!(fault.partial_responses);
        assert_eq!(fault.partial_response_probability, 0.5);
    }

    #[test]
    fn test_rule_to_scenario_webhook_failure() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.failure_type = FailureType::WebhookFailure {
            webhook_pattern: "/webhooks/*".to_string(),
        };

        let result = designer.rule_to_scenario(&rule);
        assert!(result.is_ok());

        let scenario = result.unwrap();
        // Webhook failures are handled via hooks, not chaos config,
        // so the chaos_config is default (enabled = false)
        assert!(!scenario.chaos_config.enabled);
    }

    #[test]
    fn test_rule_to_scenario_invalid_rule() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.probability = 2.0; // Invalid

        let result = designer.rule_to_scenario(&rule);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_webhook_hook_valid() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.failure_type = FailureType::WebhookFailure {
            webhook_pattern: "/webhooks/*".to_string(),
        };

        let result = designer.generate_webhook_hook(&rule);
        assert!(result.is_ok());

        let hook = result.unwrap();
        assert_eq!(hook["type"], "webhook_failure");
        assert_eq!(hook["name"], "test_rule");
        assert_eq!(hook["webhook_pattern"], "/webhooks/*");
        assert_eq!(hook["probability"], 0.5);
    }

    #[test]
    fn test_generate_webhook_hook_invalid_type() {
        let designer = FailureDesigner::new();
        let rule = create_basic_rule(); // StatusCode type

        let result = designer.generate_webhook_hook(&rule);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a webhook failure type"));
    }

    #[test]
    fn test_generate_route_chaos_config_status_code() {
        let designer = FailureDesigner::new();
        let rule = create_basic_rule();

        let result = designer.generate_route_chaos_config(&rule);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert!(config["routes"].is_array());
        assert_eq!(config["routes"].as_array().unwrap().len(), 1);

        let route = &config["routes"][0];
        assert_eq!(route["path"], "/api/webhooks");
        assert_eq!(route["probability"], 0.5);
        assert!(route["fault_injection"].is_object());
        assert_eq!(route["fault_injection"]["status_code"], 500);
    }

    #[test]
    fn test_generate_route_chaos_config_latency() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.failure_type = FailureType::Latency { delay_ms: 2000 };

        let result = designer.generate_route_chaos_config(&rule);
        assert!(result.is_ok());

        let config = result.unwrap();
        let route = &config["routes"][0];
        assert!(route["latency"].is_object());
        assert_eq!(route["latency"]["delay_ms"], 2000);
    }

    #[test]
    fn test_generate_route_chaos_config_with_methods() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.target.methods = Some(vec!["GET".to_string(), "POST".to_string()]);

        let result = designer.generate_route_chaos_config(&rule);
        assert!(result.is_ok());

        let config = result.unwrap();
        let route = &config["routes"][0];
        assert!(route["methods"].is_array());
        assert_eq!(route["methods"][0], "GET");
        assert_eq!(route["methods"][1], "POST");
    }

    #[test]
    fn test_generate_route_chaos_config_with_user_agents() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.target.user_agents = Some(vec!["Chrome.*".to_string()]);

        let result = designer.generate_route_chaos_config(&rule);
        assert!(result.is_ok());

        let config = result.unwrap();
        let route = &config["routes"][0];
        assert!(route["user_agent_patterns"].is_array());
        assert_eq!(route["user_agent_patterns"][0], "Chrome.*");
    }

    #[test]
    fn test_generate_route_chaos_config_with_ip_ranges() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.target.ip_ranges = Some(vec!["192.168.1.0/24".to_string()]);

        let result = designer.generate_route_chaos_config(&rule);
        assert!(result.is_ok());

        let config = result.unwrap();
        let route = &config["routes"][0];
        assert!(route["ip_ranges"].is_array());
        assert_eq!(route["ip_ranges"][0], "192.168.1.0/24");
    }

    #[test]
    fn test_generate_route_chaos_config_with_headers() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        let mut headers = HashMap::new();
        headers.insert("X-Test-Header".to_string(), "test-value".to_string());
        rule.target.headers = Some(headers);

        let result = designer.generate_route_chaos_config(&rule);
        assert!(result.is_ok());

        let config = result.unwrap();
        let route = &config["routes"][0];
        assert!(route["header_filters"].is_object());
        assert_eq!(route["header_filters"]["X-Test-Header"], "test-value");
    }

    #[test]
    fn test_generate_route_chaos_config_with_conditions() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.conditions = vec![FailureCondition {
            condition_type: ConditionType::Header,
            field: "User-Agent".to_string(),
            operator: ConditionOperator::Contains,
            value: json!("Chrome"),
        }];

        let result = designer.generate_route_chaos_config(&rule);
        assert!(result.is_ok());

        let config = result.unwrap();
        let route = &config["routes"][0];
        assert!(route["conditions"].is_array());
    }

    #[test]
    fn test_generate_route_chaos_config_multiple_endpoints() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.target.endpoints = vec!["/api/webhooks".to_string(), "/api/callbacks".to_string()];

        let result = designer.generate_route_chaos_config(&rule);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config["routes"].as_array().unwrap().len(), 2);
        assert_eq!(config["routes"][0]["path"], "/api/webhooks");
        assert_eq!(config["routes"][1]["path"], "/api/callbacks");
    }

    #[test]
    fn test_failure_type_serialize_deserialize() {
        let failure_type = FailureType::StatusCode { code: 404 };
        let json = serde_json::to_value(&failure_type).unwrap();
        assert_eq!(json["type"], "status_code");
        assert_eq!(json["code"], 404);

        let deserialized: FailureType = serde_json::from_value(json).unwrap();
        match deserialized {
            FailureType::StatusCode { code } => assert_eq!(code, 404),
            _ => panic!("Wrong failure type"),
        }
    }

    #[test]
    fn test_condition_type_serialize_deserialize() {
        let condition_type = ConditionType::Header;
        let json = serde_json::to_value(&condition_type).unwrap();
        assert_eq!(json, "header");

        let deserialized: ConditionType = serde_json::from_value(json).unwrap();
        match deserialized {
            ConditionType::Header => assert!(true),
            _ => panic!("Wrong condition type"),
        }
    }

    #[test]
    fn test_condition_operator_serialize_deserialize() {
        let operator = ConditionOperator::Equals;
        let json = serde_json::to_value(&operator).unwrap();
        assert_eq!(json, "equals");

        let deserialized: ConditionOperator = serde_json::from_value(json).unwrap();
        match deserialized {
            ConditionOperator::Equals => assert!(true),
            _ => panic!("Wrong operator"),
        }
    }

    #[test]
    fn test_failure_condition_serialize_deserialize() {
        let condition = FailureCondition {
            condition_type: ConditionType::Query,
            field: "user_id".to_string(),
            operator: ConditionOperator::Equals,
            value: json!("12345"),
        };

        let json = serde_json::to_value(&condition).unwrap();
        let deserialized: FailureCondition = serde_json::from_value(json).unwrap();

        assert_eq!(deserialized.field, "user_id");
        assert_eq!(deserialized.value, json!("12345"));
    }

    #[test]
    fn test_failure_target_serialize_deserialize() {
        let target = create_basic_target();
        let json = serde_json::to_value(&target).unwrap();
        let deserialized: FailureTarget = serde_json::from_value(json).unwrap();

        assert_eq!(deserialized.endpoints, vec!["/api/webhooks"]);
    }

    #[test]
    fn test_failure_design_rule_serialize_deserialize() {
        let rule = create_basic_rule();
        let json = serde_json::to_value(&rule).unwrap();
        let deserialized: FailureDesignRule = serde_json::from_value(json).unwrap();

        assert_eq!(deserialized.name, "test_rule");
        assert_eq!(deserialized.probability, 0.5);
    }

    #[test]
    fn test_all_condition_operators() {
        let operators = vec![
            ConditionOperator::Equals,
            ConditionOperator::NotEquals,
            ConditionOperator::Contains,
            ConditionOperator::Matches,
            ConditionOperator::GreaterThan,
            ConditionOperator::LessThan,
        ];

        for operator in operators {
            let json = serde_json::to_value(&operator).unwrap();
            let _deserialized: ConditionOperator = serde_json::from_value(json).unwrap();
        }
    }

    #[test]
    fn test_all_condition_types() {
        let types = vec![
            ConditionType::Header,
            ConditionType::Query,
            ConditionType::Body,
            ConditionType::Path,
        ];

        for condition_type in types {
            let json = serde_json::to_value(&condition_type).unwrap();
            let _deserialized: ConditionType = serde_json::from_value(json).unwrap();
        }
    }

    #[test]
    fn test_edge_case_probability_boundaries() {
        let designer = FailureDesigner::new();

        // Test 0.0
        let mut rule = create_basic_rule();
        rule.probability = 0.0;
        assert!(designer.validate_rule(&rule).is_ok());

        // Test 1.0
        rule.probability = 1.0;
        assert!(designer.validate_rule(&rule).is_ok());
    }

    #[test]
    fn test_edge_case_status_code_boundaries() {
        let designer = FailureDesigner::new();

        // Test 100
        let mut rule = create_basic_rule();
        rule.failure_type = FailureType::StatusCode { code: 100 };
        assert!(designer.validate_rule(&rule).is_ok());

        // Test 599
        rule.failure_type = FailureType::StatusCode { code: 599 };
        assert!(designer.validate_rule(&rule).is_ok());
    }

    #[test]
    fn test_webhook_pattern_with_wildcard() {
        let designer = FailureDesigner::new();
        let mut rule = create_basic_rule();
        rule.failure_type = FailureType::WebhookFailure {
            webhook_pattern: "/webhooks/*".to_string(),
        };

        assert!(designer.validate_rule(&rule).is_ok());
    }

    #[test]
    fn test_complex_rule_with_all_filters() {
        let designer = FailureDesigner::new();
        let mut headers = HashMap::new();
        headers.insert("X-Custom".to_string(), "value".to_string());

        let rule = FailureDesignRule {
            name: "complex_rule".to_string(),
            target: FailureTarget {
                endpoints: vec!["/api/*".to_string()],
                user_agents: Some(vec!["Chrome.*".to_string()]),
                ip_ranges: Some(vec!["10.0.0.0/8".to_string()]),
                headers: Some(headers),
                methods: Some(vec!["POST".to_string()]),
            },
            failure_type: FailureType::StatusCode { code: 503 },
            conditions: vec![FailureCondition {
                condition_type: ConditionType::Header,
                field: "Authorization".to_string(),
                operator: ConditionOperator::Contains,
                value: json!("Bearer"),
            }],
            probability: 0.25,
            description: Some("Complex test rule".to_string()),
        };

        assert!(designer.validate_rule(&rule).is_ok());
        let scenario = designer.rule_to_scenario(&rule).unwrap();
        assert_eq!(scenario.name, "complex_rule");
    }
}
