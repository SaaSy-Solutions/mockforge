# 🧠 AI-Driven Mock Generation - Quick Start

MockForge now supports next-generation AI-driven mock generation! This guide will get you started in 5 minutes.

## 🚀 Quick Start

### 1. Install & Setup

```bash
# Clone MockForge
git clone https://github.com/your-org/mockforge
cd mockforge

# Set your OpenAI API key
export OPENAI_API_KEY=sk-...

# Or use Ollama for free local AI
ollama pull llama2
export OLLAMA_HOST=http://localhost:11434
```

### 2. Your First Intelligent Mock

Create `config.yaml`:

```yaml
server:
  port: 8080

rag:
  provider: openai
  api_key: ${OPENAI_API_KEY}
  model: gpt-3.5-turbo

endpoints:
  - path: /customers
    method: GET
    response:
      mode: intelligent
      prompt: "Generate realistic customer data for a SaaS platform"
      schema:
        type: object
        properties:
          id: { type: string }
          name: { type: string }
          email: { type: string }
      count: 5
```

Run it:

```bash
mockforge serve --config config.yaml
curl http://localhost:8080/customers
```

**Output:** Realistic, AI-generated customer data!

```json
{
  "id": "cust_8f2h3k9j",
  "name": "Sarah Chen",
  "email": "sarah.chen@techcorp.com"
}
```

## 🎯 Three Powerful Features

### 1️⃣ Intelligent Mock Generation

**What:** Define intent, not examples. AI generates realistic mock data.

**Use case:** Rapidly prototype APIs with production-like data.

```yaml
response:
  mode: intelligent
  prompt: "Generate customer data for a fintech app with diverse demographics"
```

[Full Guide →](./AI_DRIVEN_MOCKING.md#intelligent-mock-generation)

### 2️⃣ Data Drift Simulation

**What:** Mock data evolves across requests - orders progress, stock depletes, prices change.

**Use case:** Test stateful workflows and edge cases.

```yaml
drift:
  enabled: true
  rules:
    - field: status
      strategy: state_machine
      states: [pending, processing, shipped, delivered]
```

[Full Guide →](./AI_DRIVEN_MOCKING.md#data-drift-simulation)

### 3️⃣ LLM-Powered Event Streams

**What:** Generate realistic WebSocket/GraphQL event streams from narrative descriptions.

**Use case:** Test real-time features without live data sources.

```yaml
websocket:
  - path: /market-data
    replay:
      mode: generated
      narrative: "Simulate 10 minutes of live stock market data"
```

[Full Guide →](./AI_DRIVEN_MOCKING.md#llm-powered-replay-augmentation)

## 📚 Examples

### Example 1: Intelligent Customer API

```yaml
# examples/ai/intelligent-customer-api.yaml
endpoints:
  - path: /customers
    response:
      mode: intelligent
      prompt: |
        Generate diverse customer data for a retail SaaS platform.
        Include realistic company names, tiers, and account values.
```

Run:
```bash
mockforge serve --config examples/ai/intelligent-customer-api.yaml
```

### Example 2: Order Status Progression

```yaml
# examples/ai/order-drift-simulation.yaml
endpoints:
  - path: /orders/:id
    response:
      body:
        status: "pending"
      drift:
        enabled: true
        rules:
          - field: status
            strategy: state_machine
            transitions:
              pending: [[processing, 0.8], [cancelled, 0.2]]
```

Test:
```bash
# Watch order status evolve
for i in {1..5}; do curl http://localhost:8080/orders/123; sleep 1; done
```

### Example 3: Live Market Data Stream

```yaml
# examples/ai/websocket-market-simulation.yaml
websocket:
  - path: /market-data
    replay:
      mode: generated
      narrative: "Simulate realistic stock prices for AAPL, GOOGL, MSFT"
      event_rate: 2.0
```

Connect:
```bash
wscat -c ws://localhost:8080/market-data
```

## 🎛️ Configuration

### Minimal Configuration

```yaml
rag:
  provider: openai
  api_key: ${OPENAI_API_KEY}
  model: gpt-3.5-turbo
```

### Full Configuration

```yaml
rag:
  # Provider
  provider: openai
  api_key: ${OPENAI_API_KEY}
  model: gpt-4

  # Generation
  max_tokens: 2048
  temperature: 0.7

  # Performance
  caching: true
  cache_ttl_secs: 3600
  timeout_secs: 30
  max_retries: 3
```

### Supported Providers

| Provider | Setup | Cost |
|----------|-------|------|
| **OpenAI** | `export OPENAI_API_KEY=sk-...` | $$ |
| **Anthropic** | `export ANTHROPIC_API_KEY=sk-ant-...` | $$ |
| **Ollama** | `ollama pull llama2` | **FREE** |

## 💡 Common Use Cases

### 1. API Development
Generate realistic mock data while building APIs.

### 2. Frontend Development
Mock backend responses without waiting for API implementation.

### 3. Testing
Create edge cases and complex scenarios automatically.

### 4. Demos
Impressive demonstrations with production-like data.

### 5. Load Testing
Generate dynamic, realistic traffic patterns.

## 🔧 Tips & Tricks

### Better Prompts = Better Results

**❌ Vague:**
```yaml
prompt: "Generate customer data"
```

**✅ Specific:**
```yaml
prompt: |
  Generate customer data for a B2B SaaS platform.
  Include diverse industries (tech, finance, healthcare).
  Vary subscription tiers based on company size.
  Use realistic email addresses with company domains.
```

### Use Schemas

Always provide schemas for structured output:

```yaml
schema:
  type: object
  properties:
    id: { type: string }
    email: { type: string, format: email }
  required: [id, email]
```

### Free Development with Ollama

```yaml
# Development (free)
rag:
  provider: ollama
  model: llama2
  api_endpoint: http://localhost:11434

# Production (paid)
rag:
  provider: openai
  model: gpt-3.5-turbo
```

### Progressive Evolution

For event streams, enable evolution:

```yaml
progressive_evolution: true
```

Events build on previous context for realistic continuity.

## 📖 Full Documentation

- **[Complete Guide](./AI_DRIVEN_MOCKING.md)** - Comprehensive documentation
- **[Examples](../examples/ai/)** - Ready-to-run configurations
- **[API Reference](./api/ai-features.md)** - Detailed API docs

## 🆚 Comparison

| Feature | MockForge AI | WireMock | Mockoon |
|---------|-------------|----------|---------|
| **AI Generation** | ✅ Full | ❌ No | ❌ No |
| **Data Drift** | ✅ Yes | ❌ No | ❌ No |
| **AI Event Streams** | ✅ Yes | ❌ No | ❌ No |
| **Local AI (Free)** | ✅ Ollama | ❌ No | ❌ No |

## 🚀 What's Next?

1. **Try the examples:** `mockforge serve --config examples/ai/*.yaml`
2. **Read the full guide:** [AI_DRIVEN_MOCKING.md](./AI_DRIVEN_MOCKING.md)
3. **Experiment:** Start with intelligent mode, then add drift and streams
4. **Join community:** Share your AI-driven mock configurations!

## ❓ FAQ

**Q: Do I need an API key?**
A: Not if you use Ollama (free, local AI). For production, we recommend OpenAI or Anthropic.

**Q: How much does it cost?**
A: With GPT-3.5-turbo and caching, typically $0.01-0.05 per 1000 API calls. Ollama is free.

**Q: Is my data sent to AI providers?**
A: Only prompts and schemas. Your actual application data stays private.

**Q: Can I use this offline?**
A: Yes, with Ollama running locally.

**Q: Does this work with GraphQL/gRPC?**
A: Yes! AI features work across all MockForge protocols.

## 🐛 Troubleshooting

**API Key Error:**
```bash
export OPENAI_API_KEY=sk-...
```

**Timeout:**
```yaml
rag:
  timeout_secs: 60
```

**Invalid JSON:**
Improve prompt specificity:
```yaml
prompt: |
  ...
  IMPORTANT: Return ONLY valid JSON.
```

## 🌟 Real-World Example

Here's a complete e-commerce mock with all three AI features:

```yaml
# Complete E-Commerce API
server:
  port: 8080

rag:
  provider: openai
  api_key: ${OPENAI_API_KEY}
  model: gpt-4

endpoints:
  # Intelligent product catalog
  - path: /products
    response:
      mode: intelligent
      prompt: "Generate diverse tech product catalog with specs and pricing"
      count: 20

  # Order with drift
  - path: /orders/:id
    response:
      body:
        id: "{{params.id}}"
        status: "pending"
      drift:
        enabled: true
        rules:
          - field: status
            strategy: state_machine
            states: [pending, processing, shipped, delivered]

websocket:
  # Real-time notifications
  - path: /notifications
    replay:
      mode: generated
      narrative: "Simulate real-time order updates and delivery notifications"
      event_rate: 1.0
```

Run it:
```bash
mockforge serve --config ecommerce.yaml
```

**Result:** A complete, realistic e-commerce API with intelligent products, evolving orders, and live notifications!

---

**Ready to revolutionize your mocking workflow?**

Start with: `mockforge serve --config examples/ai/intelligent-customer-api.yaml`

Questions? Check the [full documentation](./AI_DRIVEN_MOCKING.md) or open an issue!
