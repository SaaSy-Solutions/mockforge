# Smart Personas & Reality Continuum v2

**Pillars:** [Reality][AI]

[Reality] - Makes mocks feel like real backends through consistent persona data
[AI] - LLM-powered data generation and relationship inference

**Version:** 2.0.0
**Tags:** `#reality` `#AI` `#personas` `#graphs` `#fidelity`

## Overview

Smart Personas v2 upgrades MockForge from "random-but-consistent fake data" to a "coherent world simulation." This enables personas to maintain relationships across entities (users, orders, payments, webhooks, TCP messages) and provides a concrete way to assess "how real" your mock world is through fidelity scores.

## Key Features

- **Persona Graphs & Relationships**: Link personas across entities and ensure responses share the same underlying graph state
- **Lifecycle / Time Awareness**: Persona states (NEW, ACTIVE, CHURN_RISK, CHURNED) that influence responses across endpoints
- **Reality Continuum Integration**: Reality level influences how much persona data is synthetic, recorded, or blended with upstream
- **Fidelity Score**: Quantify how close your mock environment is to the real upstream

---

## Persona Graphs & Relationships

### What Are Persona Graphs?

Persona graphs are graph-based data structures that link personas across different entity types. When you request a user, their orders, payments, and related entities are all connected through the graph, ensuring coherent data across all endpoints.

### How It Works

1. **Automatic Linking**: When entities are registered, the system automatically establishes relationships based on common patterns:
   - `user` → `order` (via `user_id` in orders)
   - `order` → `payment` (via `order_id` in payments)
   - `user` → `device` (via `user_id` in devices)

2. **Graph Traversal**: The consistency engine can traverse the graph to find related entities:
   ```rust
   // Find all orders for a user
   let orders = engine.find_related_entities(
       workspace_id,
       "user:123",
       "order",
       Some("has_orders")
   ).await;
   ```

3. **Cross-Endpoint Consistency**: When you hit `GET /users/{id}` and `GET /orders?userId={id}`, both endpoints use the same persona graph, ensuring data coherence.

### Example: User-Order-Payment Graph

```
user:123 (Persona)
  ├─ has_orders → order:456
  │                ├─ has_payments → payment:789
  │                └─ has_items → order_item:101
  └─ has_devices → device:202
```

When you request:
- `GET /users/123` → Returns user with related orders embedded
- `GET /users/123/orders` → Returns all orders linked to user:123
- `GET /orders/456` → Returns order with user and payment data embedded

All responses share the same underlying graph state.

### Configuration

Persona graphs are automatically enabled when using the consistency engine. No additional configuration is required.

---

## Lifecycle / Time Awareness

### Persona Lifecycle States

Personas can exist in different lifecycle states that influence their behavior and the data they generate:

- **NewSignup**: Newly created persona, onboarding experience
- **Active**: Normal, active usage
- **ChurnRisk**: Persona showing signs of churn
- **Churned**: Persona that has churned
- **UpgradePending**: Persona with pending upgrade
- **PaymentFailed**: Persona with payment issues
- **PowerUser**: High-engagement persona

### How States Affect Responses

Lifecycle states influence responses across multiple endpoints:

#### Billing Endpoint

```json
// NewSignup state
{
  "billing_status": "trial",
  "subscription_type": "free_tier"
}

// Active state
{
  "billing_status": "paid",
  "subscription_type": "premium"
}

// ChurnRisk state
{
  "billing_status": "active",
  "discount_offer_available": true
}

// Churned state
{
  "billing_status": "cancelled",
  "outstanding_balance": 0.0
}
```

#### Support Endpoint

```json
// NewSignup state
{
  "support_tier": "basic",
  "onboarding_ticket_status": "open"
}

// PowerUser state
{
  "support_tier": "priority",
  "dedicated_support_agent": "true"
}

// ChurnRisk state
{
  "support_tier": "standard",
  "proactive_outreach_ticket": "open"
}
```

### Setting Persona Lifecycle State

#### Via API

```bash
POST /api/v1/consistency/persona/lifecycle
Content-Type: application/json

{
  "workspace_id": "workspace-123",
  "persona_id": "user:456",
  "lifecycle_state": "ChurnRisk"
}
```

#### Via Configuration

```yaml
# config.yaml
consistency:
  personas:
    default_lifecycle_state: "Active"
    lifecycle_states:
      - persona_id: "user:123"
        state: "NewSignup"
      - persona_id: "user:456"
        state: "PowerUser"
```

### Lifecycle State Transitions

Lifecycle states can be updated programmatically:

```rust
// Update persona lifecycle state
persona.update_lifecycle_state(LifecycleState::ChurnRisk);

// Apply lifecycle effects to traits
persona.apply_lifecycle_effects();
```

---

## Reality Continuum Integration

### How Reality Level Affects Persona Data

The reality continuum ratio determines how persona data is generated:

- **0.0 - 0.3 (Low Reality)**: Purely synthetic data generated from persona profiles
- **0.3 - 0.7 (Medium Reality)**: Blended with recorded snapshots from production
- **0.7 - 1.0 (High Reality)**: Blended with real upstream data

### Reality-Aware Generation

When generating persona data, the system considers the reality ratio:

```rust
// Generate with reality awareness
let value = generator.generate_for_persona_with_reality(
    &persona,
    "amount",
    0.5,  // 50% reality (medium)
    Some(&recorded_data),  // Optional recorded snapshot
    Some(&real_data),      // Optional real upstream data
)?;
```

### Blending Strategy

The system uses intelligent blending:

- **Numbers**: Weighted average based on reality ratio
- **Strings/Booleans**: Weighted selection based on reality ratio
- **Objects**: Deep merge with reality-aware field selection

### Configuration

```yaml
# config.yaml
reality_continuum:
  enabled: true
  default_ratio: 0.3  # Start with 30% reality (medium)

consistency:
  personas:
    reality_aware_generation: true
```

---

## Fidelity Score

### What Is Fidelity Score?

Fidelity score quantifies how close your mock environment is to the real upstream. It's a number between 0.0 and 1.0 (or 0-100%) that measures:

- **Schema Compatibility**: How well mock schemas match real schemas
- **Sample Similarity**: How similar mock responses are to real responses (shape + value distributions)
- **Error/Latency Patterns**: How similar error rates and latency distributions are

### Calculating Fidelity Score

The fidelity score is computed using:

1. **Schema Similarity (40% weight)**: Compares field types, structures, and required fields
2. **Sample Similarity (40% weight)**: Compares response shapes and value distributions
3. **Response Time Similarity (10% weight)**: Compares latency distributions
4. **Error Pattern Similarity (10% weight)**: Compares error rates and patterns

### API Endpoints

#### Calculate Fidelity Score

```bash
POST /api/v1/workspace/{workspace_id}/fidelity
Content-Type: application/json

{
  "mock_schema": { ... },
  "real_schema": { ... },
  "mock_samples": [ ... ],
  "real_samples": [ ... ],
  "mock_response_times": [100, 150, 120, ...],
  "real_response_times": [95, 145, 125, ...],
  "mock_error_patterns": {
    "404": 5,
    "500": 2
  },
  "real_error_patterns": {
    "404": 4,
    "500": 3
  }
}
```

#### Get Fidelity Score

```bash
GET /api/v1/workspace/{workspace_id}/fidelity
```

Response:

```json
{
  "success": true,
  "workspace_id": "workspace-123",
  "score": {
    "overall": 0.85,
    "overall_percentage": 85,
    "driver_metrics": {
      "schema_similarity": {
        "value": 0.92,
        "percentage": 92,
        "label": "Schema Match"
      },
      "sample_similarity": {
        "value": 0.88,
        "percentage": 88,
        "label": "Sample Similarity"
      },
      "response_time_similarity": {
        "value": 0.75,
        "percentage": 75,
        "label": "Response Time Match"
      },
      "error_pattern_similarity": {
        "value": 0.80,
        "percentage": 80,
        "label": "Error Pattern Match"
      }
    },
    "computed_at": "2025-01-27T10:30:00Z"
  }
}
```

### UI Component

The fidelity score is displayed in the workspace UI with:

- **Overall Score**: Large percentage display with color coding (green ≥80%, orange ≥60%, red <60%)
- **Driver Metrics**: Breakdown of the 4 driver metrics with progress bars
- **Last Computed**: Timestamp of when the score was last calculated

### Interpreting Fidelity Scores

- **80-100% (High Fidelity)**: Mock closely matches real upstream. Safe for production-like testing.
- **60-79% (Medium Fidelity)**: Mock is reasonably close but has some differences. Good for development.
- **0-59% (Low Fidelity)**: Mock differs significantly from real. May need schema/sample updates.

---

## Integration Example

### Complete Workflow

1. **Create Workspace with Persona Graph**:
   ```yaml
   consistency:
     enabled: true
     persona_graph:
       enabled: true
   ```

2. **Set Initial Persona State**:
   ```bash
   POST /api/v1/consistency/persona/lifecycle
   {
     "workspace_id": "workspace-123",
     "persona_id": "user:456",
     "lifecycle_state": "NewSignup"
   }
   ```

3. **Configure Reality Level**:
   ```yaml
   reality_continuum:
     enabled: true
     default_ratio: 0.5  # 50% reality
   ```

4. **Request User Data**:
   ```bash
   GET /api/v1/consistency/users/456
   # Returns user with lifecycle state and related entities
   ```

5. **Request Related Orders**:
   ```bash
   GET /api/v1/consistency/users/456/orders
   # Returns orders linked via persona graph
   ```

6. **Calculate Fidelity Score**:
   ```bash
   POST /api/v1/workspace/workspace-123/fidelity
   # Returns fidelity score with driver metrics
   ```

---

## Best Practices

1. **Use Persona Graphs for Related Entities**: Link users, orders, payments, etc. through the graph for coherent data
2. **Set Appropriate Lifecycle States**: Use lifecycle states to model realistic user journeys
3. **Gradually Increase Reality**: Start with low reality (0.0-0.3) and gradually increase as you validate
4. **Monitor Fidelity Scores**: Track fidelity scores over time to ensure mocks stay aligned with real upstream
5. **Update Lifecycle States**: Transition personas through lifecycle states to test different scenarios

---

## Troubleshooting

### Persona Graph Not Linking Entities

- Ensure entities are registered through the consistency engine
- Check that entity IDs follow the pattern `{type}:{id}` (e.g., `user:123`)
- Verify relationships are established in the graph

### Lifecycle States Not Affecting Responses

- Ensure lifecycle state is set via API or config
- Check that endpoints use lifecycle-aware response generation
- Verify persona is active in the consistency engine

### Fidelity Score Always Low

- Compare mock and real schemas for structural differences
- Check sample responses for value distribution mismatches
- Review error patterns and latency distributions

---

## Related Documentation

- [Smart Personas](book/src/user-guide/smart-personas.md) - Basic persona features
- [Reality Continuum](book/src/user-guide/reality-continuum.md) - Reality blending features
- [Consistency Engine](../crates/mockforge-core/src/consistency/README.md) - Consistency engine documentation
- [Fidelity Score API](../crates/mockforge-http/src/handlers/fidelity.rs) - Fidelity score API reference
