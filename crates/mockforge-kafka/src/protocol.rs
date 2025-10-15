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
        // Basic response serialization - full protocol serialization not yet implemented
        match response {
            KafkaResponse::ApiVersions => {
                // Minimal ApiVersions response
                let mut data = Vec::new();
                // Correlation ID
                data.extend_from_slice(&correlation_id.to_be_bytes());
                // Error code (0 = success)
                data.extend_from_slice(&0i16.to_be_bytes());
                // Empty API keys array for now
                data.extend_from_slice(&0i32.to_be_bytes());
                Ok(data)
            }
            KafkaResponse::CreateTopics => {
                // Minimal CreateTopics response
                let mut data = Vec::new();
                data.extend_from_slice(&correlation_id.to_be_bytes());
                data.extend_from_slice(&0i16.to_be_bytes()); // Error code
                data.extend_from_slice(&1i32.to_be_bytes()); // Number of topics
                                                             // Topic name (length + bytes)
                let topic_name = b"default-topic";
                data.extend_from_slice(&(topic_name.len() as i16).to_be_bytes());
                data.extend_from_slice(topic_name);
                data.extend_from_slice(&0i16.to_be_bytes()); // Error code for topic
                Ok(data)
            }
            _ => {
                // Minimal response for other types
                let mut data = Vec::new();
                data.extend_from_slice(&correlation_id.to_be_bytes());
                data.extend_from_slice(&0i16.to_be_bytes()); // Error code
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
