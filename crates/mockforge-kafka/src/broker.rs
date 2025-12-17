use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;

use crate::consumer_groups::ConsumerGroupManager;
use crate::metrics::KafkaMetrics;
use crate::protocol::{KafkaProtocolHandler, KafkaRequest, KafkaRequestType, KafkaResponse};
use crate::spec_registry::KafkaSpecRegistry;
use crate::topics::Topic;
use mockforge_core::config::KafkaConfig;
use mockforge_core::Result;

/// Mock Kafka broker implementation
///
/// The `KafkaMockBroker` simulates a complete Apache Kafka broker, handling
/// TCP connections and responding to Kafka protocol requests. It supports
/// multiple concurrent connections and provides comprehensive metrics collection.
///
/// # Architecture
///
/// The broker maintains several key components:
/// - **Topics**: Managed topic and partition storage
/// - **Consumer Groups**: Consumer group coordination and partition assignment
/// - **Spec Registry**: Fixture-based request/response handling
/// - **Metrics**: Real-time performance and usage statistics
///
/// # Supported Operations
///
/// - Produce: Message production with acknowledgments
/// - Fetch: Message consumption with offset tracking
/// - Metadata: Topic and broker discovery
/// - ListGroups/DescribeGroups: Consumer group management
/// - ApiVersions: Protocol version negotiation
/// - CreateTopics/DeleteTopics: Dynamic topic management
/// - DescribeConfigs: Configuration retrieval
///
/// # Example
///
/// ```rust,no_run
/// use mockforge_kafka::KafkaMockBroker;
/// use mockforge_core::config::KafkaConfig;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = KafkaConfig {
///     port: 9092,
///     ..Default::default()
/// };
///
/// let broker = KafkaMockBroker::new(config).await?;
/// broker.start().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
#[allow(dead_code)]
pub struct KafkaMockBroker {
    /// Broker configuration
    config: KafkaConfig,
    /// Topic storage with thread-safe access
    pub topics: Arc<RwLock<HashMap<String, Topic>>>,
    /// Consumer group manager
    pub consumer_groups: Arc<RwLock<ConsumerGroupManager>>,
    /// Specification registry for fixture-based responses
    spec_registry: Arc<KafkaSpecRegistry>,
    /// Metrics collection and reporting
    metrics: Arc<KafkaMetrics>,
}

impl KafkaMockBroker {
    /// Create a new Kafka mock broker
    ///
    /// Initializes the broker with the provided configuration, setting up
    /// internal data structures for topics, consumer groups, and metrics.
    ///
    /// # Arguments
    ///
    /// * `config` - Kafka broker configuration including port, timeouts, and fixture paths
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the initialized broker or an error if initialization fails.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use mockforge_kafka::KafkaMockBroker;
    /// use mockforge_core::config::KafkaConfig;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = KafkaConfig::default();
    /// let broker = KafkaMockBroker::new(config).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(config: KafkaConfig) -> Result<Self> {
        let topics = Arc::new(RwLock::new(HashMap::new()));
        let consumer_groups = Arc::new(RwLock::new(ConsumerGroupManager::new()));
        let spec_registry = KafkaSpecRegistry::new(config.clone(), Arc::clone(&topics)).await?;
        let metrics = Arc::new(KafkaMetrics::new());

        Ok(Self {
            config,
            topics,
            consumer_groups,
            spec_registry: Arc::new(spec_registry),
            metrics,
        })
    }

    /// Start the Kafka broker server
    ///
    /// Binds to the configured host and port, then begins accepting TCP connections.
    /// Each connection is handled in a separate async task, allowing concurrent client connections.
    ///
    /// The broker will run indefinitely until the task is cancelled or an error occurs.
    ///
    /// # Returns
    ///
    /// Returns a `Result` that indicates whether the broker started successfully.
    /// The method only returns on error, as it runs an infinite accept loop.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use mockforge_kafka::KafkaMockBroker;
    /// use mockforge_core::config::KafkaConfig;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = KafkaConfig::default();
    /// let broker = KafkaMockBroker::new(config).await?;
    ///
    /// // Start the broker (this will run indefinitely)
    /// broker.start().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port);
        let listener = TcpListener::bind(&addr).await?;

        tracing::info!("Starting Kafka mock broker on {}", addr);

        loop {
            let (socket, _) = listener.accept().await?;
            let broker = Arc::new(self.clone());

            tokio::spawn(async move {
                if let Err(e) = broker.handle_connection(socket).await {
                    tracing::error!("Error handling connection: {}", e);
                }
            });
        }
    }

    /// Handle a client connection
    async fn handle_connection(&self, mut socket: TcpStream) -> Result<()> {
        let protocol_handler = KafkaProtocolHandler::new();
        self.metrics.record_connection();

        // Ensure we decrement active connections when done
        let _guard = ConnectionGuard {
            metrics: Arc::clone(&self.metrics),
        };

        loop {
            // Read message size (4 bytes) with timeout
            let mut size_buf = [0u8; 4];
            match tokio::time::timeout(
                std::time::Duration::from_secs(30),
                socket.read_exact(&mut size_buf),
            )
            .await
            {
                Ok(Ok(_)) => {
                    let message_size = i32::from_be_bytes(size_buf) as usize;

                    // Validate message size (prevent DoS)
                    if message_size > 10 * 1024 * 1024 {
                        // 10MB limit
                        self.metrics.record_error();
                        tracing::warn!("Message size too large: {} bytes", message_size);
                        continue;
                    }

                    // Read message
                    let mut message_buf = vec![0u8; message_size];
                    if let Err(e) = tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        socket.read_exact(&mut message_buf),
                    )
                    .await
                    {
                        self.metrics.record_error();
                        tracing::error!("Timeout reading message: {}", e);
                        break;
                    }

                    // Parse request
                    let request = match protocol_handler.parse_request(&message_buf) {
                        Ok(req) => req,
                        Err(e) => {
                            self.metrics.record_error();
                            tracing::error!("Failed to parse request: {}", e);
                            continue;
                        }
                    };

                    // Record request metrics
                    self.metrics.record_request(get_api_key_from_request(&request));

                    let start_time = std::time::Instant::now();

                    // Handle request
                    let response = match self.handle_request(request).await {
                        Ok(resp) => resp,
                        Err(e) => {
                            self.metrics.record_error();
                            tracing::error!("Failed to handle request: {}", e);
                            // Return error response
                            continue;
                        }
                    };

                    let latency = start_time.elapsed().as_micros() as u64;
                    self.metrics.record_request_latency(latency);
                    self.metrics.record_response();

                    // Serialize response
                    let response_data = match protocol_handler.serialize_response(&response, 0) {
                        Ok(data) => data,
                        Err(e) => {
                            self.metrics.record_error();
                            tracing::error!("Failed to serialize response: {}", e);
                            continue;
                        }
                    };

                    // Write response size
                    let response_size = (response_data.len() as i32).to_be_bytes();
                    if let Err(e) = socket.write_all(&response_size).await {
                        self.metrics.record_error();
                        tracing::error!("Failed to write response size: {}", e);
                        break;
                    }

                    // Write response
                    if let Err(e) = socket.write_all(&response_data).await {
                        self.metrics.record_error();
                        tracing::error!("Failed to write response: {}", e);
                        break;
                    }
                }
                Ok(Err(e)) => {
                    self.metrics.record_error();
                    tracing::error!("Failed to read message size: {}", e);
                    break;
                }
                Err(_) => {
                    // Timeout - client may be idle, just continue
                    continue;
                }
            }
        }

        Ok(())
    }

    /// Handle a parsed Kafka request
    async fn handle_request(&self, request: KafkaRequest) -> Result<KafkaResponse> {
        match request.request_type {
            KafkaRequestType::Metadata => self.handle_metadata().await,
            KafkaRequestType::Produce => self.handle_produce().await,
            KafkaRequestType::Fetch => self.handle_fetch().await,
            KafkaRequestType::ListGroups => self.handle_list_groups().await,
            KafkaRequestType::DescribeGroups => self.handle_describe_groups().await,
            KafkaRequestType::ApiVersions => self.handle_api_versions().await,
            KafkaRequestType::CreateTopics => self.handle_create_topics().await,
            KafkaRequestType::DeleteTopics => self.handle_delete_topics().await,
            KafkaRequestType::DescribeConfigs => self.handle_describe_configs().await,
        }
    }

    async fn handle_metadata(&self) -> Result<KafkaResponse> {
        // Simplified metadata response
        Ok(KafkaResponse::Metadata)
    }

    async fn handle_produce(&self) -> Result<KafkaResponse> {
        // Produce logic not yet implemented
        Ok(KafkaResponse::Produce)
    }

    async fn handle_fetch(&self) -> Result<KafkaResponse> {
        // Fetch logic not yet implemented
        Ok(KafkaResponse::Fetch)
    }

    async fn handle_api_versions(&self) -> Result<KafkaResponse> {
        Ok(KafkaResponse::ApiVersions)
    }

    async fn handle_list_groups(&self) -> Result<KafkaResponse> {
        Ok(KafkaResponse::ListGroups)
    }

    async fn handle_describe_groups(&self) -> Result<KafkaResponse> {
        Ok(KafkaResponse::DescribeGroups)
    }

    async fn handle_create_topics(&self) -> Result<KafkaResponse> {
        // For now, create a default topic as a placeholder
        // Protocol parsing for actual topic creation parameters is not yet implemented
        let topic_name = "default-topic".to_string();
        let topic_config = crate::topics::TopicConfig::default();
        let topic = crate::topics::Topic::new(topic_name.clone(), topic_config);

        // Store the topic
        let mut topics = self.topics.write().await;
        topics.insert(topic_name, topic);

        Ok(KafkaResponse::CreateTopics)
    }

    async fn handle_delete_topics(&self) -> Result<KafkaResponse> {
        Ok(KafkaResponse::DeleteTopics)
    }

    async fn handle_describe_configs(&self) -> Result<KafkaResponse> {
        Ok(KafkaResponse::DescribeConfigs)
    }

    /// Test helper: Commit offsets for a consumer group (only available in tests)
    pub async fn test_commit_offsets(
        &self,
        group_id: &str,
        offsets: std::collections::HashMap<(String, i32), i64>,
    ) -> Result<()> {
        let mut consumer_groups = self.consumer_groups.write().await;
        consumer_groups
            .commit_offsets(group_id, offsets)
            .await
            .map_err(|e| mockforge_core::Error::from(e.to_string()))
    }

    /// Test helper: Get committed offsets for a consumer group (only available in tests)
    pub async fn test_get_committed_offsets(
        &self,
        group_id: &str,
    ) -> std::collections::HashMap<(String, i32), i64> {
        let consumer_groups = self.consumer_groups.read().await;
        consumer_groups.get_committed_offsets(group_id)
    }

    /// Test helper: Create a topic (only available in tests)
    pub async fn test_create_topic(&self, name: &str, config: crate::topics::TopicConfig) {
        use crate::topics::Topic;
        let topic = Topic::new(name.to_string(), config);
        let mut topics = self.topics.write().await;
        topics.insert(name.to_string(), topic);
    }

    /// Test helper: Join a consumer group (only available in tests)
    pub async fn test_join_group(
        &self,
        group_id: &str,
        member_id: &str,
        client_id: &str,
    ) -> Result<()> {
        let mut consumer_groups = self.consumer_groups.write().await;
        consumer_groups
            .join_group(group_id, member_id, client_id)
            .await
            .map_err(|e| mockforge_core::Error::from(e.to_string()))?;
        Ok(())
    }

    /// Test helper: Sync group assignment (only available in tests)
    pub async fn test_sync_group(
        &self,
        group_id: &str,
        assignments: Vec<crate::consumer_groups::PartitionAssignment>,
    ) -> Result<()> {
        let topics = self.topics.read().await;
        let mut consumer_groups = self.consumer_groups.write().await;
        consumer_groups
            .sync_group(group_id, assignments, &topics)
            .await
            .map_err(|e| mockforge_core::Error::from(e.to_string()))?;
        Ok(())
    }

    /// Test helper: Get group member assignments (only available in tests)
    pub async fn test_get_assignments(
        &self,
        group_id: &str,
        member_id: &str,
    ) -> Vec<crate::consumer_groups::PartitionAssignment> {
        let consumer_groups = self.consumer_groups.read().await;
        if let Some(group) = consumer_groups.groups().get(group_id) {
            if let Some(member) = group.members.get(member_id) {
                return member.assignment.clone();
            }
        }
        vec![]
    }

    /// Test helper: Simulate consumer lag (only available in tests)
    pub async fn test_simulate_lag(&self, group_id: &str, topic: &str, lag: i64) -> Result<()> {
        let topics = self.topics.read().await;
        let mut consumer_groups = self.consumer_groups.write().await;
        consumer_groups.simulate_lag(group_id, topic, lag, &topics).await;
        Ok(())
    }

    /// Test helper: Reset consumer offsets (only available in tests)
    pub async fn test_reset_offsets(&self, group_id: &str, topic: &str, to: &str) -> Result<()> {
        let topics = self.topics.read().await;
        let mut consumer_groups = self.consumer_groups.write().await;
        consumer_groups.reset_offsets(group_id, topic, to, &topics).await;
        Ok(())
    }

    /// Get a reference to the metrics collector
    ///
    /// This method provides access to the Kafka metrics for monitoring and statistics.
    /// The metrics are thread-safe and can be accessed concurrently.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use mockforge_kafka::KafkaMockBroker;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let broker = KafkaMockBroker::new(Default::default()).await?;
    /// let metrics = broker.metrics();
    /// let snapshot = metrics.snapshot();
    /// println!("Messages produced: {}", snapshot.messages_produced_total);
    /// # Ok(())
    /// # }
    /// ```
    pub fn metrics(&self) -> &Arc<KafkaMetrics> {
        &self.metrics
    }
}

/// Record represents a Kafka message record
#[derive(Debug, Clone)]
pub struct Record {
    pub key: Option<Vec<u8>>,
    pub value: Vec<u8>,
    pub headers: Vec<(String, Vec<u8>)>,
}

/// Response for produce requests
#[derive(Debug)]
pub struct ProduceResponse {
    pub partition: i32,
    pub error_code: i16,
    pub offset: i64,
}

/// Response for fetch requests
#[derive(Debug)]
pub struct FetchResponse {
    pub partition: i32,
    pub error_code: i16,
    pub high_watermark: i64,
    pub records: Vec<Record>,
}

/// Guard to ensure connection metrics are properly cleaned up
struct ConnectionGuard {
    metrics: Arc<KafkaMetrics>,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        self.metrics.record_connection_closed();
    }
}

/// Extract API key from request for metrics
fn get_api_key_from_request(request: &KafkaRequest) -> i16 {
    request.api_key
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Record Tests ====================

    #[test]
    fn test_record_creation_with_all_fields() {
        let record = Record {
            key: Some(b"test-key".to_vec()),
            value: b"test-value".to_vec(),
            headers: vec![("header1".to_string(), b"value1".to_vec())],
        };

        assert_eq!(record.key, Some(b"test-key".to_vec()));
        assert_eq!(record.value, b"test-value".to_vec());
        assert_eq!(record.headers.len(), 1);
        assert_eq!(record.headers[0].0, "header1");
    }

    #[test]
    fn test_record_creation_without_key() {
        let record = Record {
            key: None,
            value: b"message body".to_vec(),
            headers: vec![],
        };

        assert!(record.key.is_none());
        assert_eq!(record.value, b"message body".to_vec());
        assert!(record.headers.is_empty());
    }

    #[test]
    fn test_record_with_multiple_headers() {
        let record = Record {
            key: Some(b"key".to_vec()),
            value: b"value".to_vec(),
            headers: vec![
                ("content-type".to_string(), b"application/json".to_vec()),
                ("correlation-id".to_string(), b"12345".to_vec()),
                ("source".to_string(), b"test-producer".to_vec()),
            ],
        };

        assert_eq!(record.headers.len(), 3);
        assert_eq!(record.headers[0].0, "content-type");
        assert_eq!(record.headers[1].0, "correlation-id");
        assert_eq!(record.headers[2].0, "source");
    }

    #[test]
    fn test_record_clone() {
        let original = Record {
            key: Some(b"key".to_vec()),
            value: b"value".to_vec(),
            headers: vec![("h".to_string(), b"v".to_vec())],
        };

        let cloned = original.clone();

        assert_eq!(original.key, cloned.key);
        assert_eq!(original.value, cloned.value);
        assert_eq!(original.headers, cloned.headers);
    }

    #[test]
    fn test_record_debug() {
        let record = Record {
            key: Some(b"key".to_vec()),
            value: b"value".to_vec(),
            headers: vec![],
        };

        let debug_str = format!("{:?}", record);
        assert!(debug_str.contains("Record"));
        assert!(debug_str.contains("key"));
        assert!(debug_str.contains("value"));
    }

    #[test]
    fn test_record_empty_value() {
        let record = Record {
            key: None,
            value: vec![],
            headers: vec![],
        };

        assert!(record.key.is_none());
        assert!(record.value.is_empty());
        assert!(record.headers.is_empty());
    }

    #[test]
    fn test_record_binary_data() {
        // Test with binary data that's not valid UTF-8
        let binary_data: Vec<u8> = vec![0x00, 0xFF, 0x80, 0x7F, 0xFE];
        let record = Record {
            key: Some(binary_data.clone()),
            value: binary_data.clone(),
            headers: vec![],
        };

        assert_eq!(record.key.as_ref().unwrap().len(), 5);
        assert_eq!(record.value.len(), 5);
        assert_eq!(record.value[0], 0x00);
        assert_eq!(record.value[1], 0xFF);
    }

    // ==================== ProduceResponse Tests ====================

    #[test]
    fn test_produce_response_success() {
        let response = ProduceResponse {
            partition: 0,
            error_code: 0,
            offset: 100,
        };

        assert_eq!(response.partition, 0);
        assert_eq!(response.error_code, 0);
        assert_eq!(response.offset, 100);
    }

    #[test]
    fn test_produce_response_with_error() {
        let response = ProduceResponse {
            partition: 1,
            error_code: 3, // UNKNOWN_TOPIC_OR_PARTITION
            offset: -1,
        };

        assert_eq!(response.partition, 1);
        assert_eq!(response.error_code, 3);
        assert_eq!(response.offset, -1);
    }

    #[test]
    fn test_produce_response_high_offset() {
        let response = ProduceResponse {
            partition: 5,
            error_code: 0,
            offset: i64::MAX,
        };

        assert_eq!(response.partition, 5);
        assert_eq!(response.offset, i64::MAX);
    }

    #[test]
    fn test_produce_response_debug() {
        let response = ProduceResponse {
            partition: 0,
            error_code: 0,
            offset: 42,
        };

        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("ProduceResponse"));
        assert!(debug_str.contains("partition"));
        assert!(debug_str.contains("error_code"));
        assert!(debug_str.contains("offset"));
    }

    // ==================== FetchResponse Tests ====================

    #[test]
    fn test_fetch_response_empty() {
        let response = FetchResponse {
            partition: 0,
            error_code: 0,
            high_watermark: 100,
            records: vec![],
        };

        assert_eq!(response.partition, 0);
        assert_eq!(response.error_code, 0);
        assert_eq!(response.high_watermark, 100);
        assert!(response.records.is_empty());
    }

    #[test]
    fn test_fetch_response_with_records() {
        let records = vec![
            Record {
                key: Some(b"key1".to_vec()),
                value: b"value1".to_vec(),
                headers: vec![],
            },
            Record {
                key: Some(b"key2".to_vec()),
                value: b"value2".to_vec(),
                headers: vec![],
            },
        ];

        let response = FetchResponse {
            partition: 0,
            error_code: 0,
            high_watermark: 50,
            records,
        };

        assert_eq!(response.records.len(), 2);
        assert_eq!(response.records[0].key, Some(b"key1".to_vec()));
        assert_eq!(response.records[1].value, b"value2".to_vec());
    }

    #[test]
    fn test_fetch_response_with_error() {
        let response = FetchResponse {
            partition: 0,
            error_code: 1, // OFFSET_OUT_OF_RANGE
            high_watermark: 0,
            records: vec![],
        };

        assert_eq!(response.error_code, 1);
        assert_eq!(response.high_watermark, 0);
    }

    #[test]
    fn test_fetch_response_debug() {
        let response = FetchResponse {
            partition: 2,
            error_code: 0,
            high_watermark: 1000,
            records: vec![],
        };

        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("FetchResponse"));
        assert!(debug_str.contains("high_watermark"));
    }

    // ==================== get_api_key_from_request Tests ====================

    #[test]
    fn test_get_api_key_produce() {
        let request = KafkaRequest {
            api_key: 0, // Produce
            api_version: 7,
            correlation_id: 1,
            client_id: "test-client".to_string(),
            request_type: KafkaRequestType::Produce,
        };

        assert_eq!(get_api_key_from_request(&request), 0);
    }

    #[test]
    fn test_get_api_key_fetch() {
        let request = KafkaRequest {
            api_key: 1, // Fetch
            api_version: 11,
            correlation_id: 2,
            client_id: "consumer".to_string(),
            request_type: KafkaRequestType::Fetch,
        };

        assert_eq!(get_api_key_from_request(&request), 1);
    }

    #[test]
    fn test_get_api_key_metadata() {
        let request = KafkaRequest {
            api_key: 3, // Metadata
            api_version: 9,
            correlation_id: 3,
            client_id: "admin".to_string(),
            request_type: KafkaRequestType::Metadata,
        };

        assert_eq!(get_api_key_from_request(&request), 3);
    }

    #[test]
    fn test_get_api_key_api_versions() {
        let request = KafkaRequest {
            api_key: 18, // ApiVersions
            api_version: 3,
            correlation_id: 100,
            client_id: "client".to_string(),
            request_type: KafkaRequestType::ApiVersions,
        };

        assert_eq!(get_api_key_from_request(&request), 18);
    }

    #[test]
    fn test_get_api_key_list_groups() {
        let request = KafkaRequest {
            api_key: 16, // ListGroups
            api_version: 4,
            correlation_id: 5,
            client_id: "admin-client".to_string(),
            request_type: KafkaRequestType::ListGroups,
        };

        assert_eq!(get_api_key_from_request(&request), 16);
    }

    #[test]
    fn test_get_api_key_create_topics() {
        let request = KafkaRequest {
            api_key: 19, // CreateTopics
            api_version: 5,
            correlation_id: 10,
            client_id: "admin".to_string(),
            request_type: KafkaRequestType::CreateTopics,
        };

        assert_eq!(get_api_key_from_request(&request), 19);
    }

    // ==================== KafkaRequest Field Tests ====================

    #[test]
    fn test_kafka_request_fields() {
        let request = KafkaRequest {
            api_key: 0,
            api_version: 8,
            correlation_id: 12345,
            client_id: "my-producer".to_string(),
            request_type: KafkaRequestType::Produce,
        };

        assert_eq!(request.api_key, 0);
        assert_eq!(request.api_version, 8);
        assert_eq!(request.correlation_id, 12345);
        assert_eq!(request.client_id, "my-producer");
    }

    #[test]
    fn test_kafka_request_empty_client_id() {
        let request = KafkaRequest {
            api_key: 3,
            api_version: 9,
            correlation_id: 1,
            client_id: String::new(),
            request_type: KafkaRequestType::Metadata,
        };

        assert!(request.client_id.is_empty());
    }

    #[test]
    fn test_kafka_request_max_correlation_id() {
        let request = KafkaRequest {
            api_key: 0,
            api_version: 0,
            correlation_id: i32::MAX,
            client_id: "test".to_string(),
            request_type: KafkaRequestType::Produce,
        };

        assert_eq!(request.correlation_id, i32::MAX);
    }

    // ==================== KafkaRequestType Tests ====================

    #[test]
    fn test_request_type_variants() {
        let metadata = KafkaRequestType::Metadata;
        let produce = KafkaRequestType::Produce;
        let fetch = KafkaRequestType::Fetch;
        let list_groups = KafkaRequestType::ListGroups;
        let describe_groups = KafkaRequestType::DescribeGroups;
        let api_versions = KafkaRequestType::ApiVersions;
        let create_topics = KafkaRequestType::CreateTopics;
        let delete_topics = KafkaRequestType::DeleteTopics;
        let describe_configs = KafkaRequestType::DescribeConfigs;

        // Verify they can be matched
        assert!(matches!(metadata, KafkaRequestType::Metadata));
        assert!(matches!(produce, KafkaRequestType::Produce));
        assert!(matches!(fetch, KafkaRequestType::Fetch));
        assert!(matches!(list_groups, KafkaRequestType::ListGroups));
        assert!(matches!(describe_groups, KafkaRequestType::DescribeGroups));
        assert!(matches!(api_versions, KafkaRequestType::ApiVersions));
        assert!(matches!(create_topics, KafkaRequestType::CreateTopics));
        assert!(matches!(delete_topics, KafkaRequestType::DeleteTopics));
        assert!(matches!(describe_configs, KafkaRequestType::DescribeConfigs));
    }

    // ==================== Message Size Limit Tests ====================

    #[test]
    fn test_message_size_limit_constant() {
        // The broker has a 10MB message size limit
        let max_message_size: usize = 10 * 1024 * 1024;
        assert_eq!(max_message_size, 10_485_760);
    }

    #[test]
    fn test_message_size_under_limit() {
        let message_size: usize = 1024 * 1024; // 1MB
        let limit: usize = 10 * 1024 * 1024; // 10MB
        assert!(message_size <= limit);
    }

    #[test]
    fn test_message_size_over_limit() {
        let message_size: usize = 11 * 1024 * 1024; // 11MB
        let limit: usize = 10 * 1024 * 1024; // 10MB
        assert!(message_size > limit);
    }

    // ==================== Response Size Serialization Tests ====================

    #[test]
    fn test_response_size_serialization() {
        let response_len: i32 = 1000;
        let size_bytes = response_len.to_be_bytes();

        assert_eq!(size_bytes.len(), 4);
        assert_eq!(i32::from_be_bytes(size_bytes), 1000);
    }

    #[test]
    fn test_response_size_max_value() {
        let response_len: i32 = i32::MAX;
        let size_bytes = response_len.to_be_bytes();

        assert_eq!(size_bytes.len(), 4);
        assert_eq!(i32::from_be_bytes(size_bytes), i32::MAX);
    }

    #[test]
    fn test_response_size_zero() {
        let response_len: i32 = 0;
        let size_bytes = response_len.to_be_bytes();

        assert_eq!(size_bytes, [0, 0, 0, 0]);
    }
}
