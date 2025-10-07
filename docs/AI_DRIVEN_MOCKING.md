# AI-Driven Mock Generation

MockForge now includes cutting-edge AI-driven mock generation capabilities that go beyond static templates and allow you to define intent instead of explicit examples.

## Table of Contents

1. [Overview](#overview)
2. [Intelligent Mock Generation](#intelligent-mock-generation)
3. [Data Drift Simulation](#data-drift-simulation)
4. [LLM-Powered Replay Augmentation](#llm-powered-replay-augmentation)
5. [Configuration](#configuration)
6. [Examples](#examples)
7. [Best Practices](#best-practices)

## Overview

MockForge's AI-driven features leverage Large Language Models (LLMs) to:

- **Generate contextual mock data** based on natural language descriptions
- **Simulate data evolution** across requests with realistic drift patterns
- **Create dynamic event streams** for WebSocket and GraphQL subscriptions

### Supported LLM Providers

- **OpenAI** (GPT-3.5, GPT-4)
- **Anthropic** (Claude)
- **Ollama** (Local models)
- **OpenAI-compatible** APIs

## Intelligent Mock Generation

### What is Intelligent Mocking?

Instead of defining explicit mock responses, you can describe what you want in natural language, and MockForge will generate realistic, context-aware responses using AI.

### Configuration

```yaml
endpoints:
  - path: /customers
    method: GET
    response:
      mode: intelligent
      prompt: "Generate realistic customer data for a retail SaaS API"
      context: "Customers should have diverse demographics and purchase histories"
      schema:
        type: object
        properties:
          id: { type: string }
          name: { type: string }
          email: { type: string }
          tier: { type: string, enum: [bronze, silver, gold, platinum] }
      temperature: 0.7
      cache_enabled: true
```

### Response Modes

#### 1. Static Mode (Default)
Traditional template-based responses.

```yaml
response:
  mode: static
  body: { "id": "{{uuid}}", "name": "{{name}}" }
```

#### 2. Intelligent Mode
Fully AI-generated responses based on intent.

```yaml
response:
  mode: intelligent
  prompt: "Generate a product catalog entry for a tech gadget"
  schema:
    type: object
    properties:
      name: { type: string }
      description: { type: string }
      price: { type: number }
      features: { type: array }
```

#### 3. Hybrid Mode
Combines templates with AI enhancement.

```yaml
response:
  mode: hybrid
  prompt: "Enhance this product with realistic features and marketing copy"
  body:
    id: "{{uuid}}"
    name: "Gadget X"
```

### Example: Customer API

```yaml
# config/intelligent-customers.yaml
server:
  port: 8080

rag:
  provider: openai
  api_key: ${OPENAI_API_KEY}
  model: gpt-4
  temperature: 0.7

endpoints:
  - path: /customers
    method: GET
    response:
      mode: intelligent
      prompt: |
        Generate realistic customer data for a retail SaaS API.
        Include diverse demographics, realistic email addresses,
        and varied subscription tiers based on customer value.
      schema:
        type: object
        properties:
          id: { type: string }
          name: { type: string }
          email: { type: string }
          phone: { type: string }
          tier: { type: string }
          account_value: { type: number }
          signup_date: { type: string }
      count: 10
```

**Run:**

```bash
mockforge --config config/intelligent-customers.yaml
```

**Response:**

```json
{
  "id": "cust_8f2h3k9j",
  "name": "Sarah Chen",
  "email": "sarah.chen@techcorp.com",
  "phone": "+1-555-0142",
  "tier": "gold",
  "account_value": 45230.50,
  "signup_date": "2023-08-15T10:30:00Z"
}
```

## Data Drift Simulation

### What is Data Drift?

Data drift simulation allows mock data to evolve naturally over time or across requests, mimicking real-world scenarios like:

- Order statuses progressing from "pending" to "delivered"
- Stock quantities depleting
- Prices fluctuating
- User activity scores changing

### Drift Strategies

1. **Linear** - Values change linearly
2. **Stepped** - Values change at discrete intervals
3. **StateMachine** - Values transition between defined states
4. **RandomWalk** - Values change randomly within bounds
5. **Custom** - Define custom drift rules

### Configuration

```yaml
endpoints:
  - path: /orders/:id
    method: GET
    response:
      body:
        id: "{{uuid}}"
        status: "pending"
        quantity: 100
        price: 99.99
      drift:
        enabled: true
        request_based: true
        interval: 1  # Apply drift every request
        rules:
          - field: status
            strategy: state_machine
            states: [pending, processing, shipped, delivered]
            transitions:
              pending: [[processing, 0.7], [cancelled, 0.3]]
              processing: [[shipped, 0.9], [cancelled, 0.1]]
              shipped: [[delivered, 1.0]]

          - field: quantity
            strategy: linear
            rate: -1.0
            min_value: 0
            max_value: 100

          - field: price
            strategy: random_walk
            rate: 0.5
            min_value: 90.0
            max_value: 110.0
```

### Example: Order Status Progression

```yaml
# config/order-drift.yaml
endpoints:
  - path: /orders/12345
    method: GET
    response:
      body:
        id: "12345"
        status: "pending"
        items: 5
        total: 249.99
      drift:
        enabled: true
        request_based: true
        interval: 1
        rules:
          - field: status
            strategy: state_machine
            states:
              - pending
              - processing
              - shipped
              - delivered
              - cancelled
            transitions:
              pending:
                - [processing, 0.7]
                - [cancelled, 0.3]
              processing:
                - [shipped, 0.9]
                - [cancelled, 0.1]
              shipped:
                - [delivered, 1.0]
              delivered: []
              cancelled: []
```

**Test:**

```bash
# Request 1
curl http://localhost:8080/orders/12345
# {"id": "12345", "status": "pending", ...}

# Request 2 (status may progress)
curl http://localhost:8080/orders/12345
# {"id": "12345", "status": "processing", ...}

# Request 3
curl http://localhost:8080/orders/12345
# {"id": "12345", "status": "shipped", ...}
```

### Pre-defined Drift Scenarios

MockForge includes pre-configured drift scenarios:

```rust
use mockforge_data::drift::scenarios;

// Order status progression
let order_drift = scenarios::order_status_drift();

// Stock depletion
let stock_drift = scenarios::stock_depletion_drift();

// Price fluctuation
let price_drift = scenarios::price_fluctuation_drift();

// Activity score growth
let activity_drift = scenarios::activity_score_drift();
```

## LLM-Powered Replay Augmentation

### What is Replay Augmentation?

Replay augmentation enables AI-generated event streams for WebSocket and GraphQL subscriptions. Instead of pre-recording events, you describe a scenario in natural language, and MockForge generates realistic event sequences.

### Use Cases

- **Financial Data**: Simulate live market data with realistic price movements
- **Chat Applications**: Generate natural conversation flows
- **IoT Sensors**: Create realistic sensor reading patterns
- **Notifications**: Simulate system alerts and user activities

### Configuration

```yaml
websocket:
  - path: /market-data
    replay:
      mode: generated
      narrative: |
        Simulate 10 minutes of live stock market data for AAPL, GOOGL, and MSFT.
        Include realistic price movements with occasional volatility spikes,
        increasing volume during market opens, and smooth transitions.
      event_type: market_tick
      strategy: time_based
      duration_secs: 600
      event_rate: 2.0  # 2 events per second
      event_schema:
        type: object
        properties:
          symbol: { type: string }
          price: { type: number }
          volume: { type: number }
          timestamp: { type: string }
      progressive_evolution: true
```

### Replay Modes

#### 1. Static Replay
Pre-recorded event playback.

```yaml
replay:
  mode: static
  events_file: recorded-events.json
```

#### 2. Augmented Replay
Base events enhanced with AI.

```yaml
replay:
  mode: augmented
  narrative: "Add realistic metadata and timing variations"
  events_file: base-events.json
```

#### 3. Generated Replay
Fully AI-generated event streams.

```yaml
replay:
  mode: generated
  narrative: "Simulate a busy e-commerce platform during Black Friday"
```

### Example: Live Market Data Simulation

```yaml
# config/market-simulation.yaml
websocket:
  - path: /market-feed
    replay:
      mode: generated
      narrative: |
        Simulate realistic stock market data for tech companies (AAPL, GOOGL, MSFT).
        Start with opening prices and show natural intraday movements.
        Include occasional volatility spikes and realistic volume patterns.
      event_type: market_tick
      event_schema:
        type: object
        properties:
          symbol: { type: string, enum: [AAPL, GOOGL, MSFT] }
          price: { type: number }
          volume: { type: number }
          bid: { type: number }
          ask: { type: number }
          timestamp: { type: string }
      strategy: time_based
      duration_secs: 300  # 5 minutes
      event_rate: 3.0     # 3 ticks per second
      progressive_evolution: true

rag:
  provider: openai
  api_key: ${OPENAI_API_KEY}
  model: gpt-4
  temperature: 0.8
```

### Example: Chat Simulation

```yaml
# config/chat-simulation.yaml
websocket:
  - path: /chat/:room_id
    replay:
      mode: generated
      narrative: |
        Simulate a group chat conversation between 4 team members
        discussing a software project. Include natural message pacing,
        thread responses, emoji reactions, and realistic content about
        features, bugs, and deadlines.
      event_type: chat_message
      event_schema:
        type: object
        properties:
          user_id: { type: string }
          username: { type: string }
          message: { type: string }
          timestamp: { type: string }
          thread_id: { type: string }
      strategy: count_based
      event_count: 50
      event_rate: 0.5  # One message every 2 seconds
```

### Event Generation Strategies

#### Time-Based
Generate events for a specific duration.

```yaml
strategy: time_based
duration_secs: 600  # 10 minutes
event_rate: 2.0     # 2 events/second
```

#### Count-Based
Generate a specific number of events.

```yaml
strategy: count_based
event_count: 100
event_rate: 1.0
```

#### Conditional-Based
Generate events based on conditions.

```yaml
strategy: conditional_based
conditions:
  - name: volatility_spike
    expression: price_change > 5%
    action: generate_event

  - name: market_close
    expression: time > 16:00
    action: stop
```

### Pre-defined Scenario Templates

```rust
use mockforge_data::replay_augmentation::scenarios;

// Stock market simulation
let market_config = scenarios::stock_market_scenario();

// Chat messages
let chat_config = scenarios::chat_messages_scenario();

// IoT sensor data
let iot_config = scenarios::iot_sensor_scenario();
```

## Configuration

### Environment Variables

```bash
# OpenAI
export OPENAI_API_KEY=sk-...

# Anthropic
export ANTHROPIC_API_KEY=sk-ant-...

# Ollama (local)
export OLLAMA_HOST=http://localhost:11434
```

### Global RAG Configuration

```yaml
# config.yaml
rag:
  provider: openai
  api_key: ${OPENAI_API_KEY}
  model: gpt-4
  max_tokens: 2048
  temperature: 0.7

  # Embedding configuration
  embedding_provider: openai
  embedding_model: text-embedding-ada-002

  # Performance tuning
  caching: true
  cache_ttl_secs: 3600
  timeout_secs: 30
  max_retries: 3
```

### Per-Endpoint Configuration

```yaml
endpoints:
  - path: /customers
    response:
      mode: intelligent
      prompt: "..."
      temperature: 0.8  # Override global temperature
      rag_config:
        model: gpt-3.5-turbo  # Override model for this endpoint
```

## Examples

### Complete E-Commerce Scenario

```yaml
# config/ecommerce-ai.yaml
server:
  port: 8080

rag:
  provider: openai
  api_key: ${OPENAI_API_KEY}
  model: gpt-4

endpoints:
  # Intelligent product catalog
  - path: /products
    method: GET
    response:
      mode: intelligent
      prompt: |
        Generate a diverse tech product catalog with realistic
        descriptions, specifications, and pricing.
      count: 20

  # Order with drift simulation
  - path: /orders/:id
    method: GET
    response:
      mode: hybrid
      prompt: "Add realistic shipping and tracking details"
      body:
        id: "{{uuid}}"
        status: "pending"
        total: "{{random_number 50 500}}"
      drift:
        enabled: true
        request_based: true
        interval: 1
        rules:
          - field: status
            strategy: state_machine
            states: [pending, processing, shipped, delivered]
            transitions:
              pending: [[processing, 1.0]]
              processing: [[shipped, 1.0]]
              shipped: [[delivered, 1.0]]

websocket:
  # Live order notifications
  - path: /notifications
    replay:
      mode: generated
      narrative: |
        Simulate real-time order notifications for a busy e-commerce
        platform. Include new orders, status updates, and delivery
        confirmations with realistic timing.
      event_type: order_notification
      strategy: time_based
      duration_secs: 300
      event_rate: 0.5
```

### Financial Trading Platform

```yaml
# config/trading-platform.yaml
websocket:
  - path: /market-data
    replay:
      mode: generated
      narrative: |
        Simulate a realistic trading day for major tech stocks.
        Start with pre-market movements, show opening volatility,
        mid-day consolidation, and closing activity.
      event_type: quote
      strategy: time_based
      duration_secs: 3600  # 1 hour compressed trading day
      event_rate: 5.0
      progressive_evolution: true

  - path: /trades
    replay:
      mode: generated
      narrative: |
        Generate realistic trade executions with various order types,
        sizes, and timing patterns typical of algorithmic and retail trading.
      event_type: trade
      strategy: count_based
      event_count: 1000
      event_rate: 2.0
```

## Best Practices

### 1. Prompt Engineering

**Good:**
```yaml
prompt: |
  Generate customer data for a B2B SaaS platform.
  Customers should be diverse companies from tech, finance, and healthcare.
  Include realistic company names, contact details, and account metrics.
  Vary subscription tiers based on company size and industry.
```

**Less Effective:**
```yaml
prompt: "Generate customer data"
```

### 2. Schema Definition

Always provide schemas for structured outputs:

```yaml
schema:
  type: object
  properties:
    id: { type: string }
    name: { type: string }
    email: { type: string, format: email }
  required: [id, name, email]
```

### 3. Caching

Enable caching for deterministic scenarios:

```yaml
cache_enabled: true
```

Disable for unique responses:

```yaml
cache_enabled: false
```

### 4. Temperature Tuning

- **Low (0.1-0.3)**: Deterministic, consistent responses
- **Medium (0.5-0.7)**: Balanced creativity and consistency
- **High (0.8-1.0)**: Creative, varied responses

### 5. Progressive Evolution

For event streams, enable progressive evolution for realistic continuity:

```yaml
progressive_evolution: true
```

### 6. Rate Limiting

Set appropriate event rates to avoid overwhelming clients:

```yaml
event_rate: 1.0  # Start conservative
```

### 7. Error Handling

Always provide fallback schemas:

```yaml
response:
  mode: intelligent
  prompt: "..."
  # Fallback if AI generation fails
  fallback:
    body: { "error": "Generation unavailable" }
```

### 8. Cost Management

- Use GPT-3.5 for development, GPT-4 for production
- Enable caching to reduce API calls
- Use Ollama for local development (free)

```yaml
# Development
rag:
  provider: ollama
  model: llama2
  api_endpoint: http://localhost:11434

# Production
rag:
  provider: openai
  model: gpt-4
  caching: true
```

### 9. Testing

Test AI-generated responses for:
- Schema compliance
- Realistic data
- Consistency across requests
- Edge cases

### 10. Monitoring

Monitor AI API usage and costs:

```bash
mockforge --config config.yaml --log-level debug
```

## Troubleshooting

### Issue: API Key Not Found

```bash
Error: RAG is enabled but no API key is configured
```

**Solution:**

```bash
export OPENAI_API_KEY=sk-...
# or
mockforge --config config.yaml --rag-api-key sk-...
```

### Issue: Generation Timeout

```yaml
rag:
  timeout_secs: 60  # Increase timeout
  max_retries: 5    # More retries
```

### Issue: Invalid JSON Response

The AI sometimes generates invalid JSON. MockForge automatically handles this by:
1. Extracting JSON from markdown code blocks
2. Finding JSON objects within text
3. Falling back to schema defaults

To improve JSON quality:

```yaml
prompt: |
  ...
  IMPORTANT: Return ONLY valid JSON. No markdown, no explanations.
```

## Next Steps

- Explore [Advanced Examples](./examples/ai-driven/)
- Read [API Reference](./api/ai-features.md)
- Check [Performance Guide](./performance.md)
- Join [Community Discord](https://discord.gg/mockforge)
