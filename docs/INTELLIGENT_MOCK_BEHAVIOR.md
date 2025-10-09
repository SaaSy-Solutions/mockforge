# Intelligent Mock Behavior System

## Overview

The Intelligent Mock Behavior system transforms MockForge from a static mock server into a **pseudo-AI service simulator** that maintains stateful, context-aware behavior across multiple API calls. Using Large Language Models (LLMs) and vector storage, the system can:

- **Remember state across requests**: Items created via POST are returned in subsequent GET calls
- **Ensure logical consistency**: Login flows are tracked, data updates reflect in queries
- **Make intelligent decisions**: The LLM determines appropriate responses based on conversation history
- **Simulate realistic workflows**: Multi-step processes (login → fetch → update → delete) maintain coherence

## Architecture

### Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                    HTTP/gRPC Request                        │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│              Intelligent Mock Handler                       │
│  - Request classification                                   │
│  - Context extraction                                       │
└─────────────────────┬───────────────────────────────────────┘
                      │
                      ▼
┌─────────────────────────────────────────────────────────────┐
│            Stateful AI Context Manager                      │
│  - Session tracking                                         │
│  - Conversation history                                     │
│  - State snapshots                                          │
└─────────┬───────────────────────────────┬───────────────────┘
          │                               │
          ▼                               ▼
┌─────────────────────┐         ┌──────────────────────────┐
│   Vector Store      │         │   Behavior Model         │
│  - Long-term memory │         │  - LLM-based logic       │
│  - Semantic search  │         │  - Decision making       │
│  - State snapshots  │         │  - Consistency rules     │
└─────────────────────┘         └────────────┬─────────────┘
                                             │
                                             ▼
                                  ┌──────────────────────────┐
                                  │    Chain Execution       │
                                  │  - Dependency tracking   │
                                  │  - Response templating   │
                                  └──────────────────────────┘
```

### 1. Stateful AI Context Manager

Manages the conversational state of API interactions:

```rust
pub struct StatefulAiContext {
    /// Unique session identifier
    session_id: String,

    /// Conversation history (request/response pairs)
    history: Vec<InteractionRecord>,

    /// Current state snapshot (in-memory cache)
    state: HashMap<String, serde_json::Value>,

    /// Vector store for long-term memory
    vector_store: Arc<VectorStore>,

    /// Configuration
    config: StatefulContextConfig,
}

pub struct InteractionRecord {
    /// Timestamp
    timestamp: chrono::DateTime<chrono::Utc>,

    /// HTTP method and path
    method: String,
    path: String,

    /// Request body
    request: Option<serde_json::Value>,

    /// Response body
    response: Option<serde_json::Value>,

    /// Generated embedding for semantic search
    embedding: Option<Vec<f32>>,
}
```

**Key Features:**
- **Session Management**: Track users across multiple requests via session IDs
- **Conversation History**: Maintain a chronological record of all interactions
- **State Snapshots**: Cache current state (e.g., created resources, user sessions)
- **Vector Embeddings**: Store interaction summaries for semantic retrieval

### 2. Behavior Model

LLM-powered logic engine that makes intelligent decisions:

```rust
pub struct BehaviorModel {
    /// LLM provider (OpenAI, Anthropic, Ollama, etc.)
    llm: Arc<dyn LlmProviderTrait>,

    /// Behavior rules and patterns
    rules: BehaviorRules,

    /// Chain execution engine
    chain_engine: Arc<ChainExecutionEngine>,
}

pub struct BehaviorRules {
    /// System prompt describing the API's behavior
    system_prompt: String,

    /// Resource schemas (e.g., User, Product, Order)
    schemas: HashMap<String, serde_json::Value>,

    /// Consistency rules (e.g., "users must login before accessing data")
    consistency_rules: Vec<ConsistencyRule>,

    /// State transitions (e.g., order status progression)
    transitions: HashMap<String, StateMachine>,
}

pub struct ConsistencyRule {
    /// Rule name
    name: String,

    /// Condition for applying the rule
    condition: String,

    /// Action to take if violated
    action: RuleAction,
}

pub enum RuleAction {
    /// Return an error response
    Error { status: u16, message: String },

    /// Modify the request before processing
    Transform(String),

    /// Trigger a chain of requests
    ExecuteChain(String),
}
```

**Key Features:**
- **Intelligent Decision Making**: The LLM determines responses based on context
- **Consistency Enforcement**: Rules ensure logical behavior (e.g., can't delete non-existent items)
- **State Machine Transitions**: Resources follow realistic lifecycle patterns
- **Dynamic Schema Generation**: The LLM creates realistic data conforming to schemas

### 3. Vector Store Integration

Persistent memory using semantic embeddings:

```rust
pub struct VectorMemoryStore {
    /// RAG storage backend
    storage: Arc<dyn VectorStorage>,

    /// Embedding provider
    embedder: Arc<dyn EmbeddingProviderTrait>,
}

impl VectorMemoryStore {
    /// Store an interaction with semantic embedding
    pub async fn store_interaction(
        &self,
        session_id: &str,
        interaction: &InteractionRecord,
    ) -> Result<()>;

    /// Retrieve relevant past interactions
    pub async fn retrieve_context(
        &self,
        session_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<InteractionRecord>>;

    /// Get all state for a session
    pub async fn get_session_state(
        &self,
        session_id: &str,
    ) -> Result<HashMap<String, serde_json::Value>>;
}
```

**Key Features:**
- **Semantic Search**: Find relevant past interactions based on meaning, not exact matches
- **Long-term Memory**: Persist state beyond a single chain execution
- **Cross-request Context**: The LLM can reference any past interaction

## Usage Examples

### Example 1: E-commerce Workflow with Stateful Memory

**Configuration:**

```yaml
intelligent_behavior:
  enabled: true
  session_tracking: cookie  # or header, query_param

  behavior_model:
    llm_provider: openai
    model: gpt-4
    system_prompt: |
      You are simulating a realistic e-commerce API. Maintain consistency:
      - Users must login before accessing their cart
      - Items added to cart persist across requests
      - Order creation consumes cart items
      - Stock quantities decrease when orders are placed

    schemas:
      User:
        type: object
        properties:
          id: { type: string }
          email: { type: string }
          name: { type: string }

      CartItem:
        type: object
        properties:
          product_id: { type: string }
          quantity: { type: integer }
          price: { type: number }

      Order:
        type: object
        properties:
          id: { type: string }
          user_id: { type: string }
          items: { type: array }
          total: { type: number }
          status: { type: string }

    consistency_rules:
      - name: require_auth
        condition: "path starts_with '/api/cart' OR path starts_with '/api/orders'"
        action:
          error:
            status: 401
            message: "Authentication required"

      - name: cart_to_order
        condition: "method == 'POST' AND path == '/api/orders'"
        action:
          transform: "Create order from cart, clear cart, update stock"

    state_transitions:
      order_status:
        states: [pending, processing, shipped, delivered, cancelled]
        initial: pending
        transitions:
          - from: pending
            to: processing
            probability: 0.8
          - from: processing
            to: shipped
            probability: 0.9

  vector_store:
    enabled: true
    embedding_provider: openai
    embedding_model: text-embedding-ada-002
```

**Request Flow:**

```bash
# 1. User logs in
POST /api/auth/login
{
  "email": "alice@example.com",
  "password": "secret"
}

# Response (LLM generates realistic user data)
{
  "user": {
    "id": "usr_abc123",
    "email": "alice@example.com",
    "name": "Alice Johnson"
  },
  "token": "tok_xyz789"
}

# → Stored in context: user session, auth token

# 2. Add item to cart
POST /api/cart
Headers: { "Authorization": "Bearer tok_xyz789" }
{
  "product_id": "prod_456",
  "quantity": 2
}

# Response (LLM remembers the user from step 1)
{
  "cart": [
    {
      "product_id": "prod_456",
      "quantity": 2,
      "price": 29.99
    }
  ],
  "subtotal": 59.98
}

# → Stored in context: cart items

# 3. Get cart (different request, same session)
GET /api/cart
Headers: { "Authorization": "Bearer tok_xyz789" }

# Response (LLM retrieves cart from step 2)
{
  "cart": [
    {
      "product_id": "prod_456",
      "quantity": 2,
      "price": 29.99
    }
  ],
  "subtotal": 59.98
}

# 4. Create order
POST /api/orders
Headers: { "Authorization": "Bearer tok_xyz789" }

# Response (LLM converts cart to order, clears cart)
{
  "order": {
    "id": "ord_def456",
    "user_id": "usr_abc123",
    "items": [
      {
        "product_id": "prod_456",
        "quantity": 2,
        "price": 29.99
      }
    ],
    "total": 59.98,
    "status": "pending"
  }
}

# 5. Get order status (later request)
GET /api/orders/ord_def456
Headers: { "Authorization": "Bearer tok_xyz789" }

# Response (LLM applies state machine transition)
{
  "order": {
    "id": "ord_def456",
    "status": "shipped",  # Progressed from "pending"
    "tracking_number": "TRK789456123"
  }
}

# 6. Get cart again (should be empty after order)
GET /api/cart
Headers: { "Authorization": "Bearer tok_xyz789" }

# Response (LLM remembers cart was consumed)
{
  "cart": [],
  "subtotal": 0
}
```

### Example 2: Social Media API with Relationship Tracking

**Scenario:** A social media API where users can create posts, follow other users, and see personalized feeds.

**Configuration:**

```yaml
intelligent_behavior:
  enabled: true

  behavior_model:
    llm_provider: anthropic
    model: claude-3-sonnet-20240229
    system_prompt: |
      You are simulating a social media API. Maintain these rules:
      - Posts belong to specific users
      - Feed shows posts from followed users only
      - Cannot like your own posts
      - Follow relationships are bidirectional (following/followers)

    consistency_rules:
      - name: feed_authorization
        condition: "path == '/api/feed'"
        action:
          transform: "Show only posts from users that the current user follows"

      - name: no_self_like
        condition: "method == 'POST' AND path matches '/api/posts/*/like'"
        action:
          transform: "If post author == current user, return 400 error"
```

**Request Flow:**

```bash
# 1. Create a post
POST /api/posts
Headers: { "Authorization": "Bearer user1_token" }
{
  "content": "Hello world! This is my first post."
}

# Response
{
  "id": "post_123",
  "author_id": "user1",
  "content": "Hello world! This is my first post.",
  "likes": 0,
  "created_at": "2025-01-15T10:00:00Z"
}

# 2. Follow another user
POST /api/users/user2/follow
Headers: { "Authorization": "Bearer user1_token" }

# Response (LLM tracks relationship)
{
  "following": true,
  "follower_count": 42,
  "following_count": 15
}

# 3. Get feed (should include user2's posts now)
GET /api/feed
Headers: { "Authorization": "Bearer user1_token" }

# Response (LLM filters based on follows)
{
  "posts": [
    {
      "id": "post_456",
      "author_id": "user2",
      "author_name": "Bob Smith",
      "content": "Check out this cool feature!",
      "likes": 23
    }
  ]
}
```

## Implementation Plan

### Phase 1: Core Infrastructure

**File:** `crates/mockforge-core/src/intelligent_behavior/mod.rs`

```rust
pub mod context;      // StatefulAiContext
pub mod behavior;     // BehaviorModel
pub mod memory;       // VectorMemoryStore
pub mod rules;        // ConsistencyRule, StateMachine
pub mod session;      // Session management
```

### Phase 2: Integration with Chain Execution

Extend the existing chain execution system to support stateful AI:

```rust
// In ChainExecutionContext
pub struct ChainExecutionContext {
    // ... existing fields

    /// Optional AI context for intelligent behavior
    pub ai_context: Option<Arc<StatefulAiContext>>,
}
```

### Phase 3: Configuration Schema

```yaml
# config.yaml
intelligent_behavior:
  enabled: boolean

  session_tracking:
    method: cookie|header|query_param
    cookie_name: string (default: "mockforge_session")
    header_name: string (default: "X-Session-ID")
    query_param: string (default: "session_id")

  behavior_model:
    llm_provider: openai|anthropic|ollama|openai-compatible
    model: string
    api_key: string (or env var)
    api_endpoint: string (optional)

    system_prompt: string
    schemas: map<string, json_schema>
    consistency_rules: array<ConsistencyRule>
    state_transitions: map<string, StateMachine>

  vector_store:
    enabled: boolean
    embedding_provider: openai|openai-compatible
    embedding_model: string
    storage_path: string (optional, defaults to in-memory)

  performance:
    cache_ttl_seconds: integer (default: 300)
    max_history_length: integer (default: 50)
    semantic_search_limit: integer (default: 10)
```

### Phase 4: CLI Commands

```bash
# Test intelligent behavior
mockforge test-ai intelligent-behavior \
  --config examples/intelligent/ecommerce.yaml \
  --scenario examples/intelligent/ecommerce-flow.json

# Inspect session state
mockforge intelligent inspect-session \
  --session-id abc123 \
  --format json

# Clear session state
mockforge intelligent clear-session \
  --session-id abc123

# Export session history
mockforge intelligent export-session \
  --session-id abc123 \
  --output session-history.json
```

## Benefits

### 1. Realistic Integration Testing
- Test complete user journeys with stateful behavior
- Verify that your frontend handles state changes correctly
- Catch edge cases (e.g., deleting already-deleted items)

### 2. Development Acceleration
- Frontend teams can develop against a "smart" backend before the real API is ready
- No need to manually configure complex mock scenarios
- The LLM generates realistic, diverse test data

### 3. Cost-Effective
- Use Ollama for free local development
- OpenAI GPT-3.5 costs ~$0.01 per 1,000 requests
- No need for expensive test environments

### 4. Unique Competitive Advantage
- **Industry First**: No other mocking framework has LLM-powered stateful behavior
- **Beyond Static Mocks**: Simulates a real, thinking API
- **Easy to Configure**: Natural language rules instead of complex scripts

## Performance Considerations

### Caching Strategy
- Cache LLM responses for identical request patterns
- Use embeddings for semantic similarity matching
- TTL-based cache invalidation

### Optimization
- Batch embedding generation for multiple interactions
- Lazy loading of vector store results
- Asynchronous state persistence

### Scalability
- Horizontal scaling: Separate vector store instances per session
- Session partitioning for high-traffic scenarios
- Optional Redis backend for distributed caching

## Future Enhancements

### 1. Multi-Agent Simulation
- Simulate multiple users interacting simultaneously
- Test race conditions and concurrent access patterns

### 2. Behavior Learning
- The system learns from real API traffic
- Automatically refines behavior models based on observed patterns

### 3. Time-Travel Debugging
- Replay past sessions
- Inspect state at any point in time
- "What if" scenario testing

### 4. Visual Behavior Designer
- UI for defining behavior rules
- Drag-and-drop state machine editor
- Live session monitoring dashboard

## Security Considerations

- **LLM Prompt Injection**: Sanitize user inputs before passing to LLM
- **State Isolation**: Sessions are isolated to prevent cross-contamination
- **API Key Management**: Secure storage of LLM provider credentials
- **Rate Limiting**: Prevent abuse of AI-powered endpoints

## Conclusion

The Intelligent Mock Behavior system transforms MockForge into a groundbreaking tool that sets it apart from all competitors. By combining LLM intelligence with stateful memory, it creates a pseudo-AI service simulator that feels like a real, thinking backend.

This is not just a mock server—it's a **smart API simulator** that understands context, maintains consistency, and makes intelligent decisions.
