# Intelligent Mock Behavior - Integration Guide

This guide explains how to integrate and use MockForge's Intelligent Mock Behavior system to create stateful, context-aware API mocks powered by Large Language Models (LLMs).

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Usage Examples](#usage-examples)
- [Integration with Existing Code](#integration-with-existing-code)
- [Testing](#testing)
- [Troubleshooting](#troubleshooting)
- [Best Practices](#best-practices)

## Overview

The Intelligent Mock Behavior system adds AI-powered stateful behavior to your mock APIs:

- **Session Tracking**: Maintains state across multiple requests
- **LLM Decision Making**: Uses AI to generate intelligent, context-aware responses
- **Vector Memory**: Semantic search over past interactions for long-term context
- **Consistency Rules**: Enforces logical behavior patterns (e.g., auth requirements)
- **State Machines**: Resources follow realistic lifecycle transitions

**Key Benefits:**
- ✅ Realistic integration testing with stateful behavior
- ✅ No manual configuration of complex scenarios
- ✅ Cost-effective (use free Ollama or cheap OpenAI)
- ✅ Industry-first LLM-powered mock server

## Prerequisites

### Required Dependencies

Add the following to your `Cargo.toml`:

```toml
[dependencies]
mockforge-core = { version = "1.0", features = ["intelligent-behavior"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
```

### LLM Provider Setup

You'll need access to an LLM provider. Choose one:

#### Option 1: OpenAI (Recommended for Production)

```bash
export OPENAI_API_KEY=sk-your-api-key-here
```

#### Option 2: Ollama (Free for Local Development)

```bash
# Install Ollama
curl https://ollama.ai/install.sh | sh

# Pull a model
ollama pull llama2

# Ollama runs on http://localhost:11434 by default
```

#### Option 3: Anthropic (Claude)

```bash
export ANTHROPIC_API_KEY=sk-ant-your-api-key
```

## Quick Start

### 1. Create a Configuration File

Create `config.yaml`:

```yaml
intelligent_behavior:
  enabled: true

  session_tracking:
    method: cookie
    auto_create: true

  behavior_model:
    llm_provider: ollama  # or openai, anthropic
    model: llama2         # or gpt-4, claude-3-opus
    rules:
      system_prompt: |
        You are simulating a realistic REST API.
        Maintain consistency across requests.

http:
  enabled: true
  port: 3000
```

### 2. Start MockForge

```bash
mockforge serve --config config.yaml
```

### 3. Make Requests

```bash
# First request (creates session)
curl -c cookies.txt -X POST http://localhost:3000/api/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice", "email": "alice@example.com"}'

# Response (LLM generates realistic data)
{
  "id": "user_abc123",
  "name": "Alice",
  "email": "alice@example.com",
  "created_at": "2025-01-15T10:00:00Z"
}

# Second request (uses same session)
curl -b cookies.txt http://localhost:3000/api/users/user_abc123

# Response (LLM remembers the user from the first request!)
{
  "id": "user_abc123",
  "name": "Alice",
  "email": "alice@example.com",
  "created_at": "2025-01-15T10:00:00Z"
}
```

## Configuration

### Full Configuration Schema

```yaml
intelligent_behavior:
  # Enable/disable the system
  enabled: boolean (default: false)

  # Session tracking
  session_tracking:
    method: cookie|header|query_param (default: cookie)
    cookie_name: string (default: "mockforge_session")
    header_name: string (default: "X-Session-ID")
    query_param: string (default: "session_id")
    auto_create: boolean (default: true)

  # Behavior model
  behavior_model:
    llm_provider: openai|anthropic|ollama|openai-compatible
    model: string
    api_key: string (optional, can use env var)
    api_endpoint: string (optional)
    temperature: float (0.0 to 2.0, default: 0.7)
    max_tokens: integer (default: 1024)

    rules:
      system_prompt: string
      schemas: map<string, json_schema>
      consistency_rules: array<ConsistencyRule>
      state_transitions: map<string, StateMachine>
      max_context_interactions: integer (default: 10)
      enable_semantic_search: boolean (default: true)

  # Vector store
  vector_store:
    enabled: boolean (default: false)
    embedding_provider: openai|openai-compatible
    embedding_model: string (default: "text-embedding-ada-002")
    storage_path: string (optional)
    semantic_search_limit: integer (default: 10)
    similarity_threshold: float (0.0 to 1.0, default: 0.7)

  # Performance
  performance:
    cache_ttl_seconds: integer (default: 300)
    max_history_length: integer (default: 50)
    session_timeout_seconds: integer (default: 3600)
    enable_response_cache: boolean (default: true)
```

## Usage Examples

### Example 1: Stateful Shopping Cart

```yaml
intelligent_behavior:
  enabled: true

  behavior_model:
    llm_provider: openai
    model: gpt-3.5-turbo
    rules:
      system_prompt: |
        Simulate an e-commerce API where:
        - Cart items persist across requests
        - Creating an order consumes cart items

      schemas:
        CartItem:
          type: object
          properties:
            product_id: {type: string}
            quantity: {type: integer}
```

**Request Flow:**

```bash
# Add item to cart
POST /api/cart
{"product_id": "prod_123", "quantity": 2}

# Response
{"cart": [{"product_id": "prod_123", "quantity": 2}], "subtotal": 59.98}

# Get cart (different request, same session)
GET /api/cart

# Response (remembers the item!)
{"cart": [{"product_id": "prod_123", "quantity": 2}], "subtotal": 59.98}

# Create order (consumes cart)
POST /api/orders

# Response
{"order_id": "ord_456", "total": 59.98, "status": "pending"}

# Get cart again (should be empty)
GET /api/cart

# Response
{"cart": [], "subtotal": 0}
```

### Example 2: Authentication Flow

```yaml
intelligent_behavior:
  enabled: true

  behavior_model:
    rules:
      system_prompt: |
        Users must login before accessing protected resources.

      consistency_rules:
        - name: require_auth
          condition: "path starts_with '/api/protected'"
          action:
            type: require_auth
            message: "Authentication required"
```

**Request Flow:**

```bash
# Try to access protected resource without auth
GET /api/protected/data

# Response: 401 Unauthorized
{"error": "Authentication required"}

# Login first
POST /api/login
{"email": "user@example.com", "password": "secret"}

# Response
{"token": "abc123", "user": {"id": "user_1", "email": "user@example.com"}}

# Now access protected resource
GET /api/protected/data

# Response: 200 OK (LLM remembers you're logged in)
{"data": [...]}
```

### Example 3: Resource State Transitions

```yaml
intelligent_behavior:
  enabled: true

  behavior_model:
    rules:
      state_transitions:
        order_status:
          resource_type: Order
          states: [pending, processing, shipped, delivered]
          initial_state: pending
          transitions:
            - from: pending
              to: processing
              probability: 0.8
            - from: processing
              to: shipped
              probability: 0.9
```

**Request Flow:**

```bash
# Create order
POST /api/orders
{"items": [...]}

# Response
{"id": "ord_1", "status": "pending"}

# Check order status later
GET /api/orders/ord_1

# Response (status progressed!)
{"id": "ord_1", "status": "shipped", "tracking_number": "TRK123"}
```

## Integration with Existing Code

### Using the Rust API

```rust
use mockforge_core::intelligent_behavior::{
    IntelligentBehaviorConfig,
    StatefulAiContext,
    BehaviorModel,
    VectorMemoryStore,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Create configuration
    let config = IntelligentBehaviorConfig {
        enabled: true,
        ..Default::default()
    };

    // 2. Create stateful context
    let mut context = StatefulAiContext::new("session_123", config.clone());

    // 3. Create behavior model
    let behavior = BehaviorModel::new(config.behavior_model);

    // 4. Record an interaction
    context.record_interaction(
        "POST",
        "/api/users",
        Some(serde_json::json!({"name": "Alice"})),
        Some(serde_json::json!({"id": "user_1", "name": "Alice"})),
    ).await?;

    // 5. Generate next response based on context
    let response = behavior.generate_response(
        "GET",
        "/api/users/user_1",
        None,
        &context,
    ).await?;

    println!("Generated response: {}", response);

    Ok(())
}
```

### Integrating with HTTP Handlers

```rust
use axum::{Router, routing::post, Extension, Json};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
struct AppState {
    contexts: Arc<RwLock<HashMap<String, StatefulAiContext>>>,
    behavior: Arc<BehaviorModel>,
}

async fn handle_request(
    Extension(state): Extension<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    // Get or create session
    let session_id = extract_session_id();

    // Get or create context
    let mut contexts = state.contexts.write().await;
    let context = contexts
        .entry(session_id.clone())
        .or_insert_with(|| StatefulAiContext::new(session_id, config));

    // Generate response
    let response = state.behavior.generate_response(
        "POST",
        "/api/endpoint",
        Some(body),
        context,
    ).await.unwrap();

    Json(response)
}
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stateful_context() {
        let config = IntelligentBehaviorConfig::default();
        let mut context = StatefulAiContext::new("test_session", config);

        // Record first interaction
        context.record_interaction(
            "POST",
            "/api/login",
            Some(serde_json::json!({"email": "test@example.com"})),
            Some(serde_json::json!({"token": "abc123"})),
        ).await.unwrap();

        // Verify history
        let history = context.get_history().await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].method, "POST");
    }
}
```

### Integration Tests

```bash
# Start MockForge in test mode
mockforge serve --config test-config.yaml &
SERVER_PID=$!

# Run test scenario
curl -c cookies.txt -X POST http://localhost:3000/api/login \
  -d '{"email": "test@example.com"}'

curl -b cookies.txt http://localhost:3000/api/profile

# Cleanup
kill $SERVER_PID
```

## Troubleshooting

### Issue: "Session not found"

**Cause:** Session cookie/header not being sent with request

**Solution:** Ensure you're using `-c` and `-b` flags with curl, or properly handling cookies in your client

```bash
# Save cookies
curl -c cookies.txt ...

# Use cookies
curl -b cookies.txt ...
```

### Issue: LLM responses are inconsistent

**Cause:** Temperature is too high

**Solution:** Lower the temperature in config

```yaml
behavior_model:
  temperature: 0.3  # Lower = more deterministic
```

### Issue: "API key not found"

**Cause:** API key not set in environment

**Solution:**

```bash
export OPENAI_API_KEY=sk-your-key
# or
export ANTHROPIC_API_KEY=sk-ant-your-key
```

### Issue: Slow responses

**Cause:** LLM latency, no caching

**Solution:** Enable response caching and use a faster model

```yaml
performance:
  enable_response_cache: true

behavior_model:
  model: gpt-3.5-turbo  # Faster than gpt-4
```

## Best Practices

### 1. Use Descriptive System Prompts

**Bad:**
```yaml
system_prompt: "Simulate an API"
```

**Good:**
```yaml
system_prompt: |
  You are simulating a realistic e-commerce API.
  Follow these rules:
  1. Users must login before checkout
  2. Cart items persist across requests
  3. Stock decreases when orders are placed
```

### 2. Define Clear Schemas

Provide JSON schemas for your resources so the LLM generates consistent data:

```yaml
schemas:
  User:
    type: object
    required: [id, email, name]
    properties:
      id: {type: string}
      email: {type: string, format: email}
      name: {type: string}
```

### 3. Use Consistency Rules for Critical Logic

Don't rely solely on the LLM for critical logic like authentication:

```yaml
consistency_rules:
  - name: require_auth
    condition: "path starts_with '/api/protected'"
    action:
      type: require_auth
      message: "Authentication required"
```

### 4. Optimize Performance

- **Use caching** for identical requests
- **Limit history length** to reduce context size
- **Choose appropriate models**: gpt-3.5-turbo for speed, gpt-4 for quality
- **Use Ollama locally** for development

### 5. Monitor Sessions

Use the Admin UI to inspect active sessions:

```bash
# Access Admin UI
http://localhost:9080/

# Or use CLI
mockforge intelligent inspect-session --session-id abc123
```

### 6. Version Your Prompts

Store your configuration in version control and document changes to the system prompt:

```yaml
# v1.0 - Initial e-commerce behavior
# v1.1 - Added inventory tracking
# v1.2 - Added order status transitions
system_prompt: |
  ...
```

## Next Steps

- **Read the full design doc**: [INTELLIGENT_MOCK_BEHAVIOR.md](./INTELLIGENT_MOCK_BEHAVIOR.md)
- **Try the examples**: `examples/intelligent-behavior-ecommerce.yaml`
- **Explore advanced features**: Vector memory, state machines, chain workflows
- **Join the community**: GitHub Discussions for questions and feedback

## Support

- **Issues**: https://github.com/SaaSy-Solutions/mockforge/issues
- **Discussions**: https://github.com/SaaSy-Solutions/mockforge/discussions
- **Documentation**: https://docs.mockforge.dev/

---

**MockForge Intelligent Mock Behavior** - The world's first AI-powered stateful mock server.
