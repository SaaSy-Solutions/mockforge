//! Kafka protocol handling
//!
//! This module contains the low-level Kafka protocol implementation,
//! including request/response parsing and wire protocol handling.

use std::collections::HashMap;

/// A topic the broker wants to advertise in its Metadata response.
///
/// Kept intentionally narrow (name + partition count) so the protocol layer
/// doesn't have to understand the full fixture schema — `broker.rs`
/// translates `KafkaTopicSpec` into these before constructing the handler.
#[derive(Debug, Clone)]
pub struct TopicMetadata {
    pub name: String,
    pub partitions: i32,
}

/// Kafka protocol handler
#[derive(Debug)]
pub struct KafkaProtocolHandler {
    api_versions: HashMap<i16, ApiVersion>,
    /// Host advertised to clients in Metadata responses.
    advertised_host: String,
    /// Port advertised to clients in Metadata responses.
    advertised_port: i32,
    /// Topics advertised in Metadata responses. Empty = zero-topic response
    /// (used by non-fixture-driven Docker runs and tests).
    topics: Vec<TopicMetadata>,
}

impl KafkaProtocolHandler {
    /// Create a new protocol handler with the default advertised endpoint
    /// and no topics.
    pub fn new() -> Self {
        Self::with_advertised_endpoint("localhost", 9092)
    }

    /// Create a new protocol handler that advertises a specific endpoint
    /// (host, port) in Metadata responses, with an empty topic list.
    pub fn with_advertised_endpoint(host: impl Into<String>, port: i32) -> Self {
        Self::with_topology(host, port, Vec::new())
    }

    /// Create a new protocol handler that advertises a specific endpoint
    /// AND a list of topics. Each topic is emitted in the Metadata v4
    /// response body with its partition count; every partition lists this
    /// broker (node 1) as leader + sole replica + in-sync replica.
    pub fn with_topology(host: impl Into<String>, port: i32, topics: Vec<TopicMetadata>) -> Self {
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
           // Metadata: we only implement v4 response encoding. Advertising max=4
           // forces auto-negotiating clients (librdkafka, kafka-python, etc.) to
           // pick a version we can actually serialize.
        api_versions.insert(
            3,
            ApiVersion {
                min_version: 0,
                max_version: 4,
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

        Self {
            api_versions,
            advertised_host: host.into(),
            advertised_port: port,
            topics,
        }
    }
}

impl Default for KafkaProtocolHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl KafkaProtocolHandler {
    /// Parse a Kafka request from bytes.
    ///
    /// `data` is the request body as delivered by `broker::handle_connection`
    /// — the 4-byte length prefix has already been consumed off the socket,
    /// so offset 0 here is the request header's `api_key`. (Earlier revisions
    /// of this function read from offset 4, which silently misinterpreted
    /// every real client request as api_key=0 / Produce.)
    pub fn parse_request(&self, data: &[u8]) -> Result<KafkaRequest> {
        // Need at least api_key(2) + api_version(2) + correlation_id(4).
        if data.len() < 8 {
            return Err(anyhow::anyhow!("Message too short for header"));
        }

        let api_key = i16::from_be_bytes([data[0], data[1]]);
        let api_version = i16::from_be_bytes([data[2], data[3]]);
        let correlation_id = i32::from_be_bytes([data[4], data[5], data[6], data[7]]);

        // Request-header v1+ has a nullable client_id string (int16 length
        // then bytes; -1 means null). v0 has no client_id, but every real
        // Kafka API has a v1+ request header, so we require it here.
        if data.len() < 10 {
            return Err(anyhow::anyhow!("Message too short for client ID length"));
        }
        let client_id_len = i16::from_be_bytes([data[8], data[9]]);

        let (client_id_end, client_id) = if client_id_len < 0 {
            // Null client_id
            (10, String::new())
        } else {
            let end = 10 + client_id_len as usize;
            if data.len() < end {
                return Err(anyhow::anyhow!("Message too short for client ID"));
            }
            let s = String::from_utf8(data[10..end].to_vec())
                .map_err(|e| anyhow::anyhow!("Invalid client ID encoding: {}", e))?;
            (end, s)
        };
        // Request-header v2 (flexible) appends a TAG_BUFFER after client_id;
        // we don't consume any tagged fields yet but it's fine to ignore them
        // — the caller only reads fields we've already parsed.
        let _ = client_id_end;

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

    /// Serialize a Kafka response to bytes.
    ///
    /// `request_api_version` is the API version the client requested, not the
    /// ApiVersions of the response itself — it controls whether we emit the
    /// "flexible" wire format (compact arrays + tag buffers, introduced for
    /// ApiVersions at v3). librdkafka/kcat default to ApiVersions v3 on
    /// connect, so without the flexible branch the handshake disconnects.
    pub fn serialize_response(
        &self,
        response: &KafkaResponse,
        correlation_id: i32,
        request_api_version: i16,
    ) -> Result<Vec<u8>> {
        fn push_kafka_string(buf: &mut Vec<u8>, value: &str) {
            buf.extend_from_slice(&(value.len() as i16).to_be_bytes());
            buf.extend_from_slice(value.as_bytes());
        }

        match response {
            KafkaResponse::ApiVersions => {
                let mut api_versions = self.api_versions.iter().collect::<Vec<_>>();
                api_versions.sort_by_key(|(api_key, _)| **api_key);

                let mut data = Vec::new();
                // ApiVersions response header is ALWAYS v0 (correlation_id only,
                // no tag buffer) even when the body is flexible — that lets
                // clients bootstrap without knowing the server version.
                data.extend_from_slice(&correlation_id.to_be_bytes());

                if request_api_version >= 3 {
                    // Flexible (v3+) body: error_code, COMPACT_ARRAY of api
                    // entries (each followed by an empty tag buffer),
                    // throttle_time_ms, trailing tag buffer.
                    data.extend_from_slice(&0i16.to_be_bytes()); // error_code
                    push_unsigned_varint(&mut data, (api_versions.len() as u32) + 1);
                    for (api_key, version) in api_versions {
                        data.extend_from_slice(&api_key.to_be_bytes());
                        data.extend_from_slice(&version.min_version.to_be_bytes());
                        data.extend_from_slice(&version.max_version.to_be_bytes());
                        data.push(0); // empty tag buffer for this entry
                    }
                    data.extend_from_slice(&0i32.to_be_bytes()); // throttle_time_ms
                    data.push(0); // top-level tag buffer
                } else {
                    // Non-flexible (v0–v2) body
                    data.extend_from_slice(&0i16.to_be_bytes()); // error_code
                    data.extend_from_slice(&(api_versions.len() as i32).to_be_bytes());
                    for (api_key, version) in api_versions {
                        data.extend_from_slice(&api_key.to_be_bytes());
                        data.extend_from_slice(&version.min_version.to_be_bytes());
                        data.extend_from_slice(&version.max_version.to_be_bytes());
                    }
                    if request_api_version >= 1 {
                        data.extend_from_slice(&0i32.to_be_bytes()); // throttle_time_ms
                    }
                }
                Ok(data)
            }
            KafkaResponse::Metadata => {
                // Metadata v4 (non-flexible) response.
                // Layout:
                //   correlation_id i32
                //   throttle_time_ms i32 (v3+)
                //   brokers: i32 length + [node_id i32, host string, port i32, rack nullable_string (v1+)]
                //   cluster_id nullable_string (v2+)
                //   controller_id i32 (v1+)
                //   topics: i32 length + [
                //     error_code i16,
                //     name string,
                //     is_internal bool (v1+),
                //     partitions: i32 length + [
                //       error_code i16,
                //       partition_index i32,
                //       leader_id i32,
                //       replica_nodes: i32 length + [i32 ...],
                //       isr_nodes: i32 length + [i32 ...],
                //     ],
                //   ]
                // We report ourselves as the single broker (node 1), then
                // enumerate whatever topics the handler was constructed with.
                // Every partition lists node 1 as leader + sole replica + ISR,
                // which is the right shape for a single-node mock cluster.
                const BROKER_NODE_ID: i32 = 1;
                let mut data = Vec::new();
                data.extend_from_slice(&correlation_id.to_be_bytes());
                data.extend_from_slice(&0i32.to_be_bytes()); // throttle_time_ms
                                                             // brokers array: 1 entry (self)
                data.extend_from_slice(&1i32.to_be_bytes());
                data.extend_from_slice(&BROKER_NODE_ID.to_be_bytes());
                push_kafka_string(&mut data, &self.advertised_host);
                data.extend_from_slice(&self.advertised_port.to_be_bytes());
                // rack: null
                data.extend_from_slice(&(-1i16).to_be_bytes());
                // cluster_id
                push_kafka_string(&mut data, "mockforge-cluster");
                // controller_id
                data.extend_from_slice(&BROKER_NODE_ID.to_be_bytes());
                // topics array
                data.extend_from_slice(&(self.topics.len() as i32).to_be_bytes());
                for topic in &self.topics {
                    data.extend_from_slice(&0i16.to_be_bytes()); // error_code
                    push_kafka_string(&mut data, &topic.name);
                    data.push(0); // is_internal = false
                    let partitions = topic.partitions.max(1);
                    data.extend_from_slice(&partitions.to_be_bytes());
                    for partition_index in 0..partitions {
                        data.extend_from_slice(&0i16.to_be_bytes()); // partition error_code
                        data.extend_from_slice(&partition_index.to_be_bytes());
                        data.extend_from_slice(&BROKER_NODE_ID.to_be_bytes()); // leader_id
                                                                               // replica_nodes: [BROKER_NODE_ID]
                        data.extend_from_slice(&1i32.to_be_bytes());
                        data.extend_from_slice(&BROKER_NODE_ID.to_be_bytes());
                        // isr_nodes: [BROKER_NODE_ID]
                        data.extend_from_slice(&1i32.to_be_bytes());
                        data.extend_from_slice(&BROKER_NODE_ID.to_be_bytes());
                    }
                }
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

/// Append a Kafka unsigned varint (7 bits/byte, continuation bit in MSB).
fn push_unsigned_varint(buf: &mut Vec<u8>, mut value: u32) {
    while (value & !0x7F) != 0 {
        buf.push(((value & 0x7F) | 0x80) as u8);
        value >>= 7;
    }
    buf.push(value as u8);
}

type Result<T> = std::result::Result<T, anyhow::Error>;

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== KafkaProtocolHandler Tests ====================

    #[test]
    fn test_protocol_handler_new() {
        let handler = KafkaProtocolHandler::new();
        assert!(!handler.api_versions.is_empty());
        assert!(handler.api_versions.contains_key(&0)); // Produce
        assert!(handler.api_versions.contains_key(&1)); // Fetch
        assert!(handler.api_versions.contains_key(&18)); // ApiVersions
    }

    #[test]
    fn test_protocol_handler_default() {
        let handler = KafkaProtocolHandler::default();
        assert!(!handler.api_versions.is_empty());
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
        // Metadata API (key 3) supports versions 0-4. Capped at v4 because
        // serialize_response only emits v4-shaped bodies.
        assert!(handler.is_api_version_supported(3, 0));
        assert!(handler.is_api_version_supported(3, 4));
        assert!(!handler.is_api_version_supported(3, 5));
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
    //
    // `data` here represents the request body AFTER the 4-byte length prefix
    // has been consumed by the broker — offset 0 = api_key high byte.

    /// Build a minimal v1 request header: api_key, api_version, correlation_id,
    /// empty client_id.
    fn build_header(
        api_key: i16,
        api_version: i16,
        correlation_id: i32,
        client_id: Option<&[u8]>,
    ) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&api_key.to_be_bytes());
        data.extend_from_slice(&api_version.to_be_bytes());
        data.extend_from_slice(&correlation_id.to_be_bytes());
        match client_id {
            None => data.extend_from_slice(&(-1i16).to_be_bytes()),
            Some(b) => {
                data.extend_from_slice(&(b.len() as i16).to_be_bytes());
                data.extend_from_slice(b);
            }
        }
        data
    }

    #[test]
    fn test_parse_request_too_short() {
        let handler = KafkaProtocolHandler::new();
        let data = vec![0u8; 5];
        assert!(handler.parse_request(&data).is_err());
    }

    #[test]
    fn test_parse_request_minimal_header() {
        let handler = KafkaProtocolHandler::new();
        let data = build_header(18, 0, 1, Some(b""));
        let request = handler.parse_request(&data).unwrap();
        assert_eq!(request.api_key, 18);
        assert_eq!(request.api_version, 0);
        assert_eq!(request.correlation_id, 1);
        assert_eq!(request.client_id, "");
        assert!(matches!(request.request_type, KafkaRequestType::ApiVersions));
    }

    #[test]
    fn test_parse_request_with_client_id() {
        let handler = KafkaProtocolHandler::new();
        let data = build_header(0, 7, 42, Some(b"test-client"));
        let request = handler.parse_request(&data).unwrap();
        assert_eq!(request.api_key, 0);
        assert_eq!(request.api_version, 7);
        assert_eq!(request.correlation_id, 42);
        assert_eq!(request.client_id, "test-client");
        assert!(matches!(request.request_type, KafkaRequestType::Produce));
    }

    #[test]
    fn test_parse_request_null_client_id() {
        let handler = KafkaProtocolHandler::new();
        let data = build_header(3, 0, 99, None);
        let request = handler.parse_request(&data).unwrap();
        assert_eq!(request.api_key, 3);
        assert_eq!(request.client_id, "");
        assert!(matches!(request.request_type, KafkaRequestType::Metadata));
    }

    #[test]
    fn test_parse_request_api_key_dispatch() {
        let handler = KafkaProtocolHandler::new();
        fn discriminant_name(t: &KafkaRequestType) -> &'static str {
            match t {
                KafkaRequestType::Produce => "Produce",
                KafkaRequestType::Fetch => "Fetch",
                KafkaRequestType::Metadata => "Metadata",
                KafkaRequestType::ListGroups => "ListGroups",
                KafkaRequestType::DescribeGroups => "DescribeGroups",
                KafkaRequestType::ApiVersions => "ApiVersions",
                KafkaRequestType::CreateTopics => "CreateTopics",
                KafkaRequestType::DeleteTopics => "DeleteTopics",
                KafkaRequestType::DescribeConfigs => "DescribeConfigs",
            }
        }
        let cases = [
            (0i16, "Produce"),
            (1, "Fetch"),
            (3, "Metadata"),
            (9, "ListGroups"),
            (15, "DescribeGroups"),
            (18, "ApiVersions"),
            (19, "CreateTopics"),
            (20, "DeleteTopics"),
            (32, "DescribeConfigs"),
        ];
        for (key, expected) in cases {
            let data = build_header(key, 0, 1, Some(b""));
            let request = handler.parse_request(&data).unwrap();
            assert_eq!(
                discriminant_name(&request.request_type),
                expected,
                "api_key={key} dispatched wrong"
            );
        }
    }

    #[test]
    fn test_parse_request_unsupported_api_defaults_to_api_versions() {
        let handler = KafkaProtocolHandler::new();
        let data = build_header(99, 0, 1, Some(b""));
        let request = handler.parse_request(&data).unwrap();
        assert!(matches!(request.request_type, KafkaRequestType::ApiVersions));
    }

    #[test]
    fn test_parse_request_invalid_client_id_length() {
        let handler = KafkaProtocolHandler::new();
        // client_id_len=100 but no bytes follow
        let mut data = build_header(18, 0, 1, Some(b""));
        data[8] = 0;
        data[9] = 100;
        assert!(handler.parse_request(&data).is_err());
    }

    #[test]
    fn test_parse_request_missing_client_id_length() {
        let handler = KafkaProtocolHandler::new();
        let data = vec![0u8; 9]; // Has header but client_id_len is cut off
        assert!(handler.parse_request(&data).is_err());
    }

    #[test]
    fn test_parse_request_max_values() {
        let handler = KafkaProtocolHandler::new();
        let data = build_header(0x7FFF, 0x7FFF, 0x7FFF_FFFF, Some(b""));
        let request = handler.parse_request(&data).unwrap();
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

        let result = handler.serialize_response(&response, correlation_id, 0);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert!(!data.is_empty());

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

        let result = handler.serialize_response(&response, correlation_id, 0);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert!(!data.is_empty());

        // Check correlation ID
        let corr_id = i32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        assert_eq!(corr_id, correlation_id);
    }

    #[test]
    fn test_serialize_response_metadata_v4() {
        // Minimal v4 Metadata response: one broker pointing at us, empty topics.
        // This unblocks `kcat -L` probes after the ApiVersions handshake.
        let handler = KafkaProtocolHandler::with_advertised_endpoint("mockforge", 19092);
        let data = handler.serialize_response(&KafkaResponse::Metadata, 7, 4).unwrap();

        // correlation_id
        assert_eq!(&data[0..4], &7i32.to_be_bytes());
        // throttle_time_ms
        assert_eq!(&data[4..8], &0i32.to_be_bytes());
        // brokers array length = 1
        assert_eq!(&data[8..12], &1i32.to_be_bytes());
        // broker node_id = 1
        assert_eq!(&data[12..16], &1i32.to_be_bytes());
        // host string: i16 length then "mockforge" (9 bytes)
        assert_eq!(&data[16..18], &9i16.to_be_bytes());
        assert_eq!(&data[18..27], b"mockforge");
        // port
        assert_eq!(&data[27..31], &19092i32.to_be_bytes());
        // rack (null nullable_string)
        assert_eq!(&data[31..33], &(-1i16).to_be_bytes());
        // cluster_id "mockforge-cluster" (17 bytes)
        assert_eq!(&data[33..35], &17i16.to_be_bytes());
        assert_eq!(&data[35..52], b"mockforge-cluster");
        // controller_id = 1
        assert_eq!(&data[52..56], &1i32.to_be_bytes());
        // topics array length = 0
        assert_eq!(&data[56..60], &0i32.to_be_bytes());
        assert_eq!(data.len(), 60);
    }

    #[test]
    fn test_serialize_response_metadata_v4_with_topics() {
        // When the handler is constructed with a topic list, the Metadata
        // response must enumerate each topic + each partition, with every
        // partition leader/replica/ISR pointing at broker node 1.
        let handler = KafkaProtocolHandler::with_topology(
            "mockforge",
            19092,
            vec![
                TopicMetadata {
                    name: "orders".to_string(),
                    partitions: 2,
                },
                TopicMetadata {
                    name: "events".to_string(),
                    partitions: 1,
                },
            ],
        );
        let data = handler.serialize_response(&KafkaResponse::Metadata, 42, 4).unwrap();

        // Header fields (same prefix as the empty-topics variant). Jump
        // straight to where the topics array starts: byte 56.
        let mut off = 56;

        // topics array length = 2
        assert_eq!(&data[off..off + 4], &2i32.to_be_bytes());
        off += 4;

        // --- topic 0: orders, 2 partitions -------------------------------
        assert_eq!(&data[off..off + 2], &0i16.to_be_bytes()); // error_code
        off += 2;
        assert_eq!(&data[off..off + 2], &6i16.to_be_bytes()); // name length
        off += 2;
        assert_eq!(&data[off..off + 6], b"orders");
        off += 6;
        assert_eq!(data[off], 0); // is_internal = false
        off += 1;
        assert_eq!(&data[off..off + 4], &2i32.to_be_bytes()); // partitions len
        off += 4;

        for expected_idx in 0..2i32 {
            assert_eq!(&data[off..off + 2], &0i16.to_be_bytes()); // partition err
            off += 2;
            assert_eq!(&data[off..off + 4], &expected_idx.to_be_bytes());
            off += 4;
            assert_eq!(&data[off..off + 4], &1i32.to_be_bytes()); // leader = 1
            off += 4;
            assert_eq!(&data[off..off + 4], &1i32.to_be_bytes()); // replicas len
            off += 4;
            assert_eq!(&data[off..off + 4], &1i32.to_be_bytes()); // replica id
            off += 4;
            assert_eq!(&data[off..off + 4], &1i32.to_be_bytes()); // ISR len
            off += 4;
            assert_eq!(&data[off..off + 4], &1i32.to_be_bytes()); // ISR id
            off += 4;
        }

        // --- topic 1: events, 1 partition --------------------------------
        assert_eq!(&data[off..off + 2], &0i16.to_be_bytes());
        off += 2;
        assert_eq!(&data[off..off + 2], &6i16.to_be_bytes());
        off += 2;
        assert_eq!(&data[off..off + 6], b"events");
        off += 6;
        assert_eq!(data[off], 0);
        off += 1;
        assert_eq!(&data[off..off + 4], &1i32.to_be_bytes()); // 1 partition
        off += 4;
        // one partition record = err(2) + idx(4) + leader(4) + replicas_len(4)
        // + replica(4) + isr_len(4) + isr(4) = 26 bytes
        off += 26;

        assert_eq!(data.len(), off, "unexpected trailing bytes");
    }

    #[test]
    fn test_metadata_zero_partitions_is_clamped_to_one() {
        // Guard against a nonsensical fixture that declared 0 partitions —
        // Kafka requires at least one partition per topic.
        let handler = KafkaProtocolHandler::with_topology(
            "mockforge",
            19092,
            vec![TopicMetadata {
                name: "t".to_string(),
                partitions: 0,
            }],
        );
        let data = handler.serialize_response(&KafkaResponse::Metadata, 1, 4).unwrap();
        // topic array length = 1
        assert_eq!(&data[56..60], &1i32.to_be_bytes());
        // skip error_code(2) + name_len(2) + "t"(1) + is_internal(1) => 6 bytes
        let partitions_len = i32::from_be_bytes([data[66], data[67], data[68], data[69]]);
        assert_eq!(partitions_len, 1);
    }

    #[test]
    fn test_metadata_advertised_max_version_is_four() {
        // We only serialize Metadata v4. Advertising anything higher would let
        // auto-negotiating clients pick a version we can't encode.
        let handler = KafkaProtocolHandler::new();
        let meta = handler.api_versions.get(&3).unwrap();
        assert_eq!(meta.max_version, 4);
    }

    #[test]
    fn test_serialize_response_produce() {
        let handler = KafkaProtocolHandler::new();
        let response = KafkaResponse::Produce;
        let correlation_id = 42;

        let result = handler.serialize_response(&response, correlation_id, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_response_fetch() {
        let handler = KafkaProtocolHandler::new();
        let response = KafkaResponse::Fetch;
        let correlation_id = 100;

        let result = handler.serialize_response(&response, correlation_id, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_response_list_groups() {
        let handler = KafkaProtocolHandler::new();
        let response = KafkaResponse::ListGroups;
        let correlation_id = 200;

        let result = handler.serialize_response(&response, correlation_id, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_response_describe_groups() {
        let handler = KafkaProtocolHandler::new();
        let response = KafkaResponse::DescribeGroups;
        let correlation_id = 300;

        let result = handler.serialize_response(&response, correlation_id, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_response_delete_topics() {
        let handler = KafkaProtocolHandler::new();
        let response = KafkaResponse::DeleteTopics;
        let correlation_id = 400;

        let result = handler.serialize_response(&response, correlation_id, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_response_describe_configs() {
        let handler = KafkaProtocolHandler::new();
        let response = KafkaResponse::DescribeConfigs;
        let correlation_id = 500;

        let result = handler.serialize_response(&response, correlation_id, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_serialize_response_negative_correlation_id() {
        let handler = KafkaProtocolHandler::new();
        let response = KafkaResponse::ApiVersions;
        let correlation_id = -1;

        let result = handler.serialize_response(&response, correlation_id, 0);
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

        let result = handler.serialize_response(&response, correlation_id, 0);
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
            (3, 0, 4),  // Metadata (capped at v4 — see serialize_response)
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
        let client_id = "a".repeat(1000);
        let data = build_header(18, 0, 1, Some(client_id.as_bytes()));
        let request = handler.parse_request(&data).unwrap();
        assert_eq!(request.client_id, client_id);
    }

    #[test]
    fn test_parse_request_invalid_utf8_client_id() {
        let handler = KafkaProtocolHandler::new();
        let data = build_header(18, 0, 1, Some(&[0xFF, 0xFF, 0xFF]));
        assert!(handler.parse_request(&data).is_err());
    }

    #[test]
    fn test_parse_request_kcat_apiversions_v3() {
        // Regression test: a real kcat/librdkafka ApiVersions v3 request
        // previously got misread (api_key=0/Produce, garbage correlation_id)
        // because parse_request was reading 4 bytes past the real header.
        let handler = KafkaProtocolHandler::new();
        let mut data = Vec::new();
        data.extend_from_slice(&18i16.to_be_bytes()); // api_key = ApiVersions
        data.extend_from_slice(&3i16.to_be_bytes()); // api_version = 3 (flexible)
        data.extend_from_slice(&1i32.to_be_bytes()); // correlation_id = 1
        data.extend_from_slice(&7i16.to_be_bytes()); // client_id length = 7
        data.extend_from_slice(b"rdkafka"); // client_id bytes
        data.push(0x00); // flexible header tag buffer
                         // body (compact strings + tag buffer) — parse_request ignores it
        data.push(0x08);
        data.extend_from_slice(b"rdkafka");
        data.push(0x06);
        data.extend_from_slice(b"1.8.2");
        data.push(0x00);

        let request = handler.parse_request(&data).unwrap();
        assert_eq!(request.api_key, 18);
        assert_eq!(request.api_version, 3);
        assert_eq!(request.correlation_id, 1);
        assert_eq!(request.client_id, "rdkafka");
        assert!(matches!(request.request_type, KafkaRequestType::ApiVersions));
    }

    #[test]
    fn test_serialize_response_api_versions_v3_flexible() {
        // librdkafka/kcat default to ApiVersions v3; the response must use
        // flexible encoding (compact array + tag buffers) or the client
        // disconnects. This test locks in the layout byte-for-byte.
        let handler = KafkaProtocolHandler::new();
        let data = handler.serialize_response(&KafkaResponse::ApiVersions, 0x12345678, 3).unwrap();

        // Header: correlation_id int32 (ApiVersions response header is v0
        // even for flexible bodies — no tag buffer here).
        assert_eq!(&data[0..4], &0x12345678i32.to_be_bytes());

        // Body: error_code int16 = 0
        assert_eq!(&data[4..6], &0i16.to_be_bytes());

        // Compact array length (unsigned varint): handler registers 11 api
        // entries, varint(11 + 1) = 0x0C in a single byte.
        let n = handler.api_versions.len() as u32;
        assert_eq!(n, 11);
        assert_eq!(data[6], 0x0C);

        // Each entry: api_key(i16) + min(i16) + max(i16) + tag_buffer(0x00) = 7 bytes.
        let entries_start = 7;
        let entries_end = entries_start + (n as usize) * 7;

        // Entries must be sorted ascending by api_key so clients can binary-search.
        let mut prev_key: i32 = -1;
        for chunk in data[entries_start..entries_end].chunks_exact(7) {
            let api_key = i16::from_be_bytes([chunk[0], chunk[1]]) as i32;
            assert!(api_key > prev_key, "api entries must be ascending by key");
            prev_key = api_key;
            // Tag buffer byte for each entry
            assert_eq!(chunk[6], 0);
        }

        // throttle_time_ms int32 = 0 + trailing tag buffer 0x00
        assert_eq!(&data[entries_end..entries_end + 4], &0i32.to_be_bytes());
        assert_eq!(data[entries_end + 4], 0);
        assert_eq!(data.len(), entries_end + 5);
    }

    #[test]
    fn test_serialize_response_api_versions_v0_non_flexible() {
        // v0 has no throttle_time_ms field.
        let handler = KafkaProtocolHandler::new();
        let data = handler.serialize_response(&KafkaResponse::ApiVersions, 7, 0).unwrap();
        let n = handler.api_versions.len();
        // 4 (corr) + 2 (err) + 4 (array len) + n*6 (entries, no per-entry tag)
        assert_eq!(data.len(), 4 + 2 + 4 + n * 6);
        let arr_len = i32::from_be_bytes([data[6], data[7], data[8], data[9]]);
        assert_eq!(arr_len as usize, n);
    }

    #[test]
    fn test_push_unsigned_varint() {
        let mut buf = Vec::new();
        push_unsigned_varint(&mut buf, 0);
        assert_eq!(buf, vec![0x00]);
        buf.clear();
        push_unsigned_varint(&mut buf, 127);
        assert_eq!(buf, vec![0x7F]);
        buf.clear();
        push_unsigned_varint(&mut buf, 128);
        assert_eq!(buf, vec![0x80, 0x01]);
        buf.clear();
        push_unsigned_varint(&mut buf, 300);
        assert_eq!(buf, vec![0xAC, 0x02]);
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
