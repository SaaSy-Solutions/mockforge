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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_fixture() -> TcpFixture {
        TcpFixture {
            identifier: "test-fixture".to_string(),
            name: "Test Fixture".to_string(),
            description: "A test fixture".to_string(),
            match_criteria: MatchCriteria::default(),
            response: TcpResponse {
                data: "Hello".to_string(),
                encoding: "text".to_string(),
                file_path: None,
                delay_ms: 0,
                close_after_response: false,
                keep_alive: true,
            },
            behavior: BehaviorConfig::default(),
        }
    }

    #[test]
    fn test_match_criteria_default() {
        let criteria = MatchCriteria::default();
        assert!(criteria.data_pattern.is_none());
        assert!(criteria.text_pattern.is_none());
        assert!(criteria.exact_bytes.is_none());
        assert!(!criteria.match_all);
        assert!(criteria.min_length.is_none());
        assert!(criteria.max_length.is_none());
    }

    #[test]
    fn test_match_criteria_clone() {
        let criteria = MatchCriteria {
            data_pattern: Some("48656c6c6f".to_string()),
            text_pattern: Some("hello.*".to_string()),
            exact_bytes: None,
            match_all: true,
            min_length: Some(5),
            max_length: Some(100),
        };

        let cloned = criteria.clone();
        assert_eq!(criteria.data_pattern, cloned.data_pattern);
        assert_eq!(criteria.text_pattern, cloned.text_pattern);
        assert_eq!(criteria.match_all, cloned.match_all);
    }

    #[test]
    fn test_match_criteria_debug() {
        let criteria = MatchCriteria::default();
        let debug = format!("{:?}", criteria);
        assert!(debug.contains("MatchCriteria"));
    }

    #[test]
    fn test_tcp_response_default_encoding() {
        let json = r#"{
            "data": "test"
        }"#;
        let response: TcpResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.encoding, "text");
    }

    #[test]
    fn test_tcp_response_clone() {
        let response = TcpResponse {
            data: "Hello World".to_string(),
            encoding: "base64".to_string(),
            file_path: Some(PathBuf::from("/path/to/file")),
            delay_ms: 100,
            close_after_response: true,
            keep_alive: false,
        };

        let cloned = response.clone();
        assert_eq!(response.data, cloned.data);
        assert_eq!(response.encoding, cloned.encoding);
        assert_eq!(response.file_path, cloned.file_path);
    }

    #[test]
    fn test_tcp_response_debug() {
        let response = TcpResponse {
            data: "test".to_string(),
            encoding: "text".to_string(),
            file_path: None,
            delay_ms: 0,
            close_after_response: false,
            keep_alive: true,
        };
        let debug = format!("{:?}", response);
        assert!(debug.contains("TcpResponse"));
    }

    #[test]
    fn test_behavior_config_default() {
        let config = BehaviorConfig::default();
        assert_eq!(config.connection_delay_ms, 0);
        assert!(config.throttle_bytes_per_sec.is_none());
        assert_eq!(config.drop_connection_probability, 0.0);
        assert!(config.partial_data_bytes.is_none());
    }

    #[test]
    fn test_behavior_config_clone() {
        let config = BehaviorConfig {
            connection_delay_ms: 500,
            throttle_bytes_per_sec: Some(1024),
            drop_connection_probability: 0.1,
            partial_data_bytes: Some(100),
        };

        let cloned = config.clone();
        assert_eq!(config.connection_delay_ms, cloned.connection_delay_ms);
        assert_eq!(config.throttle_bytes_per_sec, cloned.throttle_bytes_per_sec);
        assert_eq!(config.drop_connection_probability, cloned.drop_connection_probability);
    }

    #[test]
    fn test_behavior_config_debug() {
        let config = BehaviorConfig::default();
        let debug = format!("{:?}", config);
        assert!(debug.contains("BehaviorConfig"));
    }

    #[test]
    fn test_tcp_fixture_clone() {
        let fixture = create_test_fixture();
        let cloned = fixture.clone();
        assert_eq!(fixture.identifier, cloned.identifier);
        assert_eq!(fixture.name, cloned.name);
    }

    #[test]
    fn test_tcp_fixture_debug() {
        let fixture = create_test_fixture();
        let debug = format!("{:?}", fixture);
        assert!(debug.contains("TcpFixture"));
        assert!(debug.contains("test-fixture"));
    }

    #[test]
    fn test_tcp_fixture_serialize() {
        let fixture = create_test_fixture();
        let json = serde_json::to_string(&fixture).unwrap();
        assert!(json.contains("\"identifier\":\"test-fixture\""));
        assert!(json.contains("\"name\":\"Test Fixture\""));
    }

    #[test]
    fn test_tcp_fixture_deserialize() {
        let yaml = r#"
identifier: my-fixture
name: My Fixture
description: A fixture for testing
match_criteria:
  match_all: true
response:
  data: "Hello"
  encoding: text
  delay_ms: 0
  close_after_response: false
  keep_alive: true
"#;

        let fixture: TcpFixture = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(fixture.identifier, "my-fixture");
        assert_eq!(fixture.name, "My Fixture");
        assert!(fixture.match_criteria.match_all);
    }

    #[test]
    fn test_tcp_response_with_delay() {
        let response = TcpResponse {
            data: "delayed".to_string(),
            encoding: "text".to_string(),
            file_path: None,
            delay_ms: 1000,
            close_after_response: true,
            keep_alive: false,
        };

        assert_eq!(response.delay_ms, 1000);
        assert!(response.close_after_response);
        assert!(!response.keep_alive);
    }

    #[test]
    fn test_match_criteria_with_lengths() {
        let criteria = MatchCriteria {
            min_length: Some(10),
            max_length: Some(1000),
            ..Default::default()
        };

        assert_eq!(criteria.min_length, Some(10));
        assert_eq!(criteria.max_length, Some(1000));
    }
}
