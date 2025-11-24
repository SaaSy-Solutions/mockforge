# MOD Patterns Library

**Pillars:** [DevX][Reality][Contracts]

**Version:** 1.0.0
**Last Updated:** 2025-01-27

A comprehensive library of MOD (Mock-Oriented Development) patterns, organized by use case and complexity.

## Table of Contents

- [Core Patterns](#core-patterns)
- [Data Patterns](#data-patterns)
- [State Patterns](#state-patterns)
- [Integration Patterns](#integration-patterns)
- [Testing Patterns](#testing-patterns)
- [Advanced Patterns](#advanced-patterns)
- [Anti-Patterns](#anti-patterns)

## Core Patterns

### Pattern: Contract-First Design

**When to use:** Starting a new API or redesigning an existing one.

**How it works:**
1. Define API contract (OpenAPI, gRPC, etc.)
2. Generate mock from contract
3. Review mock responses with team
4. Iterate on contract based on feedback
5. Implement backend to match contract

**Example:**

```yaml
# Step 1: Define contract
# contracts/users-api.yaml
openapi: 3.0.0
paths:
  /api/users/{id}:
    get:
      responses:
        '200':
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/User'

# Step 2: Generate mock
mockforge generate --from-openapi contracts/users-api.yaml

# Step 3: Review and iterate
# Use MockForge Admin UI to review responses

# Step 4: Implement backend
# Backend code matches contract exactly
```

**Benefits:**
- Early design validation
- Frontend can start immediately
- Clear contract definition
- Reduced rework

### Pattern: Persona-Driven Development

**When to use:** Need consistent data across endpoints.

**How it works:**
1. Define personas (user types, behaviors)
2. Configure personas in mock
3. Use personas for data generation
4. Ensure consistency across endpoints

**Example:**

```yaml
# mockforge.yaml
reality:
  personas:
    enabled: true
    personas:
      - name: "premium_user"
        domain: "ecommerce"
        traits:
          spending_level: "high"
          account_type: "premium"
          loyalty_level: "gold"

# All endpoints use same persona for consistency
# GET /api/users/123 → premium_user data
# GET /api/orders?user_id=123 → premium_user's orders
```

**Benefits:**
- Consistent data across endpoints
- Realistic user journeys
- Better testing scenarios
- Easier debugging

### Pattern: Reality Progression

**When to use:** Gradually increasing mock complexity.

**How it works:**
1. Start with simple static mocks (level 1)
2. Add dynamic data generation (level 2)
3. Add stateful behavior (level 3)
4. Add latency and chaos (level 4)
5. Full production simulation (level 5)

**Example:**

```yaml
# Phase 1: Simple static mocks
reality:
  level: 1  # Static stubs

# Phase 2: Dynamic data
reality:
  level: 2  # Light simulation
  personas:
    enabled: true

# Phase 3: Stateful behavior
reality:
  level: 3  # Moderate realism
  personas:
    enabled: true
  latency:
    enabled: true

# Phase 4: Realistic behavior
reality:
  level: 4  # High realism
  chaos:
    enabled: true
  latency:
    enabled: true
```

**Benefits:**
- Gradual complexity
- Early confidence
- Realistic testing
- Production readiness

### Pattern: Scenario-Based Development

**When to use:** Building complex workflows or user journeys.

**How it works:**
1. Define scenarios (user journeys, workflows)
2. Create mock scenarios
3. Develop against scenarios
4. Test scenarios end-to-end

**Example:**

```yaml
# scenarios/ecommerce-checkout.yaml
name: E-commerce Checkout Flow
description: Complete checkout process from cart to payment

steps:
  - name: Add to Cart
    request:
      method: POST
      path: /api/cart/items
      body:
        product_id: "prod_123"
        quantity: 2
    response:
      status: 201
      body:
        cart_id: "cart_456"
        items:
          - product_id: "prod_123"
            quantity: 2

  - name: Checkout
    request:
      method: POST
      path: /api/cart/cart_456/checkout
      body:
        payment_method: "card"
    response:
      status: 200
      body:
        order_id: "order_789"
        status: "processing"

  - name: Confirm Payment
    request:
      method: POST
      path: /api/orders/order_789/confirm
    response:
      status: 200
      body:
        order_id: "order_789"
        status: "confirmed"
```

**Benefits:**
- End-to-end thinking
- Realistic testing
- Better documentation
- Reusable scenarios

## Data Patterns

### Pattern: Generative Schema Mode

**When to use:** Need to generate API from example data.

**How it works:**
1. Provide example JSON payloads
2. MockForge infers schema and routes
3. Generate complete API ecosystem
4. Use generated API for development

**Example:**

```bash
# Provide example data
cat > examples.json <<EOF
{
  "users": [
    {"id": 1, "name": "Alice", "email": "alice@example.com"},
    {"id": 2, "name": "Bob", "email": "bob@example.com"}
  ],
  "orders": [
    {"id": 1, "user_id": 1, "total": 99.99},
    {"id": 2, "user_id": 1, "total": 149.99}
  ]
}
EOF

# Generate API ecosystem
mockforge generate --from-json examples.json --output ./generated-api

# Generated API includes:
# - GET /users (list all users)
# - GET /users/{id} (get user by ID)
# - POST /users (create user)
# - GET /orders (list all orders)
# - GET /orders/{id} (get order by ID)
# - Relationships: users → orders
```

**Benefits:**
- Fast API prototyping
- No manual schema writing
- Automatic CRUD routes
- Relationship inference

### Pattern: Reality Continuum

**When to use:** Gradually transitioning from mock to real API.

**How it works:**
1. Start with 100% mock
2. Gradually blend with real API
3. Increase real API percentage
4. Eventually use 100% real API

**Example:**

```yaml
# Phase 1: 100% mock
reality:
  continuum:
    enabled: true
    blend_ratio: 0.0  # 100% mock

# Phase 2: 50% mock, 50% real
reality:
  continuum:
    enabled: true
    blend_ratio: 0.5  # 50% mock, 50% real
    upstream_url: https://api.example.com

# Phase 3: 100% real
reality:
  continuum:
    enabled: true
    blend_ratio: 1.0  # 100% real
```

**Benefits:**
- Smooth transition
- Early integration
- Gradual migration
- Risk reduction

### Pattern: Smart Personas

**When to use:** Need consistent, relationship-aware data.

**How it works:**
1. Define personas with traits
2. Configure persona relationships
3. Use personas for data generation
4. Maintain consistency across endpoints

**Example:**

```yaml
# Define persona
personas:
  - id: "user_123"
    domain: "ecommerce"
    traits:
      spending_level: "high"
      account_type: "premium"
    relationships:
      has_orders: ["order_456", "order_789"]
      has_devices: ["device_abc"]

# Persona ensures consistency:
# - GET /api/users/user_123 → premium user data
# - GET /api/orders?user_id=user_123 → high-value orders
# - GET /api/devices?user_id=user_123 → premium devices
```

**Benefits:**
- Consistent data
- Relationship awareness
- Realistic scenarios
- Easier debugging

## State Patterns

### Pattern: Lifecycle-Aware Mocks

**When to use:** Entities have lifecycle states (user signup, order fulfillment, etc.).

**How it works:**
1. Define lifecycle states
2. Configure state transitions
3. Mock responses vary by state
4. Test state transitions

**Example:**

```yaml
# Define lifecycle
lifecycle:
  presets:
    - name: "user_engagement"
      type: "user"
      states:
        - name: "new_signup"
          transitions:
            - to: "active"
              after_days: 0
        - name: "active"
          transitions:
            - to: "churn_risk"
              after_days: 30
              condition: "activity_count < 5"
        - name: "churn_risk"
          transitions:
            - to: "churned"
              after_days: 60

# Mock responses vary by state
# GET /api/users/user_123 → returns data based on current lifecycle state
```

**Benefits:**
- Realistic state transitions
- Time-aware testing
- Better scenarios
- Production-like behavior

### Pattern: Time Travel Testing

**When to use:** Testing time-dependent behavior.

**How it works:**
1. Enable time travel
2. Set initial time
3. Advance time as needed
4. Test time-dependent behavior

**Example:**

```yaml
# Enable time travel
core:
  time_travel:
    enabled: true
    initial_time: "2025-01-01T00:00:00Z"

# Test time-dependent behavior
# - Token expiration
# - Session timeouts
# - Scheduled events
# - Lifecycle transitions
```

**Benefits:**
- Fast time-based testing
- No waiting for real time
- Deterministic tests
- Better debugging

## Integration Patterns

### Pattern: Multi-Protocol Coordination

**When to use:** System uses multiple protocols (HTTP, gRPC, WebSocket, etc.).

**How it works:**
1. Define contracts for each protocol
2. Use unified state across protocols
3. Coordinate responses across protocols
4. Test multi-protocol scenarios

**Example:**

```yaml
# Unified state across protocols
workspaces:
  - name: "ecommerce"
    reality:
      level: 3
    endpoints:
      # HTTP endpoints
      - path: /api/users/{id}
        method: GET
      # gRPC services
      - service: UserService
        method: GetUser
      # WebSocket events
      - path: /ws/notifications
        events:
          - type: "user_updated"
```

**Benefits:**
- Consistent state
- Realistic multi-protocol testing
- Better integration
- Unified testing

### Pattern: Contract-Driven Integration

**When to use:** Integrating with external services.

**How it works:**
1. Define contract for external service
2. Create mock of external service
3. Develop against mock
4. Validate integration with real service

**Example:**

```yaml
# Mock external payment service
endpoints:
  - path: /api/payments/process
    method: POST
    response:
      body:
        transaction_id: "txn_123"
        status: "success"
        amount: 99.99

# Use mock for development
# Switch to real service for production
```

**Benefits:**
- Independent development
- Safe testing
- No external dependencies
- Faster development

## Testing Patterns

### Pattern: Mock-First Testing

**When to use:** Writing integration or E2E tests.

**How it works:**
1. Define test scenarios
2. Create mock responses
3. Run tests against mocks
4. Validate test results

**Example:**

```typescript
// Test uses mock
describe('User API', () => {
  beforeAll(async () => {
    // Start mock server
    await mockServer.start();
  });

  it('should get user by ID', async () => {
    const response = await fetch('http://localhost:3000/api/users/123');
    const user = await response.json();

    expect(user.id).toBe('123');
    expect(user.name).toBeDefined();
  });
});
```

**Benefits:**
- Fast tests
- Deterministic results
- No external dependencies
- Better isolation

### Pattern: Contract Testing

**When to use:** Ensuring API implementations match contracts.

**How it works:**
1. Define contract
2. Generate mock from contract
3. Validate implementation against contract
4. Fail if contract violated

**Example:**

```bash
# Validate implementation
mockforge validate \
  --contract contracts/api.yaml \
  --target http://localhost:8080

# Fails if:
# - Response doesn't match schema
# - Status code incorrect
# - Required fields missing
```

**Benefits:**
- Automatic validation
- Contract compliance
- Breaking change detection
- Better quality

## Advanced Patterns

### Pattern: Behavioral Cloning

**When to use:** Need mocks that behave like real systems.

**How it works:**
1. Record real API interactions
2. Analyze behavior patterns
3. Generate mock from patterns
4. Use mock for testing

**Example:**

```bash
# Record real API
mockforge record --target https://api.example.com --output recordings/

# Analyze patterns
mockforge analyze --recordings recordings/

# Generate mock from patterns
mockforge generate --from-recordings recordings/ --output mocks/
```

**Benefits:**
- Realistic behavior
- Production-like mocks
- Better testing
- Easier migration

### Pattern: Chaos-Driven Development

**When to use:** Testing resilience and failure scenarios.

**How it works:**
1. Define chaos scenarios
2. Configure failure injection
3. Test failure handling
4. Validate resilience

**Example:**

```yaml
# Configure chaos
chaos:
  enabled: true
  rules:
    - name: "random_500s"
      type: "status_code"
      probability: 0.1
      status: 500
    - name: "latency_spike"
      type: "latency"
      probability: 0.2
      delay_ms: 5000
```

**Benefits:**
- Resilience testing
- Failure scenario coverage
- Better error handling
- Production readiness

## Anti-Patterns

### ❌ Anti-Pattern: Mock After Implementation

**Problem:** Creating mocks after backend is implemented.

**Why it's bad:**
- Frontend blocked during development
- No early design validation
- Contract drift likely

**Solution:** Create mocks first, before implementation.

### ❌ Anti-Pattern: Ignoring Contract Validation

**Problem:** No validation that implementation matches contract.

**Why it's bad:**
- Contract drift goes undetected
- Breaking changes not caught
- Integration issues

**Solution:** Validate contracts in CI/CD.

### ❌ Anti-Pattern: Overly Complex Mocks

**Problem:** Mocks try to replicate entire backend.

**Why it's bad:**
- Hard to maintain
- Slow to develop
- Defeats purpose

**Solution:** Start simple, increase realism gradually.

### ❌ Anti-Pattern: Mock Data Doesn't Match Real

**Problem:** Mock responses don't reflect real API.

**Why it's bad:**
- Frontend works with mock, breaks with real
- Integration surprises
- Wasted time

**Solution:** Use Reality Continuum to blend mock and real.

## Pattern Selection Guide

| Use Case | Recommended Pattern |
|----------|-------------------|
| New API design | Contract-First Design |
| Consistent data | Persona-Driven Development |
| Complex workflows | Scenario-Based Development |
| Time-dependent behavior | Time Travel Testing |
| Multi-protocol system | Multi-Protocol Coordination |
| External integration | Contract-Driven Integration |
| Resilience testing | Chaos-Driven Development |
| Realistic behavior | Behavioral Cloning |

## Further Reading

- [MOD Philosophy](MOD_PHILOSOPHY.md) — Core MOD principles
- [MOD Guide](MOD_GUIDE.md) — Step-by-step workflow
- [MOD Folder Structures](MOD_FOLDER_STRUCTURES.md) — Project organization
- [MOD Tutorials](tutorials/mod/) — Hands-on tutorials

---

**Patterns are tools—use them wisely. Start simple, iterate, and increase complexity as needed.**
