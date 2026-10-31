//! Route configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Route configuration for custom HTTP routes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RouteConfig {
    /// Route path (supports path parameters like /users/{id})
    pub path: String,
    /// HTTP method
    pub method: String,
    /// Request configuration
    pub request: Option<RouteRequestConfig>,
    /// Response configuration
    pub response: RouteResponseConfig,
    /// Per-route fault injection configuration
    #[serde(default)]
    pub fault_injection: Option<RouteFaultInjectionConfig>,
    /// Per-route latency configuration
    #[serde(default)]
    pub latency: Option<RouteLatencyConfig>,
}

/// Request configuration for routes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RouteRequestConfig {
    /// Request validation configuration
    pub validation: Option<RouteValidationConfig>,
}

/// Response configuration for routes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RouteResponseConfig {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Response body
    #[serde(
        default,
        deserialize_with = "deserialize_response_body",
        skip_serializing_if = "Option::is_none"
    )]
    pub body: Option<serde_json::Value>,
}

// Missing body retains the default response; an explicitly configured JSON
// null is a real response value and must survive configuration round trips.
fn deserialize_response_body<'de, D>(deserializer: D) -> Result<Option<serde_json::Value>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    serde_json::Value::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod response_body_tests {
    use super::RouteResponseConfig;
    use serde_json::{json, Value};

    #[test]
    fn explicit_null_and_missing_body_are_distinct() {
        let null: RouteResponseConfig =
            serde_json::from_value(json!({"status":200,"body":null})).unwrap();
        assert_eq!(null.body, Some(Value::Null));
        let missing: RouteResponseConfig = serde_json::from_value(json!({"status":200})).unwrap();
        assert_eq!(missing.body, None);
    }

    #[test]
    fn json_and_yaml_preserve_every_json_body_type() {
        for value in [
            json!(null),
            json!(true),
            json!(42),
            json!("{{uuid}}"),
            json!([1, 2]),
            json!({"id":1}),
        ] {
            let config = RouteResponseConfig {
                status: 200,
                headers: Default::default(),
                body: Some(value.clone()),
            };
            let json: RouteResponseConfig =
                serde_json::from_str(&serde_json::to_string(&config).unwrap()).unwrap();
            let yaml: RouteResponseConfig =
                serde_yaml::from_str(&serde_yaml::to_string(&config).unwrap()).unwrap();
            assert_eq!(json.body, Some(value.clone()));
            assert_eq!(yaml.body, Some(value));
        }
    }
}

/// Validation configuration for routes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RouteValidationConfig {
    /// JSON schema for request validation
    pub schema: serde_json::Value,
}

/// Per-route fault injection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RouteFaultInjectionConfig {
    /// Enable fault injection for this route
    pub enabled: bool,
    /// Probability of injecting a fault (0.0-1.0)
    pub probability: f64,
    /// Fault types to inject
    pub fault_types: Vec<RouteFaultType>,
}

/// Fault types that can be injected per route
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RouteFaultType {
    /// HTTP error with status code
    HttpError {
        /// HTTP status code to return
        status_code: u16,
        /// Optional error message
        message: Option<String>,
    },
    /// Connection error
    ConnectionError {
        /// Optional error message
        message: Option<String>,
    },
    /// Timeout error
    Timeout {
        /// Timeout duration in milliseconds
        duration_ms: u64,
        /// Optional error message
        message: Option<String>,
    },
    /// Partial response (truncate at percentage)
    PartialResponse {
        /// Percentage of response to truncate (0.0-100.0)
        truncate_percent: f64,
    },
    /// Payload corruption
    PayloadCorruption {
        /// Type of corruption to apply
        corruption_type: String,
    },
}

/// Per-route latency configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RouteLatencyConfig {
    /// Enable latency injection for this route
    pub enabled: bool,
    /// Probability of applying latency (0.0-1.0)
    pub probability: f64,
    /// Fixed delay in milliseconds
    pub fixed_delay_ms: Option<u64>,
    /// Random delay range (min_ms, max_ms)
    pub random_delay_range_ms: Option<(u64, u64)>,
    /// Jitter percentage (0.0-100.0)
    pub jitter_percent: f64,
    /// Latency distribution type
    #[serde(default)]
    pub distribution: LatencyDistribution,
}

/// Latency distribution type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum LatencyDistribution {
    /// Fixed delay
    #[default]
    Fixed,
    /// Normal distribution (requires mean and std_dev)
    Normal {
        /// Mean delay in milliseconds
        mean_ms: f64,
        /// Standard deviation in milliseconds
        std_dev_ms: f64,
    },
    /// Exponential distribution (requires lambda)
    Exponential {
        /// Lambda parameter for exponential distribution
        lambda: f64,
    },
    /// Uniform distribution (uses random_delay_range_ms)
    Uniform,
}

impl Default for RouteFaultInjectionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            probability: 0.0,
            fault_types: Vec::new(),
        }
    }
}

impl Default for RouteLatencyConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            probability: 1.0,
            fixed_delay_ms: None,
            random_delay_range_ms: None,
            jitter_percent: 0.0,
            distribution: LatencyDistribution::Fixed,
        }
    }
}
