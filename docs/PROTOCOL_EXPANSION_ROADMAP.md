# Protocol Expansion Roadmap

## Executive Summary

This document outlines the technical strategy for expanding MockForge beyond its current HTTP, gRPC, WebSocket, and GraphQL protocols to support additional communication protocols, particularly focusing on messaging systems (Kafka, RabbitMQ), IoT protocols (MQTT), and file transfer protocols (SMTP, FTP).

## Current Architecture Analysis

### Existing Protocol Support

MockForge currently supports four protocols through dedicated crates:

| Protocol | Crate | Status | Key Features |
|----------|-------|--------|--------------|
| HTTP/REST | `mockforge-http` | ✅ Complete | OpenAPI-driven, validation, templating |
| gRPC | `mockforge-grpc` | ✅ Complete | Proto-driven, HTTP bridge, reflection |
| WebSocket | `mockforge-ws` | ✅ Complete | Replay files, JSONPath matching |
| GraphQL | `mockforge-graphql` | ✅ Complete | Schema-driven, query validation |

### Protocol Abstraction Foundation

MockForge has a robust `protocol_abstraction` module (`crates/mockforge-core/src/protocol_abstraction/`) that provides:

```rust
pub enum Protocol {
    Http,
    GraphQL,
    Grpc,
    WebSocket,
    // Ready for expansion...
}

pub trait SpecRegistry: Send + Sync {
    fn protocol(&self) -> Protocol;
    fn operations(&self) -> Vec<SpecOperation>;
    fn validate_request(&self, request: &ProtocolRequest) -> Result<ValidationResult>;
    fn generate_mock_response(&self, request: &ProtocolRequest) -> Result<ProtocolResponse>;
}

pub trait ProtocolMiddleware: Send + Sync {
    async fn process_request(&self, request: &mut ProtocolRequest) -> Result<()>;
    async fn process_response(&self, request: &ProtocolRequest, response: &mut ProtocolResponse) -> Result<()>;
    fn supports_protocol(&self, protocol: Protocol) -> bool;
}
```

## Protocol Expansion Strategy

### Phase 1: Foundation Enhancement (1-2 months)

**Goal**: Strengthen the protocol abstraction layer to better support streaming and asynchronous messaging patterns.

#### 1.1 Protocol Abstraction Extensions

**File**: `crates/mockforge-core/src/protocol_abstraction/mod.rs`

Add new protocol types and message patterns:

```rust
pub enum Protocol {
    Http,
    GraphQL,
    Grpc,
    WebSocket,
    // New protocols
    Mqtt,
    Smtp,
    Ftp,
    Kafka,
    RabbitMq,
    Amqp,
}

/// Message pattern abstraction
pub enum MessagePattern {
    RequestResponse,  // HTTP, gRPC unary
    OneWay,          // Fire-and-forget (MQTT publish, email send)
    PubSub,          // Kafka, RabbitMQ, MQTT
    Streaming,       // gRPC streaming, WebSocket
}

/// Extended protocol request for async messaging
pub struct ProtocolRequest {
    pub protocol: Protocol,
    pub pattern: MessagePattern,
    pub operation: String,
    pub path: String,  // Or topic, queue, channel name
    pub metadata: HashMap<String, String>,
    pub body: Option<Vec<u8>>,
    pub client_ip: Option<String>,
    // New fields for messaging
    pub topic: Option<String>,        // For pub/sub
    pub routing_key: Option<String>,  // For AMQP
    pub partition: Option<i32>,       // For Kafka
    pub qos: Option<u8>,             // For MQTT
}
```

#### 1.2 Streaming Message Support

Create `crates/mockforge-core/src/protocol_abstraction/streaming.rs`:

```rust
/// Trait for streaming message protocols
#[async_trait::async_trait]
pub trait StreamingProtocol: Send + Sync {
    /// Start consuming messages from a topic/queue
    async fn subscribe(&self, topic: &str, consumer_id: &str) -> Result<MessageStream>;

    /// Publish a message
    async fn publish(&self, topic: &str, message: ProtocolMessage) -> Result<()>;

    /// Get protocol-specific metadata
    fn get_metadata(&self) -> StreamingMetadata;
}

pub struct ProtocolMessage {
    pub key: Option<Vec<u8>>,
    pub value: Vec<u8>,
    pub headers: HashMap<String, String>,
    pub timestamp: Option<i64>,
    pub partition: Option<i32>,
    pub offset: Option<i64>,
}

pub type MessageStream = Pin<Box<dyn Stream<Item = Result<ProtocolMessage>> + Send>>;
```

### Phase 2: SMTP/Email Mocking (2-3 weeks)

**Rationale**: SMTP is a smaller-scope protocol perfect for validating the extension architecture. Email testing is a common need in development.

#### 2.1 Create `mockforge-smtp` Crate

**Directory Structure**:
```
crates/mockforge-smtp/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── server.rs           # SMTP server implementation
│   ├── protocol.rs         # SMTP protocol handling
│   ├── mailbox.rs          # In-memory mailbox
│   ├── spec_registry.rs    # SpecRegistry implementation
│   └── fixtures.rs         # Email template fixtures
└── tests/
    └── integration.rs
```

**Dependencies** (`Cargo.toml`):
```toml
[dependencies]
mockforge-core = { path = "../mockforge-core" }
tokio = { workspace = true }
async-trait = { workspace = true }
serde = { workspace = true }
tracing = { workspace = true }

# SMTP-specific
mailin = "0.7"           # SMTP server library
mail-parser = "0.9"      # Email parsing
lettre = "0.11"          # Email building/sending (for client)
```

**Core Implementation**:

```rust
// src/lib.rs
use mockforge_core::protocol_abstration::{Protocol, ProtocolRequest, ProtocolResponse};

pub struct SmtpServer {
    config: SmtpConfig,
    mailbox: Arc<RwLock<InMemoryMailbox>>,
    spec_registry: Option<Arc<SmtpSpecRegistry>>,
}

impl SmtpServer {
    pub async fn start(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.config.bind_addr).await?;

        loop {
            let (stream, addr) = listener.accept().await?;
            let mailbox = self.mailbox.clone();

            tokio::spawn(async move {
                handle_smtp_session(stream, addr, mailbox).await
            });
        }
    }
}

// src/spec_registry.rs
pub struct SmtpSpecRegistry {
    fixtures: HashMap<String, EmailFixture>,
}

impl SpecRegistry for SmtpSpecRegistry {
    fn protocol(&self) -> Protocol {
        Protocol::Smtp
    }

    fn generate_mock_response(&self, request: &ProtocolRequest) -> Result<ProtocolResponse> {
        // Extract recipient from request
        let recipient = extract_recipient(&request)?;

        // Find matching fixture
        let fixture = self.fixtures.get(&recipient)
            .or_else(|| self.fixtures.get("default"))
            .ok_or_else(|| Error::NoFixtureFound)?;

        // Generate SMTP response (250 OK, etc.)
        Ok(ProtocolResponse {
            status: ResponseStatus::SmtpStatus(250),
            metadata: HashMap::new(),
            body: format!("250 Message accepted for {}", recipient).into_bytes(),
            content_type: "text/plain".to_string(),
        })
    }
}
```

**Configuration** (add to `crates/mockforge-core/src/config.rs`):

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    pub enabled: bool,
    pub port: u16,
    pub host: String,
    pub mailbox_storage: MailboxStorage,
    pub fixtures_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MailboxStorage {
    InMemory { max_messages: usize },
    File { path: PathBuf },
    Database { connection_string: String },
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 1025,  // Common dev SMTP port
            host: "0.0.0.0".to_string(),
            mailbox_storage: MailboxStorage::InMemory { max_messages: 1000 },
            fixtures_dir: None,
        }
    }
}

// Add to ServerConfig
pub struct ServerConfig {
    pub http: HttpConfig,
    pub websocket: WebSocketConfig,
    pub grpc: GrpcConfig,
    pub smtp: SmtpConfig,  // New
    // ... rest
}
```

**CLI Commands**:

```rust
// In mockforge-cli/src/commands/serve.rs
pub async fn start_smtp_server(config: SmtpConfig) -> Result<()> {
    let server = SmtpServer::new(config).await?;
    server.start().await
}

// New CLI command for mailbox inspection
mockforge mailbox list                    # List received emails
mockforge mailbox show <id>               # Show email details
mockforge mailbox clear                   # Clear mailbox
mockforge mailbox export --format mbox    # Export to mbox format
```

**Usage Example**:

```yaml
# config.yaml
smtp:
  enabled: true
  port: 1025
  host: "0.0.0.0"
  mailbox_storage:
    in_memory:
      max_messages: 1000
  fixtures_dir: "./fixtures/emails"

# fixtures/emails/welcome.yaml
trigger:
  recipient: "user@example.com"
response:
  accept: true
  delay_ms: 100
auto_reply:
  enabled: true
  from: "noreply@example.com"
  subject: "Welcome!"
  body: "Thank you for signing up."
```

### Phase 3: MQTT Protocol Support (3-4 weeks)

**Rationale**: MQTT is widely used in IoT and provides pub/sub patterns useful for testing event-driven systems.

#### 3.1 Create `mockforge-mqtt` Crate

**Dependencies**:
```toml
[dependencies]
mockforge-core = { path = "../mockforge-core" }
tokio = { workspace = true }
rumqttd = "0.18"         # MQTT broker
rumqttc = "0.24"         # MQTT client (for testing)
serde = { workspace = true }
```

**Key Features**:

1. **MQTT Broker Implementation**
   - QoS levels 0, 1, 2 support
   - Topic subscriptions and wildcards
   - Retained messages
   - Last Will and Testament (LWT)

2. **Topic-Based Fixtures**
   ```yaml
   # fixtures/mqtt/sensors.yaml
   topics:
     - pattern: "sensors/temperature/+"
       qos: 1
       retained: true
       response:
         payload:
           temperature: "{{faker.float 15.0 30.0}}"
           unit: "celsius"
           timestamp: "{{now}}"
       publish_interval_ms: 5000  # Publish every 5 seconds
   ```

3. **Integration with Protocol Abstraction**
   ```rust
   impl SpecRegistry for MqttSpecRegistry {
       fn protocol(&self) -> Protocol {
           Protocol::Mqtt
       }

       fn generate_mock_response(&self, request: &ProtocolRequest) -> Result<ProtocolResponse> {
           let topic = request.topic.as_ref().ok_or(Error::MissingTopic)?;
           let fixture = self.find_fixture_by_topic(topic)?;

           // Generate payload using template engine
           let payload = self.template_engine.render(&fixture.response.payload)?;

           Ok(ProtocolResponse {
               status: ResponseStatus::MqttStatus(true),
               metadata: HashMap::from([
                   ("topic".to_string(), topic.clone()),
                   ("qos".to_string(), request.qos.unwrap_or(0).to_string()),
               ]),
               body: payload.into_bytes(),
               content_type: "application/json".to_string(),
           })
       }
   }
   ```

**CLI Integration**:
```bash
# Start MQTT broker
mockforge serve --mqtt --mqtt-port 1883

# Publish test message
mockforge mqtt publish --topic "sensors/temp/room1" --payload '{"temp": 22.5}'

# Subscribe and monitor
mockforge mqtt subscribe --topic "sensors/#"

# Load topic fixtures
mockforge mqtt load-fixtures ./fixtures/mqtt/
```

### Phase 4: FTP Server Mocking (2-3 weeks)

**Rationale**: FTP testing is useful for file transfer integration testing and legacy system compatibility.

#### 4.1 Create `mockforge-ftp` Crate

**Dependencies**:
```toml
[dependencies]
mockforge-core = { path = "../mockforge-core" }
libunftp = "0.20"        # FTP server library
tokio = { workspace = true }
```

**Features**:

1. **Virtual File System**
   ```rust
   pub struct VirtualFileSystem {
       root: PathBuf,
       fixtures: HashMap<PathBuf, FileFixture>,
   }

   pub struct FileFixture {
       content: FileContent,
       metadata: FileMetadata,
   }

   pub enum FileContent {
       Static(Vec<u8>),
       Template(String),
       Generated { size: usize, pattern: GenerationPattern },
   }
   ```

2. **Fixture Configuration**
   ```yaml
   # fixtures/ftp/files.yaml
   virtual_files:
     - path: "/uploads/data.csv"
       content:
         template: |
           id,name,email
           {{#each (range 1 100)}}
           {{this}},{{faker.name}},{{faker.email}}
           {{/each}}
       permissions: "644"
       owner: "mockforge"

     - path: "/downloads/large-file.bin"
       content:
         generated:
           size: 104857600  # 100MB
           pattern: random
   ```

**CLI Commands**:
```bash
mockforge serve --ftp --ftp-port 2121
mockforge ftp ls /                      # List virtual files
mockforge ftp add-file /test.txt --content "Hello"
```

### Phase 5: Kafka Mock Server (4-6 weeks)

**Rationale**: Kafka is critical for event-driven architecture testing. Most complex but highest value.

#### 5.1 Create `mockforge-kafka` Crate

**Architecture Decision**:

Two implementation approaches:

**Option A: Embedded Kafka** (Using kafka-protocol-rs)
- Full Kafka protocol implementation
- More complex but complete control
- Better for advanced scenarios

**Option B: Kafka Test Container Wrapper** (Using testcontainers-rs)
- Faster to implement
- Real Kafka in Docker
- Simpler but requires Docker

**Recommended: Hybrid Approach**
- Start with simplified protocol implementation
- Use Apache Kafka's protocol libraries
- Implement most common operations (produce, consume, topics)

**Dependencies**:
```toml
[dependencies]
mockforge-core = { path = "../mockforge-core" }
rdkafka = "0.36"           # Kafka client
kafka-protocol = "0.12"    # Kafka protocol types
tokio = { workspace = true }
```

**Key Features**:

1. **Topic Management**
   ```rust
   pub struct KafkaMockBroker {
       topics: Arc<RwLock<HashMap<String, Topic>>>,
       config: KafkaConfig,
   }

   pub struct Topic {
       name: String,
       partitions: Vec<Partition>,
       config: TopicConfig,
   }

   pub struct Partition {
       id: i32,
       messages: VecDeque<KafkaMessage>,
       offset: i64,
   }
   ```

2. **Message Fixtures**
   ```yaml
   # fixtures/kafka/topics.yaml
   topics:
     - name: "orders.created"
       partitions: 3
       replication_factor: 1
       fixtures:
         - key_pattern: "order-{{uuid}}"
           value:
             order_id: "{{uuid}}"
             customer_id: "{{faker.uuid}}"
             total: "{{faker.float 10.0 1000.0}}"
             status: "pending"
           headers:
             event_type: "order.created"
             version: "1.0"

       # Auto-produce messages
       auto_produce:
         enabled: true
         rate_per_second: 10
         duration_seconds: 60
   ```

3. **Consumer Group Simulation**
   ```rust
   pub struct ConsumerGroupManager {
       groups: HashMap<String, ConsumerGroup>,
   }

   impl ConsumerGroupManager {
       pub async fn simulate_lag(&self, group: &str, topic: &str, lag_messages: i64) {
           // Simulate consumer lag for testing
       }

       pub async fn simulate_rebalance(&self, group: &str) {
           // Trigger consumer rebalance
       }
   }
   ```

**CLI Commands**:
```bash
# Start Kafka mock
mockforge serve --kafka --kafka-port 9092

# Topic management
mockforge kafka topic create orders --partitions 3
mockforge kafka topic list
mockforge kafka topic describe orders

# Producer/Consumer
mockforge kafka produce --topic orders --key "order1" --value '{"id": 1}'
mockforge kafka consume --topic orders --group test-group

# Fixtures
mockforge kafka load-fixtures ./fixtures/kafka/

# Testing features
mockforge kafka simulate-lag --group test-group --topic orders --lag 1000
mockforge kafka simulate-failure --broker 0 --duration 30s
```

### Phase 6: RabbitMQ/AMQP Support (4-6 weeks)

Similar to Kafka but with different messaging patterns (exchanges, queues, bindings).

**Create `mockforge-amqp` crate** using `lapin` library.

**Key Differences from Kafka**:
- Exchange types (direct, fanout, topic, headers)
- Queue bindings and routing
- Message acknowledgments
- Dead letter queues

## Implementation Priority

### Recommended Order

1. **SMTP** (Weeks 1-3)
   - Small scope, validates architecture
   - Immediate value for email testing
   - Low complexity

2. **MQTT** (Weeks 4-7)
   - Medium complexity
   - IoT use cases growing
   - Good pub/sub pattern validation

3. **FTP** (Weeks 8-10)
   - Moderate complexity
   - Completes "simpler protocols" phase
   - File transfer testing need

4. **Kafka** (Weeks 11-16)
   - High complexity, high value
   - Critical for microservices testing
   - Can leverage learnings from MQTT

5. **RabbitMQ/AMQP** (Weeks 17-22)
   - Similar to Kafka but different patterns
   - Completes messaging protocols suite

## Plugin-Based vs. Core Implementation

### Decision Matrix

| Protocol | Implementation | Rationale |
|----------|---------------|-----------|
| SMTP | **Core** | Small, commonly needed, validates architecture |
| MQTT | **Core** | IoT growing, pub/sub foundation for others |
| FTP | **Plugin** | Niche use case, self-contained |
| Kafka | **Core** | Critical for microservices, complex |
| RabbitMQ | **Core** or **Plugin** | Depends on adoption; could start as plugin |

### Plugin Implementation Template

For protocols implemented as plugins:

```rust
// Example: FTP as a plugin
pub struct FtpProtocolPlugin {
    server: Arc<FtpServer>,
    registry: Arc<FtpSpecRegistry>,
}

#[async_trait::async_trait]
impl ProtocolPlugin for FtpProtocolPlugin {
    fn protocol(&self) -> Protocol {
        Protocol::Ftp
    }

    async fn start(&self, config: PluginConfig) -> PluginResult<()> {
        self.server.start(config.into()).await
            .map_err(|e| PluginError::StartupFailed(e.to_string()))?;
        Ok(())
    }

    async fn handle_request(&self, request: ProtocolRequest) -> PluginResult<ProtocolResponse> {
        self.registry.generate_mock_response(&request)
            .map_err(|e| PluginError::RequestHandlingFailed(e.to_string()))
    }
}
```

## Architecture Enhancements Required

### 1. Protocol Registry

Create `crates/mockforge-core/src/protocol_registry.rs`:

```rust
pub struct ProtocolRegistry {
    protocols: HashMap<Protocol, Box<dyn ProtocolHandler>>,
}

#[async_trait::async_trait]
pub trait ProtocolHandler: Send + Sync {
    fn protocol(&self) -> Protocol;
    async fn start(&self, config: &ServerConfig) -> Result<()>;
    async fn shutdown(&self) -> Result<()>;
    fn spec_registry(&self) -> Arc<dyn SpecRegistry>;
}

impl ProtocolRegistry {
    pub fn register(&mut self, handler: Box<dyn ProtocolHandler>) {
        self.protocols.insert(handler.protocol(), handler);
    }

    pub async fn start_all(&self, config: &ServerConfig) -> Result<()> {
        for handler in self.protocols.values() {
            handler.start(config).await?;
        }
        Ok(())
    }
}
```

### 2. Unified Fixture Format

Extend fixture format to support all protocols:

```yaml
# config.yaml - Unified fixture format
fixtures:
  http:
    - path: "./fixtures/http/"
  smtp:
    - path: "./fixtures/smtp/"
  mqtt:
    - path: "./fixtures/mqtt/"
  kafka:
    - path: "./fixtures/kafka/"

# Individual fixture example
# fixtures/unified-example.yaml
protocol: mqtt
metadata:
  name: "Temperature Sensor Mock"
  description: "Simulates IoT temperature sensors"
  tags: ["iot", "sensors"]

topics:
  - pattern: "sensors/temperature/+"
    response:
      template: |
        {
          "sensor_id": "{{pathParam 1}}",
          "temperature": {{faker.float 15.0 35.0}},
          "unit": "celsius",
          "timestamp": "{{now}}"
        }
    behavior:
      publish_interval_ms: 5000
      qos: 1
      retained: true
```

### 3. Admin UI Enhancements

Add protocol-specific dashboards to Admin UI:

```javascript
// mockforge-ui additions
const protocolDashboards = {
  smtp: {
    component: SmtpMailboxDashboard,
    features: ['mailbox-viewer', 'email-search', 'export-mbox']
  },
  mqtt: {
    component: MqttTopicDashboard,
    features: ['topic-browser', 'message-stream', 'qos-stats']
  },
  kafka: {
    component: KafkaDashboard,
    features: ['topic-manager', 'consumer-groups', 'lag-monitor']
  }
};
```

## Testing Strategy

### 1. Protocol Conformance Tests

Each protocol implementation must pass conformance tests:

```rust
// crates/mockforge-{protocol}/tests/conformance.rs
#[tokio::test]
async fn test_smtp_rfc5321_compliance() {
    // Test SMTP RFC compliance
}

#[tokio::test]
async fn test_mqtt_v311_compliance() {
    // Test MQTT 3.1.1 spec compliance
}
```

### 2. Integration Tests

```rust
#[tokio::test]
async fn test_cross_protocol_middleware() {
    // Verify middleware works across all protocols
    let protocols = vec![
        Protocol::Http,
        Protocol::Smtp,
        Protocol::Mqtt,
        Protocol::Kafka,
    ];

    for protocol in protocols {
        let middleware = LoggingMiddleware::new(false);
        assert!(middleware.supports_protocol(protocol));
    }
}
```

### 3. Performance Benchmarks

Add benchmarks for each protocol:

```rust
// benches/protocols.rs
fn bench_kafka_throughput(c: &mut Criterion) {
    c.bench_function("kafka_produce_1k_msgs", |b| {
        b.iter(|| {
            // Benchmark Kafka message production
        });
    });
}
```

## Documentation Requirements

For each new protocol:

1. **User Guide**
   - `book/src/protocols/{protocol}/getting-started.md`
   - `book/src/protocols/{protocol}/configuration.md`
   - `book/src/protocols/{protocol}/fixtures.md`
   - `book/src/protocols/{protocol}/examples.md`

2. **API Documentation**
   - Rust API docs in code
   - CLI command reference

3. **Examples**
   - `examples/{protocol}/basic.yaml`
   - `examples/{protocol}/advanced.yaml`
   - Integration examples

## Migration Path for Users

### Opt-in Protocol Enablement

```yaml
# config.yaml
protocols:
  http:
    enabled: true
  grpc:
    enabled: true
  websocket:
    enabled: false  # Can disable unused protocols
  smtp:
    enabled: true   # Opt-in to new protocols
  mqtt:
    enabled: false
  kafka:
    enabled: false
```

### Feature Flags

```toml
# Cargo.toml
[features]
default = ["http", "grpc", "websocket", "graphql"]
smtp = ["dep:mailin", "dep:mail-parser"]
mqtt = ["dep:rumqttd"]
ftp = ["dep:libunftp"]
kafka = ["dep:rdkafka", "dep:kafka-protocol"]
amqp = ["dep:lapin"]
all-protocols = ["smtp", "mqtt", "ftp", "kafka", "amqp"]
```

## Success Metrics

### Technical Metrics
- Protocol conformance test pass rate: 100%
- Performance overhead vs native: < 5%
- Memory usage per protocol: < 50MB baseline
- Code coverage: > 80% per protocol crate

### Adoption Metrics
- Protocol usage in community
- GitHub stars/downloads
- Issue/PR activity for each protocol
- Documentation page views

## Risk Assessment

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| Scope creep | High | High | Strict phase boundaries, MVP approach |
| Protocol complexity | Medium | High | Start with simpler protocols (SMTP, FTP) |
| Maintenance burden | Medium | Medium | Plugin architecture for niche protocols |
| Breaking changes | Low | High | Careful API design, semantic versioning |
| Performance issues | Medium | High | Benchmark-driven development |

## Timeline Summary

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| Foundation | 1-2 months | Enhanced protocol abstraction |
| SMTP | 2-3 weeks | mockforge-smtp crate |
| MQTT | 3-4 weeks | mockforge-mqtt crate |
| FTP | 2-3 weeks | mockforge-ftp crate or plugin |
| Kafka | 4-6 weeks | mockforge-kafka crate |
| RabbitMQ | 4-6 weeks | mockforge-amqp crate |
| **Total** | **5-7 months** | Multi-protocol MockForge |

## Conclusion

Protocol expansion should follow a **measured, incremental approach**:

1. **Start with SMTP** to validate the architecture with a simpler protocol
2. **Progress to MQTT** for pub/sub pattern validation
3. **Add Kafka** as the flagship messaging protocol
4. **Consider plugin-based approach** for niche protocols (FTP, specialized messaging)

This strategy balances:
- ✅ **User value**: Address common testing needs (email, IoT, messaging)
- ✅ **Technical risk**: Validate architecture with simpler protocols first
- ✅ **Maintainability**: Plugin system for specialized needs
- ✅ **Performance**: Rust native implementations for core protocols
- ✅ **Ecosystem fit**: Leverage existing MockForge strengths (AI, templating, fixtures)

The protocol abstraction layer is already well-designed and can accommodate these extensions with minimal breaking changes. The key is to maintain consistency across protocols while respecting their unique characteristics.
