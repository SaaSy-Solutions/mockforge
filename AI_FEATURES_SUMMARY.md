# AI-Driven Mock Generation - Implementation Summary

## Overview

MockForge now includes three major AI-driven features that transform it from a static mocking framework into an intelligent, adaptive mock generation platform:

1. **Intelligent Mock Generation** - Context-aware responses from natural language prompts
2. **Data Drift Simulation** - Realistic data evolution across requests
3. **LLM-Powered Replay Augmentation** - AI-generated event streams for WebSocket/GraphQL

## Implementation Details

### 1. Intelligent Mock Generation

**Location:** `crates/mockforge-data/src/intelligent_mock.rs`

**Key Components:**

- `ResponseMode` enum: Static, Intelligent, Hybrid
- `IntelligentMockConfig`: Configuration for AI-driven responses
- `IntelligentMockGenerator`: Core generation engine

**Features:**

- Natural language prompts instead of explicit examples
- Schema-aware generation with JSON schema support
- Temperature control for creativity
- Caching for performance
- Automatic JSON extraction from LLM responses
- Hybrid mode combining templates with AI enhancement

**Example Usage:**

```yaml
response:
  mode: intelligent
  prompt: "Generate realistic customer data for a retail SaaS API"
  schema:
    type: object
    properties:
      id: { type: string }
      name: { type: string }
      email: { type: string }
  temperature: 0.7
```

### 2. Data Drift Simulation

**Location:** `crates/mockforge-data/src/drift.rs`

**Key Components:**

- `DriftStrategy` enum: Linear, Stepped, StateMachine, RandomWalk, Custom
- `DriftRule`: Rule configuration for field-level drift
- `DataDriftConfig`: Overall drift configuration
- `DataDriftEngine`: Runtime drift application engine

**Features:**

- Multiple drift strategies for different data patterns
- State machine for status progressions
- Time-based and request-based drift triggers
- Configurable drift rates and bounds
- Reproducible drift with seeding
- Pre-defined scenario templates

**Example Usage:**

```yaml
drift:
  enabled: true
  request_based: true
  interval: 1
  rules:
    - field: status
      strategy: state_machine
      states: [pending, processing, shipped, delivered]
      transitions:
        pending: [[processing, 0.7], [cancelled, 0.3]]
        processing: [[shipped, 0.9], [cancelled, 0.1]]
```

### 3. LLM-Powered Replay Augmentation

**Location:** `crates/mockforge-data/src/replay_augmentation.rs`

**Key Components:**

- `ReplayMode` enum: Static, Augmented, Generated
- `EventStrategy` enum: TimeBased, CountBased, ConditionalBased
- `ReplayAugmentationConfig`: Configuration for event generation
- `ReplayAugmentationEngine`: Event stream generator
- `GeneratedEvent`: Event data structure

**Features:**

- Narrative-driven event generation
- Multiple generation strategies
- Progressive scenario evolution
- Event schema validation
- Realistic event timing and pacing
- Pre-defined scenario templates (market data, chat, IoT)

**Example Usage:**

```yaml
websocket:
  - path: /market-data
    replay:
      mode: generated
      narrative: "Simulate 10 minutes of live market data with realistic price movements"
      event_type: market_tick
      strategy: time_based
      duration_secs: 600
      event_rate: 2.0
```

## Integration with Existing Features

### RAG Engine Enhancement

Added `generate_text()` method to `RagEngine` (`crates/mockforge-data/src/rag.rs:896-899`):

```rust
pub async fn generate_text(&self, prompt: &str) -> Result<String> {
    self.call_llm(prompt).await
}
```

This provides a simple interface for intelligent mock and replay modules to generate text using configured LLM providers.

### Supported LLM Providers

All features support multiple LLM providers:

- **OpenAI**: GPT-3.5, GPT-4, GPT-4-turbo
- **Anthropic**: Claude 2, Claude 3 (Opus, Sonnet, Haiku)
- **Ollama**: Local models (llama2, mistral, codellama, etc.)
- **OpenAI-compatible**: Any OpenAI-compatible API

## Configuration

### Global RAG Configuration

```yaml
rag:
  provider: openai
  api_key: ${OPENAI_API_KEY}
  model: gpt-4
  max_tokens: 2048
  temperature: 0.7

  # Embedding configuration (for future RAG features)
  embedding_provider: openai
  embedding_model: text-embedding-ada-002

  # Performance
  caching: true
  cache_ttl_secs: 3600
  timeout_secs: 30
  max_retries: 3
```

### Per-Feature Overrides

Each feature can override global RAG configuration:

```yaml
response:
  mode: intelligent
  rag_config:
    model: gpt-3.5-turbo  # Override for this endpoint
    temperature: 0.9
```

## Examples

### Complete Examples

1. **Intelligent Customer API** (`examples/ai/intelligent-customer-api.yaml`)
   - Demonstrates intelligent, hybrid, and static response modes
   - Context-aware customer data generation
   - Schema validation

2. **Order Drift Simulation** (`examples/ai/order-drift-simulation.yaml`)
   - Order status progression
   - Inventory depletion
   - Price fluctuation
   - Time-based and request-based drift

3. **WebSocket Market Simulation** (`examples/ai/websocket-market-simulation.yaml`)
   - Live market data streams
   - Chat room simulation
   - IoT sensor data
   - Real-time notifications

## Documentation

Comprehensive documentation created:

- **`docs/AI_DRIVEN_MOCKING.md`** - Complete guide with:
  - Overview and capabilities
  - Configuration reference
  - Detailed examples
  - Best practices
  - Troubleshooting
  - Performance tuning

## Testing

### Manual Testing

Test intelligent mock generation:

```bash
export OPENAI_API_KEY=sk-...
mockforge serve --config examples/ai/intelligent-customer-api.yaml
curl http://localhost:8080/customers
```

Test data drift:

```bash
mockforge serve --config examples/ai/order-drift-simulation.yaml

# Make multiple requests to see drift
for i in {1..10}; do
  curl http://localhost:8080/orders/12345
  echo ""
  sleep 1
done
```

Test WebSocket replay:

```bash
export OPENAI_API_KEY=sk-...
mockforge serve --config examples/ai/websocket-market-simulation.yaml

# Connect with wscat
npm install -g wscat
wscat -c ws://localhost:8080/market-data
```

### Unit Tests

All new modules include comprehensive unit tests:

- `intelligent_mock.rs`: Configuration validation, mode selection, JSON extraction
- `drift.rs`: Strategy application, state machines, bounds checking
- `replay_augmentation.rs`: Event generation, timing, scenario templates

Run tests:

```bash
cargo test --package mockforge-data
```

## Performance Considerations

### Caching

Intelligent mock generation includes built-in caching:

```rust
if self.config.cache_enabled {
    if let Some(cached) = self.cache.get(&cache_key) {
        return Ok(cached.clone());
    }
}
```

Reduces API calls and improves response times.

### Rate Limiting

Configure appropriate rates for event generation:

```yaml
event_rate: 1.0  # 1 event per second
```

Prevents overwhelming clients and reduces costs.

### Async Design

All AI features use async/await:

```rust
pub async fn generate(&mut self) -> Result<Value>
pub async fn apply_drift(&self, data: Value) -> Result<Value>
pub async fn generate_stream(&mut self) -> Result<Vec<GeneratedEvent>>
```

Ensures non-blocking operation in the MockForge server.

## Cost Management

### Development vs Production

**Development (free/low-cost):**

```yaml
rag:
  provider: ollama
  model: llama2
  api_endpoint: http://localhost:11434
```

**Production:**

```yaml
rag:
  provider: openai
  model: gpt-3.5-turbo  # Lower cost
  caching: true         # Reduce API calls
```

### Model Selection

- **GPT-3.5-turbo**: Fast, cost-effective for most use cases
- **GPT-4**: Higher quality for complex scenarios
- **Ollama**: Free for local development

## Next Steps

### Future Enhancements

1. **Intelligent Schema Inference**
   - Automatically infer schemas from OpenAPI specs
   - Generate prompts from schema descriptions

2. **Advanced Drift Strategies**
   - Seasonal patterns
   - Trend detection
   - Multi-field correlations

3. **Replay Learning**
   - Learn patterns from real event streams
   - Fine-tune generation based on actual data

4. **Cost Optimization**
   - Prompt caching
   - Response compression
   - Batch generation

5. **UI Integration**
   - Visual prompt editor
   - Drift rule builder
   - Event stream visualizer

### Integration Tasks

1. Update main MockForge server to expose AI features
2. Add CLI commands for AI configuration
3. Create UI components for AI feature management
4. Add metrics and monitoring for AI usage

## Migration Guide

### From Static to Intelligent Mocks

**Before:**

```yaml
response:
  body:
    id: "{{uuid}}"
    name: "John Doe"
    email: "john@example.com"
```

**After:**

```yaml
response:
  mode: intelligent
  prompt: "Generate realistic customer data"
  schema:
    type: object
    properties:
      id: { type: string }
      name: { type: string }
      email: { type: string }
```

### Adding Drift to Existing Endpoints

```yaml
# Add drift configuration to existing response
response:
  body: { ... existing response ... }
  drift:
    enabled: true
    request_based: true
    interval: 1
    rules:
      - field: status
        strategy: state_machine
        ...
```

## Troubleshooting

### Common Issues

1. **API Key Not Found**
   ```bash
   export OPENAI_API_KEY=sk-...
   ```

2. **Timeout Errors**
   ```yaml
   rag:
     timeout_secs: 60
     max_retries: 5
   ```

3. **Invalid JSON Response**
   - Improve prompt specificity
   - Add explicit JSON format instructions
   - Use lower temperature for determinism

4. **Drift Not Applying**
   - Check `enabled: true`
   - Verify interval matches request pattern
   - Ensure field paths are correct

## Conclusion

MockForge now offers industry-leading AI-driven mock generation capabilities that set it apart from competitors. The combination of intelligent responses, realistic data drift, and dynamic event streams creates a powerful platform for:

- **API Development**: Rapidly prototype with realistic mock data
- **Testing**: Simulate complex scenarios and edge cases
- **Demo Environments**: Create convincing demonstrations
- **Load Testing**: Generate dynamic, realistic traffic patterns
- **Training**: Safe environments with production-like data

These features position MockForge as the most advanced mock server framework available, combining traditional mocking capabilities with cutting-edge AI technology.
