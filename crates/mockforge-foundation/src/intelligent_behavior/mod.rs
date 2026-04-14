//! Foundational types for intelligent behavior
//!
//! These types are shared between `mockforge-core` (which defines the richer
//! behavior rules, state machines, and MockAI implementation) and consumers
//! that only need the base request/response types and personas.
//!
//! Kept minimal: only pure data with no cross-crate dependencies.

pub mod config;
pub mod session;
pub mod session_state;
pub mod types;

pub use config::{
    BehaviorModelConfig, IntelligentBehaviorConfig, PerformanceConfig, PersonasConfig,
    VectorStoreConfig,
};
pub use session::{SessionManager, SessionTracking, SessionTrackingMethod};
pub use session_state::{InteractionRecord, SessionState};
pub use types::BehaviorRules;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A persona defines consistent data patterns across endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Persona {
    /// Persona name (e.g., "commercial_midwest", "hobbyist_urban")
    pub name: String,

    /// Persona traits (key-value pairs, e.g., "apiary_count": "20-40", "hive_count": "800-1500")
    #[serde(default)]
    pub traits: HashMap<String, String>,
}

impl Persona {
    /// Get a numeric trait value, parsing ranges like "20-40" or single values.
    /// Returns the midpoint for ranges, or the value for single numbers.
    pub fn get_numeric_trait(&self, key: &str) -> Option<u64> {
        self.traits.get(key).and_then(|value| {
            if let Some((min_str, max_str)) = value.split_once('-') {
                if let (Ok(min), Ok(max)) =
                    (min_str.trim().parse::<u64>(), max_str.trim().parse::<u64>())
                {
                    return Some((min + max) / 2);
                }
            }
            value.parse::<u64>().ok()
        })
    }

    /// Get a trait value as string.
    pub fn get_trait(&self, key: &str) -> Option<&String> {
        self.traits.get(key)
    }
}

/// LLM generation request — passed to `LlmClient::generate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmGenerationRequest {
    /// System prompt (instructions to the model).
    pub system_prompt: String,
    /// User prompt (constructed from request context).
    pub user_prompt: String,
    /// Sampling temperature (0.0–2.0).
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    /// Maximum tokens to generate.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
    /// Expected response schema (JSON Schema).
    pub schema: Option<serde_json::Value>,
}

impl LlmGenerationRequest {
    /// Create a new LLM generation request.
    pub fn new(system_prompt: impl Into<String>, user_prompt: impl Into<String>) -> Self {
        Self {
            system_prompt: system_prompt.into(),
            user_prompt: user_prompt.into(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            schema: None,
        }
    }

    /// Set temperature.
    #[must_use]
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = temperature;
        self
    }

    /// Set max tokens.
    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set expected schema.
    #[must_use]
    pub fn with_schema(mut self, schema: serde_json::Value) -> Self {
        self.schema = Some(schema);
        self
    }
}

fn default_temperature() -> f64 {
    0.7
}

fn default_max_tokens() -> usize {
    1024
}

/// HTTP request for MockAI processing.
#[derive(Debug, Clone)]
pub struct Request {
    /// HTTP method.
    pub method: String,
    /// Request path.
    pub path: String,
    /// Request body.
    pub body: Option<Value>,
    /// Query parameters.
    pub query_params: HashMap<String, String>,
    /// Headers.
    pub headers: HashMap<String, String>,
}

/// HTTP response from MockAI.
#[derive(Debug, Clone)]
pub struct Response {
    /// HTTP status code.
    pub status_code: u16,
    /// Response body.
    pub body: Value,
    /// Response headers.
    pub headers: HashMap<String, String>,
}

/// Captured HTTP request/response exchange used for behavioral analysis.
#[derive(Debug, Clone)]
pub struct HttpExchange {
    /// HTTP method.
    pub method: String,
    /// Request path.
    pub path: String,
    /// Query parameters (raw query string).
    pub query_params: Option<String>,
    /// Request headers (JSON string).
    pub headers: String,
    /// Request body (optional).
    pub body: Option<String>,
    /// Request body encoding.
    pub body_encoding: String,
    /// Response status code.
    pub status_code: Option<i32>,
    /// Response headers (JSON string).
    pub response_headers: Option<String>,
    /// Response body (optional).
    pub response_body: Option<String>,
    /// Response body encoding.
    pub response_body_encoding: Option<String>,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
}
