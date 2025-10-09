# Protocol Expansion Summary

## Overview

This document summarizes the comprehensive protocol expansion strategy created for MockForge, addressing the request to add new protocol simulators beyond the current HTTP, gRPC, WebSocket, and GraphQL support.

## What Has Been Created

### 1. Strategic Roadmap
**File:** [`PROTOCOL_EXPANSION_ROADMAP.md`](./PROTOCOL_EXPANSION_ROADMAP.md)

A detailed 5-7 month roadmap for adding new protocols:

**Priority Order:**
1. **SMTP** (2-3 weeks) - Email mocking for testing notifications
2. **MQTT** (3-4 weeks) - IoT protocol for sensor simulation
3. **FTP** (2-3 weeks) - File transfer testing
4. **Kafka** (4-6 weeks) - Event streaming for microservices
5. **RabbitMQ/AMQP** (4-6 weeks) - Advanced messaging patterns

**Key Insights:**
- Start with simpler protocols (SMTP, FTP) to validate architecture
- MQTT validates pub/sub patterns before tackling Kafka
- Kafka is the flagship messaging protocol (highest complexity, highest value)
- Plugin system can handle niche protocols

### 2. Implementation Guide
**File:** [`PROTOCOL_IMPLEMENTATION_GUIDE.md`](./PROTOCOL_IMPLEMENTATION_GUIDE.md)

A practical developer guide with:
- Step-by-step crate creation instructions
- Complete code templates for new protocols
- Trait implementation patterns
- Configuration integration examples
- CLI command integration
- Testing strategies
- A complete SMTP example implementation

**Highlights:**
- Leverages existing `protocol_abstraction` layer
- Follows established patterns from HTTP/gRPC
- Includes benchmarking and testing templates
- Shows how to integrate with MockForge's template engine

### 3. Example Fixtures
**Location:** `examples/protocols/`

Ready-to-use example fixtures for each protocol:

#### SMTP - Email Server
**File:** [`examples/protocols/smtp/welcome-email.yaml`](../examples/protocols/smtp/welcome-email.yaml)

Features:
- Auto-reply configuration
- Template-based email generation
- Mailbox storage options
- Email validation rules
- Simulated delivery delays

#### MQTT - IoT Sensors
**File:** [`examples/protocols/mqtt/iot-sensors.yaml`](../examples/protocols/mqtt/iot-sensors.yaml)

Features:
- Multi-topic sensor simulation (temperature, humidity)
- QoS levels and retained messages
- Auto-publish with configurable intervals
- Data drift simulation (realistic sensor behavior)
- Last Will and Testament (LWT)
- MQTT 5.0 support
- Daily patterns (temperature cycles)

#### Kafka - Event Streaming
**File:** [`examples/protocols/kafka/order-events.yaml`](../examples/protocols/kafka/order-events.yaml)

Features:
- Multi-topic event flows (orders, payments, inventory)
- State machine-based event progression
- Consumer group simulation with lag
- Event relationships and chaining
- Scenario-based testing (successful orders, failed payments, fraud checks)
- Broker failure simulation
- Metrics and monitoring

#### FTP - File Server
**File:** [`examples/protocols/ftp/file-server.yaml`](../examples/protocols/ftp/file-server.yaml)

Features:
- Virtual file system
- Template-based file generation (CSV, JSON, XML)
- Dynamic file creation on-demand
- Upload validation and processing
- Bandwidth throttling
- User authentication and permissions
- Quota management

### 4. Protocol Examples Documentation
**File:** [`examples/protocols/README.md`](../examples/protocols/README.md)

Comprehensive guide covering:
- Quick start for each protocol
- Use case examples
- Fixture format overview
- Protocol comparison table
- Testing workflows
- Development status and roadmap

## Architecture Highlights

### Existing Foundation ✅

MockForge already has a robust foundation:

```rust
// From crates/mockforge-core/src/protocol_abstraction/mod.rs
pub enum Protocol {
    Http, GraphQL, Grpc, WebSocket,
    // Ready to add: Mqtt, Smtp, Ftp, Kafka, Amqp
}

pub trait SpecRegistry: Send + Sync {
    fn protocol(&self) -> Protocol;
    fn validate_request(&self, request: &ProtocolRequest) -> Result<ValidationResult>;
    fn generate_mock_response(&self, request: &ProtocolRequest) -> Result<ProtocolResponse>;
}

pub trait ProtocolMiddleware: Send + Sync {
    async fn process_request(&self, request: &mut ProtocolRequest) -> Result<()>;
    async fn process_response(&self, request: &ProtocolRequest, response: &mut ProtocolResponse) -> Result<()>;
}
```

### Recommended Enhancements

#### 1. Extend Protocol Enum
```rust
pub enum Protocol {
    Http, GraphQL, Grpc, WebSocket,
    Mqtt, Smtp, Ftp, Kafka, Amqp,  // Add these
}

pub enum MessagePattern {
    RequestResponse,  // HTTP, gRPC, FTP
    OneWay,          // SMTP
    PubSub,          // MQTT, Kafka, RabbitMQ
    Streaming,       // gRPC streaming, WebSocket
}
```

#### 2. Streaming Support
```rust
#[async_trait::async_trait]
pub trait StreamingProtocol: Send + Sync {
    async fn subscribe(&self, topic: &str, consumer_id: &str) -> Result<MessageStream>;
    async fn publish(&self, topic: &str, message: ProtocolMessage) -> Result<()>;
}
```

#### 3. Configuration Extensions
```rust
// In crates/mockforge-core/src/config.rs
pub struct ServerConfig {
    pub http: HttpConfig,
    pub grpc: GrpcConfig,
    pub websocket: WebSocketConfig,
    pub smtp: SmtpConfig,     // Add
    pub mqtt: MqttConfig,     // Add
    pub ftp: FtpConfig,       // Add
    pub kafka: KafkaConfig,   // Add
}
```

## Implementation Approach

### Phase-Based Strategy

**Phase 1: Foundation (1-2 months)**
- Extend protocol abstraction for streaming/messaging
- Add streaming protocol trait
- Create protocol registry system

**Phase 2: SMTP (2-3 weeks)**
- ✅ Validates architecture with simpler protocol
- Small codebase, clear spec (RFC 5321)
- Immediate value for email testing

**Phase 3: MQTT (3-4 weeks)**
- Validates pub/sub pattern
- IoT use cases
- Prepares for Kafka

**Phase 4: FTP (2-3 weeks)**
- Can be plugin-based
- File transfer testing
- Good for integration tests

**Phase 5: Kafka (4-6 weeks)**
- Most complex, highest value
- Critical for microservices testing
- Leverage learnings from MQTT

**Phase 6: RabbitMQ (4-6 weeks)**
- Alternative messaging patterns
- Completes messaging suite
- Can leverage Kafka work

### Core vs. Plugin Decision

| Protocol | Recommendation | Rationale |
|----------|---------------|-----------|
| SMTP | **Core** | Common need, validates architecture |
| MQTT | **Core** | IoT growing, foundational pub/sub |
| FTP | **Plugin** | Niche, self-contained |
| Kafka | **Core** | Critical for microservices |
| RabbitMQ | **Core or Plugin** | Depends on adoption |

## Key Benefits

### 1. Leverages Existing Strengths
- ✅ Protocol abstraction already designed for multi-protocol
- ✅ Template engine works across all protocols
- ✅ Middleware chain supports any protocol
- ✅ AI-powered data generation applies universally
- ✅ Admin UI can add protocol-specific dashboards

### 2. Consistent Developer Experience
```yaml
# Same fixture format across protocols
fixture:
  name: "My Fixture"
  protocol: mqtt  # or smtp, kafka, etc.

response:
  template: "{{faker.data}}"  # Same templating

behavior:
  delay_ms: 100              # Same simulation features
  failure_rate: 0.01
```

### 3. Unified Testing Workflow
```bash
# Start multiple protocols
mockforge serve --http --mqtt --kafka --config config.yaml

# Load fixtures for any protocol
mockforge {protocol} load-fixtures ./fixtures/

# Monitor all protocols
mockforge logs --all-protocols
```

### 4. Incremental Adoption
```yaml
# config.yaml - Enable protocols as needed
protocols:
  http: { enabled: true }
  grpc: { enabled: true }
  mqtt: { enabled: true }   # Opt-in
  kafka: { enabled: false } # Opt-out
```

## Success Criteria

### Technical
- ✅ Protocol conformance: 100% of core features
- ✅ Performance overhead: < 5% vs native
- ✅ Memory per protocol: < 50MB baseline
- ✅ Test coverage: > 80%

### Adoption
- GitHub stars/downloads
- Protocol usage metrics
- Community feedback
- Issue/PR activity

## Risk Mitigation

| Risk | Mitigation Strategy |
|------|-------------------|
| Scope creep | Strict phase boundaries, MVP approach |
| Protocol complexity | Start simple (SMTP, FTP) before complex (Kafka) |
| Maintenance burden | Plugin system for niche protocols |
| Breaking changes | Careful API design, semantic versioning |
| Performance issues | Benchmark-driven development |

## Timeline

```
Month 1-2:  Foundation enhancements
Month 2:    SMTP implementation
Month 3:    MQTT implementation
Month 3-4:  FTP implementation
Month 4-5:  Kafka implementation (core features)
Month 6-7:  Kafka advanced features + RabbitMQ
```

**Total: 5-7 months** for comprehensive multi-protocol support

## Next Steps

### Immediate Actions

1. **Review and Approve Roadmap**
   - Validate prioritization
   - Confirm scope and timeline
   - Identify resource allocation

2. **Start with SMTP Proof of Concept**
   - Implement minimal viable SMTP server
   - Validate protocol abstraction integration
   - Test fixture loading and middleware

3. **Create GitHub Issues**
   - Break down roadmap into issues
   - Label by protocol and priority
   - Assign to milestones

4. **Update Documentation**
   - Add protocol expansion to main README
   - Create protocol comparison page
   - Update contributing guide

### Development Workflow

```bash
# For each protocol:

# 1. Create crate structure
cargo new crates/mockforge-{protocol} --lib

# 2. Implement SpecRegistry trait
# See PROTOCOL_IMPLEMENTATION_GUIDE.md

# 3. Write tests
cargo test -p mockforge-{protocol}

# 4. Add examples
# Create fixtures in examples/protocols/{protocol}/

# 5. Update CLI
# Add commands in mockforge-cli

# 6. Document
# Add to book/src/protocols/{protocol}/

# 7. Submit PR
git checkout -b feat/protocol-{protocol}
```

## Resources Created

### Documentation
1. `docs/PROTOCOL_EXPANSION_ROADMAP.md` - Strategic roadmap
2. `docs/PROTOCOL_IMPLEMENTATION_GUIDE.md` - Developer guide
3. `docs/PROTOCOL_EXPANSION_SUMMARY.md` - This document

### Examples
4. `examples/protocols/smtp/welcome-email.yaml` - SMTP fixture
5. `examples/protocols/mqtt/iot-sensors.yaml` - MQTT fixture
6. `examples/protocols/kafka/order-events.yaml` - Kafka fixture
7. `examples/protocols/ftp/file-server.yaml` - FTP fixture
8. `examples/protocols/README.md` - Examples overview

### Code Templates
- Crate structure template
- SpecRegistry implementation
- Server implementation
- Configuration integration
- CLI integration
- Complete SMTP example

## Questions to Address

1. **Priority**: Does the suggested order (SMTP → MQTT → FTP → Kafka) align with user needs?

2. **Scope**: Should we start with a single protocol MVP or multiple in parallel?

3. **Plugin vs Core**: Should FTP be plugin-based or core? What about RabbitMQ?

4. **Timeline**: Is 5-7 months reasonable? Can we accelerate with more resources?

5. **Community**: Should we open this up for community contributions with bounties?

## Conclusion

MockForge is **excellently positioned** for protocol expansion:

✅ **Strong Foundation**: Protocol abstraction layer is well-designed
✅ **Clear Path**: Incremental approach reduces risk
✅ **High Value**: Addresses real testing needs (messaging, IoT, email)
✅ **Maintainable**: Plugin system handles edge cases
✅ **Consistent**: Same patterns across all protocols

The roadmap provides a **measured, validated approach** that:
- Starts simple to validate architecture
- Builds complexity incrementally
- Maintains consistency across protocols
- Leverages existing MockForge strengths (AI, templating, fixtures)
- Provides plugin escape hatch for niche protocols

**Recommendation**: Start with SMTP as a 2-3 week proof of concept to validate the architecture before committing to the full roadmap.

---

**Next Step**: Review this summary and roadmap, then decide whether to proceed with SMTP POC or adjust priorities based on user feedback.
