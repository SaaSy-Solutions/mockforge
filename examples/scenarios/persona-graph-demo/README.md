# Smart Personas & Reality Continuum v2 Demo

This scenario demonstrates MockForge's Smart Personas v2 features, showcasing how persona graphs, lifecycle states, and reality continuum work together to create a coherent world simulation.

## Features Demonstrated

### 1. Persona Graphs & Relationships

Persona graphs link entities across different types, ensuring coherent data across all endpoints:

- **User → Orders**: Users are linked to their orders via the persona graph
- **Order → Payments**: Orders are linked to their payments
- **User → Devices**: Users are linked to their devices

When you request a user, their related orders, payments, and devices are all connected through the graph.

### 2. Lifecycle States

Personas can exist in different lifecycle states that influence their behavior:

- **NewSignup**: Newly created persona, onboarding experience
- **Active**: Normal, active usage
- **ChurnRisk**: Persona showing signs of churn
- **Churned**: Persona that has churned
- **UpgradePending**: Persona with pending upgrade
- **PaymentFailed**: Persona with payment issues
- **PowerUser**: High-engagement persona

Lifecycle states affect responses across multiple endpoints (billing, support, etc.).

### 3. Reality Continuum Integration

The reality continuum ratio determines how persona data is generated:

- **0.0 - 0.3 (Low Reality)**: Purely synthetic data
- **0.3 - 0.7 (Medium Reality)**: Blended with recorded snapshots
- **0.7 - 1.0 (High Reality)**: Blended with real upstream data

### 4. Fidelity Score

Track how close your mock environment is to the real upstream with fidelity scores that measure schema compatibility, sample similarity, and error/latency patterns.

## API Endpoints

### Persona Graph Endpoints

#### Get User with Graph Data
```bash
GET /api/v1/consistency/users/{id}
```

Returns user data enriched with related entities from the persona graph (orders, devices, etc.).

**Example:**
```bash
curl http://localhost:3000/api/v1/consistency/users/123
```

**Response:**
```json
{
  "id": "123",
  "name": "Alice",
  "email": "alice@example.com",
  "orders": [
    {
      "id": "456",
      "user_id": "123",
      "total": 99.99,
      "status": "completed"
    }
  ],
  "lifecycle_state": "Active"
}
```

#### Get User Orders via Graph
```bash
GET /api/v1/consistency/users/{id}/orders
```

Returns all orders linked to a user through the persona graph.

**Example:**
```bash
curl http://localhost:3000/api/v1/consistency/users/123/orders
```

#### Get Order with Graph Data
```bash
GET /api/v1/consistency/orders/{id}
```

Returns order data enriched with related entities (user, payments).

**Example:**
```bash
curl http://localhost:3000/api/v1/consistency/orders/456
```

**Response:**
```json
{
  "id": "456",
  "user_id": "123",
  "user": {
    "id": "123",
    "name": "Alice",
    "email": "alice@example.com"
  },
  "total": 99.99,
  "status": "completed",
  "payments": [
    {
      "id": "789",
      "order_id": "456",
      "amount": 99.99,
      "status": "completed"
    }
  ]
}
```

### Lifecycle Management

#### Set Persona Lifecycle State
```bash
POST /api/v1/consistency/persona/lifecycle
Content-Type: application/json

{
  "workspace_id": "default",
  "persona_id": "user:123",
  "lifecycle_state": "ChurnRisk"
}
```

**Example:**
```bash
curl -X POST http://localhost:3000/api/v1/consistency/persona/lifecycle \
  -H "Content-Type: application/json" \
  -d '{
    "workspace_id": "default",
    "persona_id": "user:123",
    "lifecycle_state": "ChurnRisk"
  }'
```

After setting the lifecycle state, subsequent requests will reflect the new state in billing and support endpoints.

### Fidelity Score

#### Get Fidelity Score
```bash
GET /api/v1/workspace/{workspace_id}/fidelity
```

Returns the current fidelity score for the workspace.

**Example:**
```bash
curl http://localhost:3000/api/v1/workspace/default/fidelity
```

**Response:**
```json
{
  "success": true,
  "workspace_id": "default",
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

#### Calculate Fidelity Score
```bash
POST /api/v1/workspace/{workspace_id}/fidelity
Content-Type: application/json

{
  "mock_schema": { ... },
  "real_schema": { ... },
  "mock_samples": [ ... ],
  "real_samples": [ ... ],
  "mock_response_times": [100, 150, 120],
  "real_response_times": [95, 145, 125],
  "mock_error_patterns": { "404": 5, "500": 2 },
  "real_error_patterns": { "404": 4, "500": 3 }
}
```

## Usage Examples

### 1. Explore Persona Graph

```bash
# Get a user
curl http://localhost:3000/api/v1/consistency/users/123

# Get their orders (linked via graph)
curl http://localhost:3000/api/v1/consistency/users/123/orders

# Get an order with related data
curl http://localhost:3000/api/v1/consistency/orders/456
```

### 2. Test Lifecycle States

```bash
# Set user to NewSignup state
curl -X POST http://localhost:3000/api/v1/consistency/persona/lifecycle \
  -H "Content-Type: application/json" \
  -d '{"workspace_id": "default", "persona_id": "user:123", "lifecycle_state": "NewSignup"}'

# Request user data - billing status will reflect "trial"
curl http://localhost:3000/api/v1/consistency/users/123

# Set to ChurnRisk
curl -X POST http://localhost:3000/api/v1/consistency/persona/lifecycle \
  -H "Content-Type: application/json" \
  -d '{"workspace_id": "default", "persona_id": "user:123", "lifecycle_state": "ChurnRisk"}'

# Request user data - will show discount offers and proactive outreach
curl http://localhost:3000/api/v1/consistency/users/123
```

### 3. Monitor Fidelity

```bash
# Get current fidelity score
curl http://localhost:3000/api/v1/workspace/default/fidelity

# Calculate new fidelity score (requires mock and real data)
curl -X POST http://localhost:3000/api/v1/workspace/default/fidelity \
  -H "Content-Type: application/json" \
  -d @fidelity-request.json
```

## Configuration

The scenario is pre-configured with:

- **Consistency Engine**: Enabled with persona graphs
- **Reality Continuum**: Set to 30% reality (medium)
- **Default Lifecycle State**: Active

You can modify these settings in `config.yaml`:

```yaml
consistency:
  enabled: true
  persona_graph:
    enabled: true
  personas:
    default_lifecycle_state: "Active"  # Change default state

reality_continuum:
  enabled: true
  default_ratio: 0.3  # Adjust reality level (0.0-1.0)
```

## Best Practices

1. **Use Consistent Entity IDs**: Use the same persona ID across related endpoints (e.g., `user:123` for user, `order:456` for orders)

2. **Set Appropriate Lifecycle States**: Use lifecycle states to model realistic user journeys

3. **Gradually Increase Reality**: Start with low reality (0.0-0.3) and gradually increase as you validate

4. **Monitor Fidelity Scores**: Track fidelity scores over time to ensure mocks stay aligned with real upstream

5. **Leverage Graph Traversal**: Use graph endpoints to explore relationships between entities

## Related Documentation

- [Smart Personas & Reality Continuum v2](../../../docs/PERSONAS.md) - Complete documentation
- [Reality Continuum Guide](../../../book/src/user-guide/reality-continuum.md) - Reality blending features
- [Consistency Engine](../../../crates/mockforge-core/src/consistency/README.md) - Consistency engine docs

## Support

For questions or issues, please visit:
- GitHub Issues: https://github.com/mockforge/mockforge/issues
- Documentation: https://docs.mockforge.dev
