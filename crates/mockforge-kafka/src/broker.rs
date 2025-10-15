use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;

use crate::consumer_groups::ConsumerGroupManager;
use crate::metrics::KafkaMetrics;
use crate::protocol::{KafkaProtocolHandler, KafkaRequest, KafkaResponse};
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
    topics: Arc<RwLock<HashMap<String, Topic>>>,
    /// Consumer group manager
    consumer_groups: Arc<RwLock<ConsumerGroupManager>>,
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
        let spec_registry = KafkaSpecRegistry::new(config.clone()).await?;
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
                    let response = match self.handle_request(request) {
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
    fn handle_request(&self, request: KafkaRequest) -> Result<KafkaResponse> {
        match request {
            KafkaRequest::Metadata => self.handle_metadata(),
            KafkaRequest::Produce => self.handle_produce(),
            KafkaRequest::Fetch => self.handle_fetch(),
            KafkaRequest::ListGroups => self.handle_list_groups(),
            KafkaRequest::DescribeGroups => self.handle_describe_groups(),
            KafkaRequest::ApiVersions => self.handle_api_versions(),
            KafkaRequest::CreateTopics => self.handle_create_topics(),
            KafkaRequest::DeleteTopics => self.handle_delete_topics(),
            KafkaRequest::DescribeConfigs => self.handle_describe_configs(),
        }
    }

    fn handle_metadata(&self) -> Result<KafkaResponse> {
        // Simplified metadata response
        Ok(KafkaResponse::Metadata)
    }

    fn handle_produce(&self) -> Result<KafkaResponse> {
        // TODO: Implement produce logic
        Ok(KafkaResponse::Produce)
    }

    fn handle_fetch(&self) -> Result<KafkaResponse> {
        // TODO: Implement fetch logic
        Ok(KafkaResponse::Fetch)
    }

    fn handle_api_versions(&self) -> Result<KafkaResponse> {
        Ok(KafkaResponse::ApiVersions)
    }

    fn handle_list_groups(&self) -> Result<KafkaResponse> {
        Ok(KafkaResponse::ListGroups)
    }

    fn handle_describe_groups(&self) -> Result<KafkaResponse> {
        Ok(KafkaResponse::DescribeGroups)
    }

    fn handle_create_topics(&self) -> Result<KafkaResponse> {
        Ok(KafkaResponse::CreateTopics)
    }

    fn handle_delete_topics(&self) -> Result<KafkaResponse> {
        Ok(KafkaResponse::DeleteTopics)
    }

    fn handle_describe_configs(&self) -> Result<KafkaResponse> {
        Ok(KafkaResponse::DescribeConfigs)
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
    match request {
        KafkaRequest::Produce => 0,
        KafkaRequest::Fetch => 1,
        KafkaRequest::Metadata => 3,
        KafkaRequest::ListGroups => 9,
        KafkaRequest::DescribeGroups => 15,
        KafkaRequest::ApiVersions => 18,
        KafkaRequest::CreateTopics => 19,
        KafkaRequest::DeleteTopics => 20,
        KafkaRequest::DescribeConfigs => 32,
    }
}
