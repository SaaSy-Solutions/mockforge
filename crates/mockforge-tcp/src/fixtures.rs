//! TCP fixture definitions and loading

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A TCP fixture defining how to handle TCP connections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpFixture {
    /// Unique identifier for this fixture
    pub identifier: String,

    /// Human-readable name
    pub name: String,

    /// Description of what this fixture does
    #[serde(default)]
    pub description: String,

    /// Matching criteria for incoming data
    pub match_criteria: MatchCriteria,

    /// Response configuration
    pub response: TcpResponse,

    /// Behavior simulation
    #[serde(default)]
    pub behavior: BehaviorConfig,
}

/// Criteria for matching incoming TCP data
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchCriteria {
    /// Match by data pattern (hex string, e.g., "48656c6c6f" for "Hello")
    #[serde(default)]
    pub data_pattern: Option<String>,

    /// Match by data pattern (regex for text data)
    #[serde(default)]
    pub text_pattern: Option<String>,

    /// Match by exact bytes (base64 encoded)
    #[serde(default)]
    pub exact_bytes: Option<String>,

    /// Match all (default fixture)
    #[serde(default)]
    pub match_all: bool,

    /// Minimum data length required to match
    #[serde(default)]
    pub min_length: Option<usize>,

    /// Maximum data length to match
    #[serde(default)]
    pub max_length: Option<usize>,
}

/// TCP response configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpResponse {
    /// Response data (hex string, base64, or plain text)
    pub data: String,

    /// Data encoding: "hex", "base64", "text", "file"
    #[serde(default = "default_encoding")]
    pub encoding: String,

    /// File path to load response from (if encoding is "file")
    #[serde(default)]
    pub file_path: Option<PathBuf>,

    /// Delay before responding (milliseconds)
    #[serde(default)]
    pub delay_ms: u64,

    /// Close connection after sending response
    #[serde(default)]
    pub close_after_response: bool,

    /// Keep connection open for streaming
    #[serde(default)]
    pub keep_alive: bool,
}

fn default_encoding() -> String {
    "text".to_string()
}

/// Behavior simulation configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BehaviorConfig {
    /// Simulate connection delay (milliseconds)
    #[serde(default)]
    pub connection_delay_ms: u64,

    /// Simulate slow data transfer (bytes per second)
    #[serde(default)]
    pub throttle_bytes_per_sec: Option<u64>,

    /// Simulate connection drops (probability 0.0-1.0)
    #[serde(default)]
    pub drop_connection_probability: f64,

    /// Simulate partial data (send N bytes then close)
    #[serde(default)]
    pub partial_data_bytes: Option<usize>,
}
