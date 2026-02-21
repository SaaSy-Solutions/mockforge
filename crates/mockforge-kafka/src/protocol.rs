//! Kafka protocol handling
//!
//! This module contains the low-level Kafka protocol implementation,
//! including request/response parsing and wire protocol handling.

use std::collections::HashMap;

/// Kafka protocol handler
#[derive(Debug)]
pub struct KafkaProtocolHandler {
    api_versions: HashMap<i16, ApiVersion>,
}

impl KafkaProtocolHandler {
    /// Create a new protocol handler
    pub fn new() -> Self {
        let mut api_versions = HashMap::new();
        // Add supported API versions
        api_versions.insert(
            0,
            ApiVersion {
                min_version: 0,
                max_version: 12,
            },
        ); // Produce
        api_versions.insert(
            1,
            ApiVersion {
                min_version: 0,
                max_version: 16,
            },
        ); // Fetch
        api_versions.insert(
            3,
            ApiVersion {
                min_version: 0,
                max_version: 12,
            },
        ); // Metadata
        api_versions.insert(
            9,
            ApiVersion {
                min_version: 0,
                max_version: 5,
            },
        ); // ListGroups
        api_versions.insert(
            15,
            ApiVersion {
                min_version: 0,
                max_version: 9,
            },
        ); // DescribeGroups
        api_versions.insert(
            16,
            ApiVersion {
                min_version: 0,
                max_version: 9,
            },
        ); // DescribeGroups (alternative)
        api_versions.insert(
            18,
            ApiVersion {
                min_version: 0,
                max_version: 4,
            },
        ); // ApiVersions
        api_versions.insert(
            19,
            ApiVersion {
                min_version: 0,
                max_version: 7,
            },
        ); // CreateTopics
        api_versions.insert(
            20,
            ApiVersion {
                min_version: 0,
                max_version: 6,
            },
        ); // DeleteTopics
        api_versions.insert(
            32,
            ApiVersion {
                min_version: 0,
                max_version: 4,
            },
        ); // DescribeConfigs
        api_versions.insert(
            49,
            ApiVersion {
                min_version: 0,
                max_version: 4,
            },
        ); // DescribeConfigs (alternative)

        Self { api_versions }
    }
}

impl Default for KafkaProtocolHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl KafkaProtocolHandler {
    /// Parse a Kafka request from bytes
    pub fn parse_request(&self, data: &[u8]) -> Result<KafkaRequest> {
        // Parse Kafka protocol header
        if data.len() < 12 {
            return Err(anyhow::anyhow!("Message too short for header"));
        }

        // Extract API key from bytes 4-5 (big-endian i16)
        let api_key = ((data[4] as i16) << 8) | (data[5] as i16);

        // Extract API version from bytes 6-7 (big-endian i16)
        let api_version = ((data[6] as i16) << 8) | (data[7] as i16);

        // Extract correlation ID from bytes 8-11 (big-endian i32)
        let correlation_id = ((data[8] as i32) << 24)
            | ((data[9] as i32) << 16)
            | ((data[10] as i32) << 8)
            | (data[11] as i32);

        // Parse client ID length from bytes 12-13 (big-endian i16)
        if data.len() < 14 {
            return Err(anyhow::anyhow!("Message too short for client ID length"));
        }
        let client_id_len = ((data[12] as i16) << 8) | (data[13] as i16);

        // Parse client ID
        let client_id_start = 14;
        let client_id_end = client_id_start + (client_id_len as usize);
        if data.len() < client_id_end {
            return Err(anyhow::anyhow!("Message too short for client ID"));
        }
        let client_id = if client_id_len > 0 {
            String::from_utf8(data[client_id_start..client_id_end].to_vec())
                .map_err(|e| anyhow::anyhow!("Invalid client ID encoding: {}", e))?
        } else {
            String::new()
        };

        let request_type = match api_key {
            0 => KafkaRequestType::Produce,
            1 => KafkaRequestType::Fetch,
            3 => KafkaRequestType::Metadata,
            9 => KafkaRequestType::ListGroups,
            15 => KafkaRequestType::DescribeGroups,
            18 => KafkaRequestType::ApiVersions,
            19 => KafkaRequestType::CreateTopics,
            20 => KafkaRequestType::DeleteTopics,
            32 => KafkaRequestType::DescribeConfigs,
            _ => KafkaRequestType::ApiVersions, // Default to ApiVersions for unsupported APIs
        };

        Ok(KafkaRequest {
            api_key,
            api_version,
            correlation_id,
            client_id,
            request_type,
        })
    }

    /// Serialize a Kafka response to bytes
    pub fn serialize_response(
        &self,
        response: &KafkaResponse,
        correlation_id: i32,
    ) -> Result<Vec<u8>> {
        fn push_kafka_string(buf: &mut Vec<u8>, value: &str) {
            buf.extend_from_slice(&(value.len() as i16).to_be_bytes());
            buf.extend_from_slice(value.as_bytes());
        }

        match response {
            KafkaResponse::ApiVersions => {
                // ApiVersions response with all registered API keys.
                let mut api_versions = self.api_versions.iter().collect::<Vec<_>>();
                api_versions.sort_by_key(|(api_key, _)| **api_key);

                let mut data = Vec::new();
                data.extend_from_slice(&correlation_id.to_be_bytes());
                data.extend_from_slice(&0i16.to_be_bytes());
                data.extend_from_slice(&(api_versions.len() as i32).to_be_bytes());
                for (api_key, version) in api_versions {
                    data.extend_from_slice(&api_key.to_be_bytes());
                    data.extend_from_slice(&version.min_version.to_be_bytes());
                    data.extend_from_slice(&version.max_version.to_be_bytes());
                }
                data.extend_from_slice(&0i32.to_be_bytes()); // throttle_time_ms
                Ok(data)
            }
            KafkaResponse::CreateTopics => {
                let mut data = Vec::new();
                data.extend_from_slice(&correlation_id.to_be_bytes());
                data.extend_from_slice(&0i32.to_be_bytes()); // throttle_time_ms
                data.extend_from_slice(&1i32.to_be_bytes()); // topics array length
                push_kafka_string(&mut data, "default-topic");
                data.extend_from_slice(&0i16.to_be_bytes()); // error_code
                data.extend_from_slice(&(-1i16).to_be_bytes()); // nullable error message
                Ok(data)
            }
            _ => {
                // Generic "success" response envelope for currently-supported request handlers.
                let mut data = Vec::new();
                data.extend_from_slice(&correlation_id.to_be_bytes());
                data.extend_from_slice(&0i16.to_be_bytes());
                Ok(data)
            }
        }
    }

    /// Check if API version is supported
    pub fn is_api_version_supported(&self, api_key: i16, version: i16) -> bool {
        if let Some(api_version) = self.api_versions.get(&api_key) {
            version >= api_version.min_version && version <= api_version.max_version
        } else {
            false
        }
    }
}

/// Represents a parsed Kafka request with header information
#[derive(Debug)]
pub struct KafkaRequest {
    pub api_key: i16,
    pub api_version: i16,
    pub correlation_id: i32,
    pub client_id: String,
    pub request_type: KafkaRequestType,
}

/// Kafka request types
#[derive(Debug)]
pub enum KafkaRequestType {
    Metadata,
    Produce,
    Fetch,
    ListGroups,
    DescribeGroups,
    ApiVersions,
    CreateTopics,
    DeleteTopics,
    DescribeConfigs,
}

/// Represents a Kafka response
#[derive(Debug)]
pub enum KafkaResponse {
    Metadata,
    Produce,
    Fetch,
    ListGroups,
    DescribeGroups,
    ApiVersions,
    CreateTopics,
    DeleteTopics,
    DescribeConfigs,
}

#[derive(Debug)]
struct ApiVersion {
    min_version: i16,
    max_version: i16,
}

type Result<T> = std::result::Result<T, anyhow::Error>;

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== KafkaProtocolHandler Tests ====================

    #[test]
    fn test_protocol_handler_new() {
        let handler = KafkaProtocolHandler::new();
        assert!(handler.api_versions.len() > 0);
        assert!(handler.api_versions.contains_key(&0)); // Produce
        assert!(handler.api_versions.contains_key(&1)); // Fetch
        assert!(handler.api_versions.contains_key(&18)); // ApiVersions
    }

    #[test]
    fn test_protocol_handler_default() {
        let handler = KafkaProtocolHandler::default();
        assert!(handler.api_versions.len() > 0);
    }

    #[test]
    fn test_is_api_version_supported_produce() {
        let handler = KafkaProtocolHandler::new();
        // Produce API (key 0) supports versions 0-12
        assert!(handler.is_api_version_supported(0, 0));
        assert!(handler.is_api_version_supported(0, 12));
        assert!(!handler.is_api_version_supported(0, 13));
        assert!(!handler.is_api_version_supported(0, -1));
    }

    #[test]
    fn test_is_api_version_supported_fetch() {
        let handler = KafkaProtocolHandler::new();
        // Fetch API (key 1) supports versions 0-16
        assert!(handler.is_api_version_supported(1, 0));
        assert!(handler.is_api_version_supported(1, 16));
        assert!(!handler.is_api_version_supported(1, 17));
    }

    #[test]
    fn test_is_api_version_supported_metadata() {
        let handler = KafkaProtocolHandler::new();
        // Metadata API (key 3) supports versions 0-12
        assert!(handler.is_api_version_supported(3, 0));
        assert!(handler.is_api_version_supported(3, 12));
        assert!(!handler.is_api_version_supported(3, 13));
    }

    #[test]
    fn test_is_api_version_supported_api_versions() {
        let handler = KafkaProtocolHandler::new();
        // ApiVersions API (key 18) supports versions 0-4
        assert!(handler.is_api_version_supported(18, 0));
        assert!(handler.is_api_version_supported(18, 4));
        assert!(!handler.is_api_version_supported(18, 5));
    }

    #[test]
    fn test_is_api_version_unsupported_api_key() {
        let handler = KafkaProtocolHandler::new();
        // API key 999 doesn't exist
        assert!(!handler.is_api_version_supported(999, 0));
        assert!(!handler.is_api_version_supported(-1, 0));
    }

    // ==================== parse_request Tests ====================

    #[test]
    fn test_parse_request_too_short() {
        let handler = KafkaProtocolHandler::new();
        let data = vec![0u8; 5]; // Too short for header
        let result = handler.parse_request(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_request_minimal_header() {
        let handler = KafkaProtocolHandler::new();
        // Create minimal valid request
        let mut data = vec![0u8; 14];
        // API key (2 bytes): 18 (ApiVersions)
        data[4] = 0;
        data[5] = 18;
        // API version (2 bytes): 0
        data[6] = 0;
        data[7] = 0;
        // Correlation ID (4 bytes): 1
        data[8] = 0;
        data[9] = 0;
        data[10] = 0;
        data[11] = 1;
        // Client ID length (2 bytes): 0 (empty string)
        data[12] = 0;
        data[13] = 0;

        let result = handler.parse_request(&data);
        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.api_key, 18);
        assert_eq!(request.api_version, 0);
        assert_eq!(request.correlation_id, 1);
        assert_eq!(request.client_id, "");
    }

    #[test]
    fn test_parse_request_with_client_id() {
        let handler = KafkaProtocolHandler::new();
        let client_id = b"test-client";
        let client_id_len = client_id.len() as i16;

        let mut data = vec![0u8; 14 + client_id.len()];
        // API key: 0 (Produce)
        data[4] = 0;
        data[5] = 0;
        // API version: 7
        data[6] = 0;
        data[7] = 7;
        // Correlation ID: 42
        data[8] = 0;
        data[9] = 0;
        data[10] = 0;
        data[11] = 42;
        // Client ID length
        data[12] = (client_id_len >> 8) as u8;
        data[13] = (client_id_len & 0xFF) as u8;
        // Client ID
        data[14..].copy_from_slice(client_id);

        let result = handler.parse_request(&data);
        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.api_key, 0);
        assert_eq!(request.api_version, 7);
        assert_eq!(request.correlation_id, 42);
        assert_eq!(request.client_id, "test-client");
        assert!(matches!(request.request_type, KafkaRequestType::Produce));
    }

    #[test]
    fn test_parse_request_produce() {
        let handler = KafkaProtocolHandler::new();
        let mut data = vec![0u8; 14];
        data[4] = 0; // API key: 0 (Produce)
        data[5] = 0;
        data[12] = 0; // Client ID length: 0
        data[13] = 0;

        let result = handler.parse_request(&data).unwrap();
        assert!(matches!(result.request_type, KafkaRequestType::Produce));
    }

    #[test]
    fn test_parse_request_fetch() {
        let handler = KafkaProtocolHandler::new();
        let mut data = vec![0u8; 14];
        data[4] = 0; // API key: 1 (Fetch)
        data[5] = 1;
        data[12] = 0;
        data[13] = 0;

        let result = handler.parse_request(&data).unwrap();
        assert!(matches!(result.request_type, KafkaRequestType::Fetch));
    }

    #[test]
    fn test_parse_request_metadata() {
        let handler = KafkaProtocolHandler::new();
        let mut data = vec![0u8; 14];
        data[4] = 0; // API key: 3 (Metadata)
        data[5] = 3;
        data[12] = 0;
        data[13] = 0;

        let result = handler.parse_request(&data).unwrap();
        assert!(matches!(result.request_type, KafkaRequestType::Metadata));
    }

    #[test]
    fn test_parse_request_list_groups() {
        let handler = KafkaProtocolHandler::new();
        let mut data = vec![0u8; 14];
        data[4] = 0; // API key: 9 (ListGroups)
        data[5] = 9;
        data[12] = 0;
        data[13] = 0;

        let result = handler.parse_request(&data).unwrap();
        assert!(matches!(result.request_type, KafkaRequestType::ListGroups));
    }

    #[test]
    fn test_parse_request_describe_groups() {
        let handler = KafkaProtocolHandler::new();
        let mut data = vec![0u8; 14];
        data[4] = 0; // API key: 15 (DescribeGroups)
        data[5] = 15;
        data[12] = 0;
        data[13] = 0;

        let result = handler.parse_request(&data).unwrap();
        assert!(matches!(result.request_type, KafkaRequestType::DescribeGroups));
    }

    #[test]
    fn test_parse_request_api_versions() {
        let handler = KafkaProtocolHandler::new();
        let mut data = vec![0u8; 14];
        data[4] = 0; // API key: 18 (ApiVersions)
        data[5] = 18;
        data[12] = 0;
        data[13] = 0;

        let result = handler.parse_request(&data).unwrap();
        assert!(matches!(result.request_type, KafkaRequestType::ApiVersions));
    }

    #[test]
    fn test_parse_request_create_topics() {
        let handler = KafkaProtocolHandler::new();
        let mut data = vec![0u8; 14];
        data[4] = 0; // API key: 19 (CreateTopics)
        data[5] = 19;
        data[12] = 0;
        data[13] = 0;

        let result = handler.parse_request(&data).unwrap();
        assert!(matches!(result.request_type, KafkaRequestType::CreateTopics));
    }

    #[test]
    fn test_parse_request_delete_topics() {
        let handler = KafkaProtocolHandler::new();
        let mut data = vec![0u8; 14];
        data[4] = 0; // API key: 20 (DeleteTopics)
        data[5] = 20;
        data[12] = 0;
        data[13] = 0;

        let result = handler.parse_request(&data).unwrap();
        assert!(matches!(result.request_type, KafkaRequestType::DeleteTopics));
    }

    #[test]
    fn test_parse_request_describe_configs() {
        let handler = KafkaProtocolHandler::new();
        let mut data = vec![0u8; 14];
        data[4] = 0; // API key: 32 (DescribeConfigs)
        data[5] = 32;
        data[12] = 0;
        data[13] = 0;

        let result = handler.parse_request(&data).unwrap();
        assert!(matches!(result.request_type, KafkaRequestType::DescribeConfigs));
    }

    #[test]
    fn test_parse_request_unsupported_api_defaults_to_api_versions() {
        let handler = KafkaProtocolHandler::new();
        let mut data = vec![0u8; 14];
        data[4] = 0; // API key: 99 (unsupported)
        data[5] = 99;
        data[12] = 0;
        data[13] = 0;

        let result = handler.parse_request(&data).unwrap();
        // Unsupported APIs default to ApiVersions
        assert!(matches!(result.request_type, KafkaRequestType::ApiVersions));
    }

    #[test]
    fn test_parse_request_invalid_client_id_length() {
        let handler = KafkaProtocolHandler::new();
        let mut data = vec![0u8; 14];
        data[4] = 0;
        data[5] = 18;
        // Client ID length: 100 (but not enough data)
        data[12] = 0;
        data[13] = 100;

        let result = handler.parse_request(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_request_missing_client_id_length() {
        let handler = KafkaProtocolHandler::new();
        let data = vec![0u8; 12]; // Too short to contain client ID length

        let result = handler.parse_request(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_request_max_values() {
        let handler = KafkaProtocolHandler::new();
        let mut data = vec![0u8; 14];
        // Max API key value
        data[4] = 0x7F;
        data[5] = 0xFF;
        // Max API version
        data[6] = 0x7F;
        data[7] = 0xFF;
        // Max correlation ID
        data[8] = 0x7F;
        data[9] = 0xFF;
        data[10] = 0xFF;
        data[11] = 0xFF;
        // Empty client ID
        data[12] = 0;
        data[13] = 0;

        let result = handler.parse_request(&data);
        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.api_key, 0x7FFF);
        assert_eq!(request.api_version, 0x7FFF);
        assert_eq!(request.correlation_id, 0x7FFFFFFF);
    }

    // ==================== serialize_response Tests ====================

    #[test]
    fn test_serialize_response_api_versions() {
        let handler = KafkaProtocolHandler::new();
        let response = KafkaResponse::ApiVersions;
        let correlation_id = 12345;

        let result = handler.serialize_response(&response, correlation_id);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert!(data.len() > 0);

        // Check correlation ID (first 4 bytes)
        let corr_id = i32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(corr_id, correlation_id);

        // Check error code (next 2 bytes)
        let error_code = i16::from_be_bytes([data[4], data[5]]);
        assert_eq!(error_code, 0); // No error
    }

    #[test]
    fn test_serialize_response_create_topics() {
        let handler = KafkaProtocolHandler::new();
        let response = KafkaResponse::CreateTopics;
        let correlation_id = 999;

        let result = handler.serialize_response(&response, correlation_id);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert!(data.len() > 0);

        // Check correlation ID
        let corr_id = i32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(corr_id, correlation_id);
    }

    #[test]
    fn test_serialize_response_metadata() {
        let handler = KafkaProtocolHandler::new();
        let response = KafkaResponse::Metadata;
        let correlation_id = 1;

        let result = handler.serialize_response(&response, correlation_id);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert!(data.len() >= 6); // correlation_id (4) + error_code (2)
    }

    #[test]
    fn test_serialize_response_produce() {
        let handler = KafkaProtocolHandler::new();
        let response = KafkaResponse::Produce;
        let correlation_id = 42;

        let result = handler.serialize_response(&response, correlation_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_response_fetch() {
        let handler = KafkaProtocolHandler::new();
        let response = KafkaResponse::Fetch;
        let correlation_id = 100;

        let result = handler.serialize_response(&response, correlation_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_response_list_groups() {
        let handler = KafkaProtocolHandler::new();
        let response = KafkaResponse::ListGroups;
        let correlation_id = 200;

        let result = handler.serialize_response(&response, correlation_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_response_describe_groups() {
        let handler = KafkaProtocolHandler::new();
        let response = KafkaResponse::DescribeGroups;
        let correlation_id = 300;

        let result = handler.serialize_response(&response, correlation_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_response_delete_topics() {
        let handler = KafkaProtocolHandler::new();
        let response = KafkaResponse::DeleteTopics;
        let correlation_id = 400;

        let result = handler.serialize_response(&response, correlation_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_response_describe_configs() {
        let handler = KafkaProtocolHandler::new();
        let response = KafkaResponse::DescribeConfigs;
        let correlation_id = 500;

        let result = handler.serialize_response(&response, correlation_id);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_response_negative_correlation_id() {
        let handler = KafkaProtocolHandler::new();
        let response = KafkaResponse::ApiVersions;
        let correlation_id = -1;

        let result = handler.serialize_response(&response, correlation_id);
        assert!(result.is_ok());

        let data = result.unwrap();
        let corr_id = i32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(corr_id, -1);
    }

    #[test]
    fn test_serialize_response_zero_correlation_id() {
        let handler = KafkaProtocolHandler::new();
        let response = KafkaResponse::ApiVersions;
        let correlation_id = 0;

        let result = handler.serialize_response(&response, correlation_id);
        assert!(result.is_ok());

        let data = result.unwrap();
        let corr_id = i32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(corr_id, 0);
    }

    // ==================== KafkaRequest Debug Tests ====================

    #[test]
    fn test_kafka_request_debug() {
        let request = KafkaRequest {
            api_key: 0,
            api_version: 7,
            correlation_id: 123,
            client_id: "test".to_string(),
            request_type: KafkaRequestType::Produce,
        };

        let debug_str = format!("{:?}", request);
        assert!(debug_str.contains("KafkaRequest"));
        assert!(debug_str.contains("api_key"));
    }

    #[test]
    fn test_kafka_request_type_debug() {
        let metadata = KafkaRequestType::Metadata;
        let debug_str = format!("{:?}", metadata);
        assert!(debug_str.contains("Metadata"));
    }

    #[test]
    fn test_kafka_response_debug() {
        let response = KafkaResponse::Produce;
        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("Produce"));
    }

    // ==================== API Version Ranges Tests ====================

    #[test]
    fn test_api_version_ranges_complete() {
        let handler = KafkaProtocolHandler::new();

        // Test all configured API versions
        let api_configs = vec![
            (0, 0, 12), // Produce
            (1, 0, 16), // Fetch
            (3, 0, 12), // Metadata
            (9, 0, 5),  // ListGroups
            (15, 0, 9), // DescribeGroups
            (16, 0, 9), // DescribeGroups (alternative)
            (18, 0, 4), // ApiVersions
            (19, 0, 7), // CreateTopics
            (20, 0, 6), // DeleteTopics
            (32, 0, 4), // DescribeConfigs
            (49, 0, 4), // DescribeConfigs (alternative)
        ];

        for (api_key, min_ver, max_ver) in api_configs {
            assert!(handler.is_api_version_supported(api_key, min_ver));
            assert!(handler.is_api_version_supported(api_key, max_ver));
            assert!(!handler.is_api_version_supported(api_key, max_ver + 1));
            if min_ver > 0 {
                assert!(!handler.is_api_version_supported(api_key, min_ver - 1));
            }
        }
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_parse_request_large_client_id() {
        let handler = KafkaProtocolHandler::new();
        let client_id = "a".repeat(1000); // Large client ID
        let client_id_len = client_id.len() as i16;

        let mut data = vec![0u8; 14 + client_id.len()];
        data[4] = 0;
        data[5] = 18;
        data[12] = (client_id_len >> 8) as u8;
        data[13] = (client_id_len & 0xFF) as u8;
        data[14..].copy_from_slice(client_id.as_bytes());

        let result = handler.parse_request(&data);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().client_id, client_id);
    }

    #[test]
    fn test_parse_request_invalid_utf8_client_id() {
        let handler = KafkaProtocolHandler::new();
        let mut data = vec![0u8; 17];
        data[4] = 0;
        data[5] = 18;
        data[12] = 0;
        data[13] = 3; // 3 bytes client ID
                      // Invalid UTF-8 sequence
        data[14] = 0xFF;
        data[15] = 0xFF;
        data[16] = 0xFF;

        let result = handler.parse_request(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_api_version_struct() {
        let api_version = ApiVersion {
            min_version: 0,
            max_version: 10,
        };

        assert_eq!(api_version.min_version, 0);
        assert_eq!(api_version.max_version, 10);
    }
}
