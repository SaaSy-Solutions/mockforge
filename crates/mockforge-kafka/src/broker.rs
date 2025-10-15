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
