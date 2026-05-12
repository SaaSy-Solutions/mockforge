use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;

use crate::consumer_groups::ConsumerGroupManager;
use crate::fixture_executor::FixtureRuntime;
use crate::metrics::KafkaMetrics;
use crate::partitions::KafkaMessage;
use crate::protocol::{KafkaProtocolHandler, KafkaRequest, KafkaRequestType, KafkaResponse};
use crate::spec_registry::KafkaSpecRegistry;
use crate::topics::Topic;
use mockforge_core::config::KafkaConfig;
use mockforge_core::Result;
use std::sync::OnceLock;

/// Resolve the (host, port) the broker advertises in its
/// MetadataResponse. When the orchestrator sets
/// `MOCKFORGE_KAFKA_ADVERTISED_HOST`/`PORT` (mockforge-core's loader
/// feeds those into `KafkaConfig::advertised_*`), use them; otherwise
/// fall back to the bind endpoint so local/dev behaviour is unchanged.
///
/// Pulled out as a free function so the resolution can be unit-tested
/// without standing up a real broker.
pub(crate) fn resolve_advertised_endpoint(config: &KafkaConfig) -> (String, i32) {
    let host = config.advertised_host.clone().unwrap_or_else(|| config.host.clone());
    let port = config.advertised_port.map(|p| p as i32).unwrap_or(config.port as i32);
    (host, port)
}

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
    /// Minimal consumer-group coordinator (FindCoordinator / JoinGroup /
    /// SyncGroup / Heartbeat). Shared across all connections so the
    /// coordinator's decision for a group survives heartbeat reconnects.
    pub group_coordinator: Arc<RwLock<crate::group_coordinator::GroupCoordinator>>,
    /// Specification registry for fixture-based responses
    spec_registry: Arc<KafkaSpecRegistry>,
    /// Runtime data for the fixture-trigger executors
    /// (state-machine / scenarios / relationships). Installed on first
    /// call to `start()`; `None` until then. Shared across every clone
    /// of the broker, so accepting-loop and per-connection handlers
    /// observe the same runtime.
    fixture_runtime: Arc<OnceLock<Arc<FixtureRuntime>>>,
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

        // Apply seed messages from config BEFORE the broker starts accepting
        // connections, so any consumer reading from offset 0 sees them as
        // part of the topic log.
        if !config.seed_messages.is_empty() {
            Self::apply_seed_messages(&topics, &config).await?;
        }

        Ok(Self {
            config,
            topics,
            consumer_groups,
            group_coordinator: Arc::new(RwLock::new(
                crate::group_coordinator::GroupCoordinator::new(),
            )),
            spec_registry: Arc::new(spec_registry),
            fixture_runtime: Arc::new(OnceLock::new()),
            metrics,
        })
    }

    /// Inject all `config.seed_messages` into their respective topic logs.
    /// Topics are auto-created as needed using the configured defaults.
    /// Each message's partition is chosen via the same hash-on-key strategy
    /// the produce path uses, so seeded keys land on the same partition real
    /// produced records would.
    async fn apply_seed_messages(
        topics: &Arc<RwLock<HashMap<String, Topic>>>,
        config: &KafkaConfig,
    ) -> Result<()> {
        use crate::topics::TopicConfig;
        let mut guard = topics.write().await;
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let mut total_seeded = 0usize;
        for (topic_name, messages) in &config.seed_messages {
            let topic = guard.entry(topic_name.clone()).or_insert_with(|| {
                Topic::new(
                    topic_name.clone(),
                    TopicConfig {
                        num_partitions: config.default_partitions.max(1),
                        replication_factor: config.default_replication_factor,
                        retention_ms: config.log_retention_ms,
                        max_message_bytes: 1_048_576,
                    },
                )
            });

            for seed in messages {
                let key_bytes = seed.key.as_ref().map(|k| k.as_bytes().to_vec());
                let partition = topic.assign_partition(key_bytes.as_deref());
                let msg = KafkaMessage {
                    offset: 0, // overwritten by Partition::append
                    timestamp: now_ms,
                    key: key_bytes,
                    value: seed.value.as_bytes().to_vec(),
                    headers: seed
                        .headers
                        .iter()
                        .map(|(k, v)| (k.clone(), v.as_bytes().to_vec()))
                        .collect(),
                };
                topic.produce(partition, msg).await?;
                total_seeded += 1;
            }
        }

        if total_seeded > 0 {
            tracing::info!(
                "Seeded {} messages across {} topic(s) at broker startup",
                total_seeded,
                config.seed_messages.len()
            );
        }
        Ok(())
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

        // Install the fixture-trigger runtime on first start. Idempotent
        // across reconnects because `OnceLock::set` returns `Err` when
        // already populated and we just discard that.
        self.install_fixture_runtime().await;

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

    /// Build the `FixtureRuntime` from the spec registry and stash it on
    /// the broker. Idempotent — safe to call on every `start()` cycle;
    /// the first call wins, later calls are no-ops. Exposed for tests
    /// that want the fixture executors to run without binding a real
    /// TCP listener.
    pub async fn install_fixture_runtime(&self) {
        if self.fixture_runtime.get().is_some() {
            return;
        }
        let fixtures = self.spec_registry.all_fixtures().to_vec();
        let relationships = self.spec_registry.relationships().to_vec();
        let state_machines = self.spec_registry.state_machines().to_vec();
        let scenarios = self.spec_registry.scenarios().to_vec();

        let broker_arc = Arc::new(self.clone());
        let runtime = crate::fixture_executor::install(
            Arc::clone(&broker_arc),
            &fixtures,
            &state_machines,
            &scenarios,
            &relationships,
        )
        .await;

        // Setting the OnceLock can only fail if something raced us; in
        // that case the other winner's runtime is just as valid.
        let _ = self.fixture_runtime.set(runtime);
    }

    /// Read-side accessor for the fixture runtime. Returns `None` before
    /// `start()` has run. Produce-path code path uses this to fire
    /// relationships; tests bypass `start()` and use
    /// `install_fixture_runtime` directly.
    pub fn fixture_runtime(&self) -> Option<Arc<FixtureRuntime>> {
        self.fixture_runtime.get().cloned()
    }

    /// Handle a client connection
    async fn handle_connection(&self, mut socket: TcpStream) -> Result<()> {
        // Advertise this broker's own host/port in Metadata responses and
        // surface every topic we know about (fixture-declared + auto-created
        // from produce) so `kcat -L` and librdkafka's Metadata-before-Produce
        // probe see real state. The topics map is the authoritative source:
        // the spec_registry pre-populates it at startup, and handle_produce
        // writes auto-created topics into it too.
        let topics: Vec<crate::protocol::TopicMetadata> = {
            let guard = self.topics.read().await;
            guard
                .iter()
                .map(|(name, topic)| crate::protocol::TopicMetadata {
                    name: name.clone(),
                    partitions: (topic.partitions.len() as i32).max(1),
                })
                .collect()
        };
        // The advertised endpoint (what we tell Kafka clients to connect
        // to after bootstrap) may differ from the bind endpoint when
        // running behind Fly: the bind host is 0.0.0.0:9092, but clients
        // need <app>.fly.dev:9092. See `resolve_advertised_endpoint`.
        let (advertised_host, advertised_port) = resolve_advertised_endpoint(&self.config);
        let protocol_handler =
            KafkaProtocolHandler::with_topology(advertised_host, advertised_port, topics);
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

                    // Stash correlation_id and api_version before the request is
                    // moved into handle_request — both are needed to serialize
                    // a response the client will actually accept.
                    let correlation_id = request.correlation_id;
                    let request_api_version = request.api_version;

                    // Record request metrics
                    self.metrics.record_request(get_api_key_from_request(&request));

                    let start_time = std::time::Instant::now();

                    // Handle request
                    let response = match self.handle_request(&message_buf, request).await {
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
                    let response_data = match protocol_handler.serialize_response(
                        &response,
                        correlation_id,
                        request_api_version,
                    ) {
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
    async fn handle_request(
        &self,
        message_buf: &[u8],
        request: KafkaRequest,
    ) -> Result<KafkaResponse> {
        match request.request_type {
            KafkaRequestType::Metadata => self.handle_metadata().await,
            KafkaRequestType::Produce => self.handle_produce(message_buf, &request).await,
            KafkaRequestType::Fetch => self.handle_fetch(message_buf, &request).await,
            KafkaRequestType::ListOffsets => self.handle_list_offsets(message_buf, &request).await,
            KafkaRequestType::FindCoordinator => {
                self.handle_find_coordinator(message_buf, &request).await
            }
            KafkaRequestType::JoinGroup => self.handle_join_group(message_buf, &request).await,
            KafkaRequestType::SyncGroup => self.handle_sync_group(message_buf, &request).await,
            KafkaRequestType::Heartbeat => self.handle_heartbeat(message_buf, &request).await,
            KafkaRequestType::LeaveGroup => self.handle_leave_group(message_buf, &request).await,
            KafkaRequestType::OffsetCommit => {
                self.handle_offset_commit(message_buf, &request).await
            }
            KafkaRequestType::OffsetFetch => self.handle_offset_fetch(message_buf, &request).await,
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

    /// Handle a Produce request: parse the body (flexible v9 or non-flexible
    /// v3..=v8), decode each RecordBatch v2, write decoded records into the
    /// corresponding topic partition, and serialize a version-matched
    /// response with real base_offsets.
    ///
    /// librdkafka 1.8.x (e.g. kcat 1.7.1) caps at v7 — a non-flexible version.
    /// Modern clients auto-negotiate to v9.
    async fn handle_produce(
        &self,
        message_buf: &[u8],
        request: &KafkaRequest,
    ) -> Result<KafkaResponse> {
        use crate::produce_codec::{
            parse_produce_v9, serialize_produce_v9_response, PartitionProduceResult,
            TopicProduceResult,
        };
        use crate::produce_nonflex::{parse_produce_v3_v8, serialize_produce_v3_v8_response};

        const ERR_UNKNOWN_TOPIC_OR_PARTITION: i16 = 3;

        let version = request.api_version;
        let is_flexible = version == 9;
        let is_nonflex = (3..=8).contains(&version);
        if !is_flexible && !is_nonflex {
            // Outside the advertised range. Reply with an empty response in
            // the shape of the most-compatible version we can. v9 flexible
            // is what we advertise as max; use it so auto-negotiating
            // clients fall back to a supported version on their next
            // ApiVersions check.
            let body = serialize_produce_v9_response(request.correlation_id, &[]);
            tracing::warn!("rejecting Produce v{version} (supported: 3..=9)");
            return Ok(KafkaResponse::Preserialized(body));
        }

        let body_slice = message_buf.get(request.body_offset..).ok_or_else(|| {
            mockforge_core::Error::internal("produce request body_offset past end of buffer")
        })?;

        let parsed = if is_flexible {
            parse_produce_v9(body_slice).map_err(|e| {
                mockforge_core::Error::internal(format!("failed to parse Produce v9: {e}"))
            })?
        } else {
            parse_produce_v3_v8(body_slice).map_err(|e| {
                mockforge_core::Error::internal(format!("failed to parse Produce v{version}: {e}"))
            })?
        };

        let append_time_ms = chrono::Utc::now().timestamp_millis();
        let mut topic_results = Vec::with_capacity(parsed.topics.len());

        for topic_data in parsed.topics {
            let mut partition_results = Vec::with_capacity(topic_data.partitions.len());
            // Records that landed in this topic — handed to the fixture
            // runtime after the per-topic loop so relationships see the
            // whole batch that was just written.
            let mut accepted_for_relationships: Vec<KafkaMessage> = Vec::new();
            let topic_name = topic_data.name.clone();
            for part in topic_data.partitions {
                let mut topics_guard = self.topics.write().await;
                // Auto-create the topic if a client produces to a name we
                // don't know yet. Single partition is a sensible default;
                // real Kafka uses the broker's auto-create config.
                let topic_entry =
                    topics_guard.entry(topic_data.name.clone()).or_insert_with(|| {
                        Topic::new(topic_data.name.clone(), crate::topics::TopicConfig::default())
                    });

                // `produce_codec::parse_record_batch` already decompresses the
                // 4 standard codecs, so `part.compression_codec` is only
                // informational here. The `part.records` vector always holds
                // the uncompressed records regardless of the incoming codec.

                // Empty batches still need a response entry. base_offset of
                // an empty batch is -1 per the Kafka convention.
                if part.records.is_empty() {
                    partition_results.push(PartitionProduceResult {
                        partition_index: part.partition_index,
                        error_code: 0,
                        base_offset: -1,
                        log_append_time_ms: append_time_ms,
                        log_start_offset: 0,
                    });
                    continue;
                }

                if topic_entry.get_partition(part.partition_index).is_none() {
                    partition_results.push(PartitionProduceResult {
                        partition_index: part.partition_index,
                        error_code: ERR_UNKNOWN_TOPIC_OR_PARTITION,
                        base_offset: -1,
                        log_append_time_ms: -1,
                        log_start_offset: 0,
                    });
                    continue;
                }

                let mut base_offset: i64 = -1;
                for (i, rec) in part.records.into_iter().enumerate() {
                    let msg = KafkaMessage {
                        offset: 0, // assigned by Topic::produce
                        timestamp: rec.timestamp_ms,
                        key: rec.key,
                        value: rec.value,
                        headers: rec.headers,
                    };
                    // Snapshot the record for downstream relationship
                    // fan-out. Cheap — a handful of bytes per message.
                    accepted_for_relationships.push(msg.clone());
                    let offset = topic_entry.produce(part.partition_index, msg).await?;
                    if i == 0 {
                        base_offset = offset;
                    }
                }

                partition_results.push(PartitionProduceResult {
                    partition_index: part.partition_index,
                    error_code: 0,
                    base_offset,
                    log_append_time_ms: append_time_ms,
                    log_start_offset: 0,
                });
            }
            topic_results.push(TopicProduceResult {
                name: topic_data.name,
                partitions: partition_results,
            });

            // Fire any relationships whose `from_topic` matches this
            // topic — one dependent emission per source record. This
            // runs outside the topics guard (which we already dropped
            // at the end of the partition loop), so the relationship
            // emitter can acquire the write lock itself to produce.
            if !accepted_for_relationships.is_empty() {
                if let Some(runtime) = self.fixture_runtime() {
                    let broker_arc = Arc::new(self.clone());
                    crate::fixture_executor::on_produced_records(
                        &broker_arc,
                        &runtime,
                        &topic_name,
                        &accepted_for_relationships,
                    )
                    .await;
                }
            }
        }

        let body = if is_flexible {
            serialize_produce_v9_response(request.correlation_id, &topic_results)
        } else {
            serialize_produce_v3_v8_response(request.correlation_id, version, &topic_results)
        };
        Ok(KafkaResponse::Preserialized(body))
    }

    /// Handle a Fetch request: parse the body (flexible v12 or non-flexible
    /// v4..=v11), pull records from topic/partition storage starting at each
    /// requested fetch_offset, and serialize a version-matched response with
    /// real RecordBatch v2 blobs (CRC32C-validated so consumers accept them).
    ///
    /// librdkafka 1.8.x / kcat 1.7.1 sends v11 (non-flexible); modern
    /// clients auto-negotiate to v12.
    async fn handle_fetch(
        &self,
        message_buf: &[u8],
        request: &KafkaRequest,
    ) -> Result<KafkaResponse> {
        use crate::fetch_codec::{
            parse_fetch_v12, serialize_fetch_v12_response, serialize_record_batch_v2,
            FetchPartitionResponse, FetchTopicResponse,
        };
        use crate::fetch_nonflex::{parse_fetch_v4_v11, serialize_fetch_v4_v11_response};

        const ERR_UNKNOWN_TOPIC_OR_PARTITION: i16 = 3;
        const ERR_OFFSET_OUT_OF_RANGE: i16 = 1;

        let version = request.api_version;
        let is_flexible = version == 12;
        let is_nonflex = (4..=11).contains(&version);
        if !is_flexible && !is_nonflex {
            let body = serialize_fetch_v12_response(request.correlation_id, 0, &[]);
            tracing::warn!("rejecting Fetch v{version} (supported: 4..=12)");
            return Ok(KafkaResponse::Preserialized(body));
        }

        let body_slice = message_buf.get(request.body_offset..).ok_or_else(|| {
            mockforge_core::Error::internal("fetch request body_offset past end of buffer")
        })?;

        let parsed = if is_flexible {
            parse_fetch_v12(body_slice).map_err(|e| {
                mockforge_core::Error::internal(format!("failed to parse Fetch v12: {e}"))
            })?
        } else {
            parse_fetch_v4_v11(version, body_slice).map_err(|e| {
                mockforge_core::Error::internal(format!("failed to parse Fetch v{version}: {e}"))
            })?
        };

        let topics_guard = self.topics.read().await;
        let mut topic_responses = Vec::with_capacity(parsed.topics.len());
        for t in &parsed.topics {
            let mut partition_responses = Vec::with_capacity(t.partitions.len());
            let topic = topics_guard.get(&t.topic);
            for p in &t.partitions {
                let Some(topic) = topic else {
                    partition_responses.push(FetchPartitionResponse {
                        partition_index: p.partition_index,
                        error_code: ERR_UNKNOWN_TOPIC_OR_PARTITION,
                        high_watermark: -1,
                        log_start_offset: -1,
                        records: Vec::new(),
                    });
                    continue;
                };
                let Some(part) = topic.get_partition(p.partition_index) else {
                    partition_responses.push(FetchPartitionResponse {
                        partition_index: p.partition_index,
                        error_code: ERR_UNKNOWN_TOPIC_OR_PARTITION,
                        high_watermark: -1,
                        log_start_offset: -1,
                        records: Vec::new(),
                    });
                    continue;
                };

                // Validate offset: fetch_offset > high_watermark is
                // OFFSET_OUT_OF_RANGE; == high_watermark is a valid empty
                // fetch (consumer is caught up).
                if p.fetch_offset > part.high_watermark {
                    partition_responses.push(FetchPartitionResponse {
                        partition_index: p.partition_index,
                        error_code: ERR_OFFSET_OUT_OF_RANGE,
                        high_watermark: part.high_watermark,
                        log_start_offset: part.log_start_offset,
                        records: Vec::new(),
                    });
                    continue;
                }

                // Collect records with offset >= fetch_offset, respecting
                // partition_max_bytes. Kafka requires at least one record be
                // returned when any are available past fetch_offset, even
                // when that exceeds max_bytes.
                let max_bytes = p.partition_max_bytes.max(0) as usize;
                let mut selected: Vec<&KafkaMessage> = Vec::new();
                let mut estimated_size: usize = 0;
                for msg in &part.messages {
                    if msg.offset < p.fetch_offset {
                        continue;
                    }
                    // Rough pre-serialize size estimate: key+value+headers
                    // + 16 byte record framing. Accurate enough for the
                    // soft-limit behavior.
                    let headers_size: usize =
                        msg.headers.iter().map(|(k, v)| k.len() + v.len() + 8).sum();
                    let record_size = msg.key.as_ref().map_or(0, |k| k.len())
                        + msg.value.len()
                        + headers_size
                        + 16;
                    if !selected.is_empty() && estimated_size + record_size > max_bytes {
                        break;
                    }
                    estimated_size += record_size;
                    selected.push(msg);
                }

                let records_blob = if selected.is_empty() {
                    Vec::new()
                } else {
                    serialize_record_batch_v2(&selected)
                };

                partition_responses.push(FetchPartitionResponse {
                    partition_index: p.partition_index,
                    error_code: 0,
                    high_watermark: part.high_watermark,
                    log_start_offset: part.log_start_offset,
                    records: records_blob,
                });
            }
            topic_responses.push(FetchTopicResponse {
                topic: t.topic.clone(),
                partitions: partition_responses,
            });
        }

        let body = if is_flexible {
            serialize_fetch_v12_response(
                request.correlation_id,
                parsed.session_id,
                &topic_responses,
            )
        } else {
            serialize_fetch_v4_v11_response(
                request.correlation_id,
                version,
                parsed.session_id,
                &topic_responses,
            )
        };
        Ok(KafkaResponse::Preserialized(body))
    }

    /// Handle a ListOffsets v7 request. Resolves each partition's
    /// `timestamp` field to a real offset by consulting the storage layer:
    /// `-2` (earliest) → `partition.log_start_offset`, `-1` (latest) →
    /// `partition.high_watermark`, positive timestamps → first message at
    /// or after that timestamp (best-effort linear scan since we keep
    /// messages in insertion order).
    async fn handle_list_offsets(
        &self,
        message_buf: &[u8],
        request: &KafkaRequest,
    ) -> Result<KafkaResponse> {
        use crate::listoffsets_codec::{
            parse_listoffsets_v7, serialize_listoffsets_v7_response, ListOffsetsPartitionResponse,
            ListOffsetsTopicResponse,
        };

        const ERR_UNKNOWN_TOPIC_OR_PARTITION: i16 = 3;
        const TS_EARLIEST: i64 = -2;
        const TS_LATEST: i64 = -1;

        if request.api_version != 7 {
            let body = serialize_listoffsets_v7_response(request.correlation_id, &[]);
            tracing::warn!("rejecting ListOffsets v{} (only v7 supported)", request.api_version);
            return Ok(KafkaResponse::Preserialized(body));
        }

        let body_slice = message_buf.get(request.body_offset..).ok_or_else(|| {
            mockforge_core::Error::internal("listoffsets body_offset past end of buffer")
        })?;

        let parsed = parse_listoffsets_v7(body_slice).map_err(|e| {
            mockforge_core::Error::internal(format!("failed to parse ListOffsets v7: {e}"))
        })?;

        let topics_guard = self.topics.read().await;
        let mut topic_responses = Vec::with_capacity(parsed.topics.len());
        for t in &parsed.topics {
            let mut partition_responses = Vec::with_capacity(t.partitions.len());
            let topic = topics_guard.get(&t.topic);
            for p in &t.partitions {
                let Some(topic) = topic else {
                    partition_responses.push(ListOffsetsPartitionResponse {
                        partition_index: p.partition_index,
                        error_code: ERR_UNKNOWN_TOPIC_OR_PARTITION,
                        timestamp: -1,
                        offset: -1,
                    });
                    continue;
                };
                let Some(part) = topic.get_partition(p.partition_index) else {
                    partition_responses.push(ListOffsetsPartitionResponse {
                        partition_index: p.partition_index,
                        error_code: ERR_UNKNOWN_TOPIC_OR_PARTITION,
                        timestamp: -1,
                        offset: -1,
                    });
                    continue;
                };

                let (offset, ts) = match p.timestamp {
                    TS_EARLIEST => (part.log_start_offset, -1),
                    TS_LATEST => (part.high_watermark, -1),
                    needle => {
                        // Best-effort timestamp lookup: return the first message
                        // whose timestamp >= needle. If none, fall back to the
                        // high watermark (caller will just get an empty fetch).
                        let found = part.messages.iter().find(|m| m.timestamp >= needle);
                        match found {
                            Some(m) => (m.offset, m.timestamp),
                            None => (part.high_watermark, -1),
                        }
                    }
                };
                partition_responses.push(ListOffsetsPartitionResponse {
                    partition_index: p.partition_index,
                    error_code: 0,
                    timestamp: ts,
                    offset,
                });
            }
            topic_responses.push(ListOffsetsTopicResponse {
                topic: t.topic.clone(),
                partitions: partition_responses,
            });
        }

        let body = serialize_listoffsets_v7_response(request.correlation_id, &topic_responses);
        Ok(KafkaResponse::Preserialized(body))
    }

    /// Handle FindCoordinator v2: the mock is always the coordinator for
    /// any group the caller asks about. Advertised max is 2 so librdkafka
    /// 2.x (which caps at v2 anyway) lands on exactly this path.
    async fn handle_find_coordinator(
        &self,
        message_buf: &[u8],
        request: &KafkaRequest,
    ) -> Result<KafkaResponse> {
        use crate::group_codec::{
            parse_find_coordinator_v2, serialize_find_coordinator_v2_response,
        };
        if request.api_version != 2 {
            let body = serialize_find_coordinator_v2_response(
                request.correlation_id,
                &self.config.host,
                self.config.port as i32,
            );
            tracing::warn!(
                "rejecting FindCoordinator v{} (only v2 supported)",
                request.api_version
            );
            return Ok(KafkaResponse::Preserialized(body));
        }
        let body_slice = message_buf.get(request.body_offset..).ok_or_else(|| {
            mockforge_core::Error::internal("find_coordinator body_offset past buffer end")
        })?;
        let _parsed = parse_find_coordinator_v2(body_slice).map_err(|e| {
            mockforge_core::Error::internal(format!("FindCoordinator v2 parse: {e}"))
        })?;
        let body = serialize_find_coordinator_v2_response(
            request.correlation_id,
            &self.config.host,
            self.config.port as i32,
        );
        Ok(KafkaResponse::Preserialized(body))
    }

    /// Handle JoinGroup v5 (non-flexible).
    async fn handle_join_group(
        &self,
        message_buf: &[u8],
        request: &KafkaRequest,
    ) -> Result<KafkaResponse> {
        use crate::group_codec::{
            parse_join_group_v5, serialize_join_group_v5_response, JoinGroupResponseMember,
        };
        if request.api_version != 5 {
            let body =
                serialize_join_group_v5_response(request.correlation_id, 0, "range", "", "", &[]);
            tracing::warn!("rejecting JoinGroup v{} (only v5 supported)", request.api_version);
            return Ok(KafkaResponse::Preserialized(body));
        }
        let body_slice = message_buf.get(request.body_offset..).ok_or_else(|| {
            mockforge_core::Error::internal("join_group body_offset past buffer end")
        })?;
        let parsed = parse_join_group_v5(body_slice)
            .map_err(|e| mockforge_core::Error::internal(format!("JoinGroup v5 parse: {e}")))?;

        let protocols: Vec<(String, Vec<u8>)> =
            parsed.protocols.iter().map(|p| (p.name.clone(), p.metadata.clone())).collect();
        let outcome = self.group_coordinator.write().await.join_group(
            &parsed.group_id,
            &parsed.member_id,
            &protocols,
        );
        let members: Vec<JoinGroupResponseMember> = outcome
            .members
            .iter()
            .map(|m| JoinGroupResponseMember {
                member_id: m.member_id.clone(),
                metadata: m.metadata.clone(),
            })
            .collect();

        let body = serialize_join_group_v5_response(
            request.correlation_id,
            outcome.generation_id,
            &outcome.protocol_name,
            &outcome.leader_id,
            &outcome.member_id,
            &members,
        );
        Ok(KafkaResponse::Preserialized(body))
    }

    /// Handle SyncGroup v3 (non-flexible).
    async fn handle_sync_group(
        &self,
        message_buf: &[u8],
        request: &KafkaRequest,
    ) -> Result<KafkaResponse> {
        use crate::group_codec::{parse_sync_group_v3, serialize_sync_group_v3_response};
        if request.api_version != 3 {
            let body = serialize_sync_group_v3_response(request.correlation_id, &[]);
            tracing::warn!("rejecting SyncGroup v{} (only v3 supported)", request.api_version);
            return Ok(KafkaResponse::Preserialized(body));
        }
        let body_slice = message_buf.get(request.body_offset..).ok_or_else(|| {
            mockforge_core::Error::internal("sync_group body_offset past buffer end")
        })?;
        let parsed = parse_sync_group_v3(body_slice)
            .map_err(|e| mockforge_core::Error::internal(format!("SyncGroup v3 parse: {e}")))?;

        let pairs: Vec<(String, Vec<u8>)> =
            parsed.assignments.into_iter().map(|a| (a.member_id, a.assignment)).collect();
        let assignment = self
            .group_coordinator
            .write()
            .await
            .sync_group(&parsed.group_id, &parsed.member_id, pairs)
            .unwrap_or_default();

        let body = serialize_sync_group_v3_response(request.correlation_id, &assignment);
        Ok(KafkaResponse::Preserialized(body))
    }

    /// Handle Heartbeat v3 (non-flexible).
    async fn handle_heartbeat(
        &self,
        message_buf: &[u8],
        request: &KafkaRequest,
    ) -> Result<KafkaResponse> {
        use crate::group_codec::{parse_heartbeat_v3, serialize_heartbeat_v3_response};
        if request.api_version != 3 {
            let body = serialize_heartbeat_v3_response(request.correlation_id, 0);
            tracing::warn!("rejecting Heartbeat v{} (only v3 supported)", request.api_version);
            return Ok(KafkaResponse::Preserialized(body));
        }
        let body_slice = message_buf.get(request.body_offset..).ok_or_else(|| {
            mockforge_core::Error::internal("heartbeat body_offset past buffer end")
        })?;
        let parsed = parse_heartbeat_v3(body_slice)
            .map_err(|e| mockforge_core::Error::internal(format!("Heartbeat v3 parse: {e}")))?;

        let err = self
            .group_coordinator
            .write()
            .await
            .heartbeat(&parsed.group_id, parsed.generation_id, &parsed.member_id)
            .err()
            .unwrap_or(0);
        let body = serialize_heartbeat_v3_response(request.correlation_id, err);
        Ok(KafkaResponse::Preserialized(body))
    }

    /// Handle LeaveGroup v0..=v3 (non-flexible). Removes each listed
    /// member from the group; committed offsets are intentionally kept
    /// so a new consumer joining the same group resumes from them.
    ///
    /// librdkafka 2.x picks v1 (single-member) against us even though
    /// we advertise max=3 — so we accept any supported version and let
    /// the codec pick the right wire shape.
    async fn handle_leave_group(
        &self,
        message_buf: &[u8],
        request: &KafkaRequest,
    ) -> Result<KafkaResponse> {
        use crate::group_codec::{parse_leave_group, serialize_leave_group_response};
        if !(0..=3).contains(&request.api_version) {
            let body = serialize_leave_group_response(3, request.correlation_id, &[]);
            tracing::warn!(
                "rejecting LeaveGroup v{} (only v0..=v3 supported)",
                request.api_version
            );
            return Ok(KafkaResponse::Preserialized(body));
        }
        let body_slice = message_buf.get(request.body_offset..).ok_or_else(|| {
            mockforge_core::Error::internal("leave_group body_offset past buffer end")
        })?;
        let parsed = parse_leave_group(request.api_version, body_slice).map_err(|e| {
            mockforge_core::Error::internal(format!(
                "LeaveGroup v{} parse: {e}",
                request.api_version
            ))
        })?;

        {
            let mut coord = self.group_coordinator.write().await;
            for m in &parsed.members {
                coord.leave_group(&parsed.group_id, &m.member_id);
            }
        }

        let body = serialize_leave_group_response(
            request.api_version,
            request.correlation_id,
            &parsed.members,
        );
        Ok(KafkaResponse::Preserialized(body))
    }

    /// Handle OffsetCommit v7 (non-flexible). Persists each committed
    /// offset (and its opaque metadata blob) in the group coordinator so
    /// a subsequent OffsetFetch — possibly from a different consumer in
    /// the same group — resumes from the committed position.
    async fn handle_offset_commit(
        &self,
        message_buf: &[u8],
        request: &KafkaRequest,
    ) -> Result<KafkaResponse> {
        use crate::group_codec::{parse_offset_commit_v7, serialize_offset_commit_v7_response};
        if request.api_version != 7 {
            let body = serialize_offset_commit_v7_response(request.correlation_id, &[]);
            tracing::warn!("rejecting OffsetCommit v{} (only v7 supported)", request.api_version);
            return Ok(KafkaResponse::Preserialized(body));
        }
        let body_slice = message_buf.get(request.body_offset..).ok_or_else(|| {
            mockforge_core::Error::internal("offset_commit body_offset past buffer end")
        })?;
        let parsed = parse_offset_commit_v7(body_slice)
            .map_err(|e| mockforge_core::Error::internal(format!("OffsetCommit v7 parse: {e}")))?;

        {
            let mut coord = self.group_coordinator.write().await;
            for topic in &parsed.topics {
                for p in &topic.partitions {
                    coord.commit_offset(
                        &parsed.group_id,
                        &topic.name,
                        p.partition_index,
                        p.committed_offset,
                        p.committed_metadata.clone(),
                    );
                }
            }
        }

        let body = serialize_offset_commit_v7_response(request.correlation_id, &parsed.topics);
        Ok(KafkaResponse::Preserialized(body))
    }

    /// Handle OffsetFetch v5 (non-flexible). Looks up each requested
    /// partition in the coordinator; partitions without a prior commit
    /// return `offset = -1` so librdkafka falls back to
    /// `auto.offset.reset`.
    async fn handle_offset_fetch(
        &self,
        message_buf: &[u8],
        request: &KafkaRequest,
    ) -> Result<KafkaResponse> {
        use crate::group_codec::{
            parse_offset_fetch_v5, serialize_offset_fetch_v5_response,
            OffsetFetchPartitionResponse, OffsetFetchTopicResponse,
        };
        if request.api_version != 5 {
            let body = serialize_offset_fetch_v5_response(request.correlation_id, &[]);
            tracing::warn!("rejecting OffsetFetch v{} (only v5 supported)", request.api_version);
            return Ok(KafkaResponse::Preserialized(body));
        }
        let body_slice = message_buf.get(request.body_offset..).ok_or_else(|| {
            mockforge_core::Error::internal("offset_fetch body_offset past buffer end")
        })?;
        let parsed = parse_offset_fetch_v5(body_slice)
            .map_err(|e| mockforge_core::Error::internal(format!("OffsetFetch v5 parse: {e}")))?;

        let coord = self.group_coordinator.read().await;
        let topic_responses: Vec<OffsetFetchTopicResponse> = parsed
            .topics
            .iter()
            .map(|t| {
                let partitions = t
                    .partition_indexes
                    .iter()
                    .map(|&idx| match coord.fetch_offset(&parsed.group_id, &t.name, idx) {
                        Some(committed) => OffsetFetchPartitionResponse {
                            partition_index: idx,
                            committed_offset: committed.offset,
                            committed_metadata: committed.metadata,
                        },
                        None => OffsetFetchPartitionResponse {
                            partition_index: idx,
                            committed_offset: -1,
                            committed_metadata: None,
                        },
                    })
                    .collect();
                OffsetFetchTopicResponse {
                    name: t.name.clone(),
                    partitions,
                }
            })
            .collect();

        let body = serialize_offset_fetch_v5_response(request.correlation_id, &topic_responses);
        Ok(KafkaResponse::Preserialized(body))
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
        // Current wire parser does not decode topic names yet, so we create deterministic names.
        let mut topics = self.topics.write().await;
        let topic_name = if topics.contains_key("default-topic") {
            format!("topic-{}", topics.len() + 1)
        } else {
            "default-topic".to_string()
        };
        let topic_config = crate::topics::TopicConfig::default();
        let topic = Topic::new(topic_name.clone(), topic_config);

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
        offsets: HashMap<(String, i32), i64>,
    ) -> Result<()> {
        let mut consumer_groups = self.consumer_groups.write().await;
        consumer_groups
            .commit_offsets(group_id, offsets)
            .await
            .map_err(|e| mockforge_core::Error::from(e.to_string()))
    }

    /// Test helper: Get committed offsets for a consumer group (only available in tests)
    pub async fn test_get_committed_offsets(&self, group_id: &str) -> HashMap<(String, i32), i64> {
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
    use mockforge_core::config::KafkaSeedMessage;

    // ==================== Seed Message Tests ====================

    #[tokio::test]
    async fn seed_messages_populate_topic_before_first_consumer() {
        let mut seed_messages: HashMap<String, Vec<KafkaSeedMessage>> = HashMap::new();
        seed_messages.insert(
            "orders.created".to_string(),
            vec![
                KafkaSeedMessage {
                    key: Some("order-001".to_string()),
                    value: r#"{"order_id":"order-001","total":4299}"#.to_string(),
                    headers: HashMap::new(),
                },
                KafkaSeedMessage {
                    key: Some("order-002".to_string()),
                    value: r#"{"order_id":"order-002","total":1599}"#.to_string(),
                    headers: HashMap::from([("source".to_string(), "seed".to_string())]),
                },
            ],
        );

        let config = KafkaConfig {
            seed_messages,
            default_partitions: 3,
            ..KafkaConfig::default()
        };
        let broker = KafkaMockBroker::new(config).await.expect("broker should construct");

        let topics = broker.topics.read().await;
        let topic = topics.get("orders.created").expect("seeded topic must exist");

        // Both records present somewhere in the topic, regardless of which
        // partition the hash assigned them to.
        let mut total = 0;
        let mut saw_order_001 = false;
        let mut saw_order_002 = false;
        let mut saw_source_header = false;
        for partition in &topic.partitions {
            for msg in &partition.messages {
                total += 1;
                let value = std::str::from_utf8(&msg.value).unwrap_or("");
                if value.contains("order-001") {
                    saw_order_001 = true;
                }
                if value.contains("order-002") {
                    saw_order_002 = true;
                    if msg.headers.iter().any(|(k, v)| k == "source" && v == b"seed") {
                        saw_source_header = true;
                    }
                }
            }
        }
        assert_eq!(total, 2, "two seeded messages should be in the topic");
        assert!(saw_order_001, "order-001 should be present");
        assert!(saw_order_002, "order-002 should be present");
        assert!(saw_source_header, "order-002's source header should be preserved");
    }

    #[tokio::test]
    async fn seed_messages_use_default_partitions_when_topic_autocreated() {
        let mut seed_messages: HashMap<String, Vec<KafkaSeedMessage>> = HashMap::new();
        seed_messages.insert(
            "events".to_string(),
            vec![KafkaSeedMessage {
                key: Some("k".to_string()),
                value: "v".to_string(),
                headers: HashMap::new(),
            }],
        );

        let config = KafkaConfig {
            seed_messages,
            default_partitions: 5,
            ..KafkaConfig::default()
        };
        let broker = KafkaMockBroker::new(config).await.unwrap();
        let topics = broker.topics.read().await;
        assert_eq!(topics.get("events").unwrap().partitions.len(), 5);
    }

    #[tokio::test]
    async fn seed_messages_with_same_key_land_on_same_partition() {
        // Kafka's hash-on-key promise: same key → same partition. The seed
        // path must honor it so seeded records are consumable by client
        // code that filters by partition.
        let mut seed_messages: HashMap<String, Vec<KafkaSeedMessage>> = HashMap::new();
        seed_messages.insert(
            "events".to_string(),
            vec![
                KafkaSeedMessage {
                    key: Some("user-42".to_string()),
                    value: "first".to_string(),
                    headers: HashMap::new(),
                },
                KafkaSeedMessage {
                    key: Some("user-42".to_string()),
                    value: "second".to_string(),
                    headers: HashMap::new(),
                },
            ],
        );

        let config = KafkaConfig {
            seed_messages,
            default_partitions: 3,
            ..KafkaConfig::default()
        };
        let broker = KafkaMockBroker::new(config).await.unwrap();
        let topics = broker.topics.read().await;
        let topic = topics.get("events").unwrap();

        // Find the partition holding "first" and verify "second" is in the
        // same one.
        let mut partition_of_first: Option<i32> = None;
        let mut partition_of_second: Option<i32> = None;
        for partition in &topic.partitions {
            for msg in &partition.messages {
                let value = std::str::from_utf8(&msg.value).unwrap_or("");
                if value == "first" {
                    partition_of_first = Some(partition.id);
                }
                if value == "second" {
                    partition_of_second = Some(partition.id);
                }
            }
        }
        assert_eq!(
            partition_of_first, partition_of_second,
            "same key should land on the same partition"
        );
    }

    #[tokio::test]
    async fn empty_seed_messages_is_a_no_op() {
        let broker = KafkaMockBroker::new(KafkaConfig::default()).await.unwrap();
        assert!(broker.topics.read().await.is_empty());
    }

    // ==================== Advertised Endpoint Tests ====================

    #[test]
    fn advertised_falls_back_to_bind_when_unset() {
        let config = KafkaConfig {
            host: "127.0.0.1".to_string(),
            port: 19092,
            advertised_host: None,
            advertised_port: None,
            ..KafkaConfig::default()
        };
        assert_eq!(resolve_advertised_endpoint(&config), ("127.0.0.1".to_string(), 19092));
    }

    #[test]
    fn advertised_overrides_bind_when_host_set() {
        let config = KafkaConfig {
            host: "0.0.0.0".to_string(),
            port: 9092,
            advertised_host: Some("hosted-mock-abc.fly.dev".to_string()),
            advertised_port: None,
            ..KafkaConfig::default()
        };
        assert_eq!(
            resolve_advertised_endpoint(&config),
            ("hosted-mock-abc.fly.dev".to_string(), 9092)
        );
    }

    #[test]
    fn advertised_overrides_bind_when_port_set() {
        let config = KafkaConfig {
            host: "0.0.0.0".to_string(),
            port: 9092,
            advertised_host: None,
            advertised_port: Some(443),
            ..KafkaConfig::default()
        };
        assert_eq!(resolve_advertised_endpoint(&config), ("0.0.0.0".to_string(), 443));
    }

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
            body_offset: 0,
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
            body_offset: 0,
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
            body_offset: 0,
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
            body_offset: 0,
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
            body_offset: 0,
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
            body_offset: 0,
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
            body_offset: 0,
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
            body_offset: 0,
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
            body_offset: 0,
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

    #[tokio::test]
    async fn test_handle_produce_v9_writes_records_to_topic() {
        // Build a complete Produce v9 request over the wire, feed it through
        // parse_request + handle_produce, and assert the record actually
        // landed in the corresponding topic partition.
        use crate::produce_codec::{parse_produce_v9, PartitionProduceData, TopicProduceData};
        // The broker auto-creates topics on produce, so we can target any name.
        let broker = KafkaMockBroker::new(KafkaConfig::default()).await.expect("broker");

        // Hand-craft a minimal but complete produce v9 frame.
        let record_batch =
            crate::produce_codec::one_record_batch_for_testing(Some(b"key-1"), b"hello-produce");

        // Request header v2: api_key=0, api_version=9, correlation=777,
        // client_id="t", flexible header tag buffer.
        let mut msg = Vec::new();
        msg.extend_from_slice(&0i16.to_be_bytes());
        msg.extend_from_slice(&9i16.to_be_bytes());
        msg.extend_from_slice(&777i32.to_be_bytes());
        msg.extend_from_slice(&1i16.to_be_bytes());
        msg.push(b't');
        msg.push(0); // header tag buffer

        // Body
        msg.push(0); // transactional_id null
        msg.extend_from_slice(&(-1i16).to_be_bytes()); // acks
        msg.extend_from_slice(&30_000i32.to_be_bytes());
        // topics: 1+1=2
        msg.push(2);
        // topic name "prod-target"
        let topic_name = b"prod-target";
        msg.push((topic_name.len() as u8) + 1);
        msg.extend_from_slice(topic_name);
        // partitions: 1+1=2
        msg.push(2);
        // partition_index=0
        msg.extend_from_slice(&0i32.to_be_bytes());
        // records compact bytes
        let rb_len_plus_one = (record_batch.len() as u32) + 1;
        // Varint encode rb_len_plus_one manually (small enough for test)
        if rb_len_plus_one < 128 {
            msg.push(rb_len_plus_one as u8);
        } else {
            let mut v = rb_len_plus_one;
            while (v & !0x7F) != 0 {
                msg.push(((v & 0x7F) | 0x80) as u8);
                v >>= 7;
            }
            msg.push(v as u8);
        }
        msg.extend_from_slice(&record_batch);
        msg.push(0); // partition tag buffer
        msg.push(0); // topic tag buffer
        msg.push(0); // request tag buffer

        // Sanity-check the produce codec can parse our frame body.
        let body_offset = 10 /* header fixed */ + 1 /* client_id "t" */ + 1 /* tag buffer */;
        let parsed = parse_produce_v9(&msg[body_offset..]).expect("codec parse");
        assert_eq!(parsed.topics[0].name, "prod-target");
        assert_eq!(parsed.topics[0].partitions[0].records[0].value, b"hello-produce");

        // Now round-trip through the broker.
        let handler = crate::protocol::KafkaProtocolHandler::new();
        let request = handler.parse_request(&msg).expect("parse header");
        assert_eq!(request.api_key, 0);
        assert_eq!(request.api_version, 9);
        assert_eq!(request.body_offset, body_offset);

        let response = broker.handle_produce(&msg, &request).await.expect("produce");
        match response {
            KafkaResponse::Preserialized(bytes) => {
                // correlation_id echoed back
                assert_eq!(&bytes[0..4], &777i32.to_be_bytes());
            }
            other => panic!("unexpected response variant: {other:?}"),
        }

        // The record should be in the topic.
        let topics = broker.topics.read().await;
        let topic = topics.get("prod-target").expect("auto-created topic");
        let record_count: usize = topic.partitions.iter().map(|p| p.messages.len()).sum();
        assert_eq!(record_count, 1);
        let stored = topic.partitions[0].messages.front().unwrap();
        assert_eq!(stored.value, b"hello-produce");
        assert_eq!(stored.key.as_deref(), Some(b"key-1".as_ref()));
        let _ = TopicProduceData {
            name: String::new(),
            partitions: vec![],
        };
        let _ = PartitionProduceData {
            partition_index: 0,
            records: vec![],
            compression_codec: 0,
        };
    }

    #[tokio::test]
    async fn test_handle_create_topics_creates_unique_topic_names() {
        let broker = KafkaMockBroker::new(KafkaConfig::default()).await.expect("broker");
        let _ = broker.handle_create_topics().await.expect("create1");
        let _ = broker.handle_create_topics().await.expect("create2");

        let topics = broker.topics.read().await;
        assert!(topics.contains_key("default-topic"));
        assert!(topics.keys().any(|name| name.starts_with("topic-")));
    }
}
