# Behavioral Cloning v1

**Pillars:** [Reality]

[Reality] - Makes mocks feel like real backends through behavioral cloning from recorded traffic

Behavioral Cloning v1 enables MockForge to record multi-step API flows and replay them as named scenarios. This moves from endpoint-level mocks to journey-level simulations, allowing teams to replay realistic flows captured from real systems.

## Overview

Behavioral Cloning v1 provides:

- **Flow Recording**: Capture sequences of requests/responses with timing
- **Flow Viewer**: Visualize flows with timeline and step labels
- **Flow-to-Scenario Compiler**: Convert recorded flows into replayable scenarios
- **Deterministic Replay**: Replay scenarios with state coherence across steps
- **Strict & Flex Modes**: Exact sequence matching or allow minor variations
- **Export/Import**: Share scenarios via YAML/JSON

## Quick Start

### 1. Record a Flow

Start MockForge with recording enabled and proxy mode to capture real traffic:

```bash
# Start MockForge with proxy mode
mockforge serve --proxy --proxy-upstream https://api.example.com

# Or use tunnel mode to capture from a real service
mockforge tunnel start --local-url http://localhost:8080
```

As requests flow through MockForge, they are automatically grouped into flows based on:
- `trace_id` (preferred, if available)
- `session_id` (from cookies/headers)
- Client IP + time window (fallback)

### 2. View Recorded Flows

List all recorded flows:

```bash
mockforge flow list
```

View a specific flow timeline:

```bash
mockforge flow view <flow-id>
```

Example output:

```
Flow: abc-123-def - checkout_success

Timeline (4 steps):

  Step | Label      | Timing  | Request ID
  -----|------------|---------|------------
     1 | login      | -       | req-001
     2 | list       | 150ms   | req-002
     3 | detail     | 200ms   | req-003
     4 | checkout   | 300ms   | req-004
```

### 3. Tag a Flow as a Named Scenario

Tag a flow with a descriptive name:

```bash
mockforge flow tag <flow-id> \
  --name "checkout_success" \
  --tags ecommerce,checkout,success \
  --description "Complete checkout flow with successful payment"
```

### 4. Compile Flow to Scenario

Convert a flow into a replayable behavioral scenario:

```bash
mockforge flow compile <flow-id> \
  --scenario-name "checkout_success" \
  --flex-mode  # or omit for strict mode
```

The compiler will:
- Extract state variables (user_id, cart_id, order_id, etc.) from responses
- Generate step dependencies
- Preserve timing information
- Apply heuristic step labeling

### 5. Use Scenario in CI for E2E Test

Export the scenario:

```bash
mockforge flow export <scenario-id> \
  --output scenarios/checkout_success.yaml \
  --format yaml
```

Import and activate in your test environment:

```bash
# Import scenario
mockforge flow import --input scenarios/checkout_success.yaml

# Start MockForge with scenario activated
mockforge serve \
  --config config.yaml \
  --behavioral-cloning-enabled
```

In your E2E test, make requests in the same sequence. MockForge will replay the scenario responses with state coherence.

## Complete Example: E-Commerce Checkout Flow

### Step 1: Capture from Real Service

Start MockForge in proxy mode to capture real API traffic:

```bash
mockforge serve \
  --proxy \
  --proxy-upstream https://api.ecommerce.example.com \
  --recorder-enabled
```

Or use tunnel mode:

```bash
mockforge tunnel start --local-url http://localhost:8080
```

### Step 2: Execute the Flow

Make requests to your application (which uses the proxied/tunneled endpoint):

```bash
# Login
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "user@example.com", "password": "secret"}'

# List products
curl http://localhost:3000/api/products

# Get product detail
curl http://localhost:3000/api/products/123

# Add to cart
curl -X POST http://localhost:3000/api/cart/items \
  -H "Content-Type: application/json" \
  -d '{"product_id": 123, "quantity": 1}'

# Checkout
curl -X POST http://localhost:3000/api/checkout \
  -H "Content-Type: application/json" \
  -d '{"payment_method": "card", "card_token": "tok_123"}'
```

### Step 3: View and Tag the Flow

List recorded flows:

```bash
mockforge flow list
```

Output:

```
Found 1 flows:

  abc-123-def-456 - (unnamed) (5 steps)
```

View the flow timeline:

```bash
mockforge flow view abc-123-def-456 --verbose
```

Tag it:

```bash
mockforge flow tag abc-123-def-456 \
  --name "checkout_success" \
  --tags ecommerce,checkout \
  --description "Complete checkout flow with successful payment"
```

### Step 4: Compile to Scenario

Compile the flow into a behavioral scenario:

```bash
mockforge flow compile abc-123-def-456 \
  --scenario-name "checkout_success"
```

Output:

```
Compiled flow abc-123-def-456 into scenario 'checkout_success' (ID: scenario-789, Version: 1.0.0)
```

The compiler automatically:
- Extracted `user_id` from login response
- Extracted `cart_id` from cart response
- Extracted `order_id` from checkout response
- Generated step dependencies
- Preserved timing between steps

### Step 5: Export for Sharing

Export the scenario:

```bash
mockforge flow export scenario-789 \
  --output scenarios/checkout_success.yaml \
  --format yaml
```

The exported YAML file contains:

```yaml
id: scenario-789
name: checkout_success
description: Complete checkout flow with successful payment
strict_mode: true
steps:
  - step_id: step_0
    label: login
    request:
      method: POST
      path: /api/auth/login
      # ... request details
    response:
      status_code: 200
      body: '{"user_id": "user-123", "token": "..."}'
    extracts:
      user_id: user_id
    timing_ms: null
  - step_id: step_1
    label: list
    request:
      method: GET
      path: /api/products
    response:
      status_code: 200
      body: '[{"id": 123, "name": "Product"}]'
    timing_ms: 150
  # ... more steps
state_variables:
  user_id:
    name: user_id
    json_path: user_id
    extracted_from_step: step_0
  cart_id:
    name: cart_id
    json_path: cart_id
    extracted_from_step: step_2
```

### Step 6: Use in CI for E2E Test

In your CI pipeline:

```yaml
# .github/workflows/e2e-test.yml
- name: Start MockForge with scenario
  run: |
    mockforge flow import --input scenarios/checkout_success.yaml
    mockforge serve \
      --config config.yaml \
      --behavioral-cloning-enabled &

- name: Run E2E tests
  run: |
    # Your E2E test makes the same sequence of requests
    # MockForge replays the scenario responses
    npm run test:e2e
```

Your E2E test code:

```javascript
// test/checkout.e2e.test.js
describe('Checkout Flow', () => {
  it('should complete checkout successfully', async () => {
    // Login
    const loginRes = await api.post('/api/auth/login', {
      email: 'user@example.com',
      password: 'secret'
    });
    expect(loginRes.status).toBe(200);
    const userId = loginRes.data.user_id;

    // List products
    const productsRes = await api.get('/api/products');
    expect(productsRes.status).toBe(200);

    // Get product detail
    const productRes = await api.get('/api/products/123');
    expect(productRes.status).toBe(200);

    // Add to cart
    const cartRes = await api.post('/api/cart/items', {
      product_id: 123,
      quantity: 1
    });
    expect(cartRes.status).toBe(200);
    const cartId = cartRes.data.cart_id;

    // Checkout
    const checkoutRes = await api.post('/api/checkout', {
      payment_method: 'card',
      card_token: 'tok_123'
    });
    expect(checkoutRes.status).toBe(200);
    expect(checkoutRes.data.order_id).toBeDefined();
  });
});
```

MockForge will replay the exact responses from the recorded flow, maintaining state coherence (user_id, cart_id, order_id) across steps.

## Configuration

Configure behavioral cloning in your `mockforge.yaml`:

```yaml
behavioral_cloning:
  enabled: true
  flow_recording:
    enabled: true
    group_by: trace_id  # trace_id, session_id, or ip_time_window
    time_window_seconds: 300  # for ip_time_window grouping
  scenario_replay:
    enabled: true
    default_mode: strict  # strict or flex
    active_scenarios: []  # scenario IDs to activate on startup
```

Or via environment variables:

```bash
export MOCKFORGE_BEHAVIORAL_CLONING_ENABLED=true
export MOCKFORGE_BEHAVIORAL_CLONING_FLOW_RECORDING_ENABLED=true
export MOCKFORGE_BEHAVIORAL_CLONING_FLOW_RECORDING_GROUP_BY=trace_id
export MOCKFORGE_BEHAVIORAL_CLONING_SCENARIO_REPLAY_ENABLED=true
export MOCKFORGE_BEHAVIORAL_CLONING_SCENARIO_REPLAY_DEFAULT_MODE=strict
```

## CLI Commands

### Flow Management

```bash
# List recorded flows
mockforge flow list [--limit 50]

# View flow timeline
mockforge flow view <flow-id> [--verbose]

# Tag a flow
mockforge flow tag <flow-id> \
  --name "scenario_name" \
  --tags tag1,tag2 \
  --description "Description"

# Compile flow to scenario
mockforge flow compile <flow-id> \
  --scenario-name "name" \
  [--flex-mode]
```

### Scenario Management

```bash
# List scenarios
mockforge flow scenarios [--limit 50]

# Export scenario
mockforge flow export <scenario-id> \
  --output scenario.yaml \
  [--format yaml|json]

# Import scenario
mockforge flow import \
  --input scenario.yaml \
  [--format yaml|json]

# Replay scenario (validate)
mockforge flow replay <scenario-id> [--flex-mode]
```

## Strict vs Flex Mode

### Strict Mode (Default)

- Must match exact sequence of requests
- Must match exact endpoints
- Enforces step order strictly
- Best for deterministic testing

### Flex Mode

- Allows minor variations in sequence
- Allows different IDs in path parameters
- Can skip optional steps
- Best for more flexible testing scenarios

## State Coherence

Scenarios maintain state across steps. Variables extracted from responses are available in subsequent steps:

```yaml
steps:
  - step_id: step_0
    label: login
    extracts:
      user_id: user_id  # Extract from response.user_id
    response:
      body: '{"user_id": "user-123", "token": "..."}'

  - step_id: step_1
    label: get_profile
    request:
      path: /api/users/{{scenario.user_id}}  # Use extracted variable
    response:
      body: '{"user_id": "{{scenario.user_id}}", "name": "John"}'
```

## Heuristic Step Labeling

The compiler automatically labels steps based on path patterns:

- `/login`, `/auth` → "login"
- `/checkout` → "checkout"
- `/payment` → "payment"
- `/cart` → "cart"
- `/order` → "order"
- `GET /users` → "list"
- `GET /users/{id}` → "detail"
- `POST` → "create"
- `PUT`/`PATCH` → "update"
- `DELETE` → "delete"

You can override labels manually when tagging flows.

## Integration with Proxy/Tunnel

Behavioral cloning works seamlessly with MockForge's proxy and tunnel modes:

**Proxy Mode**: Capture flows from proxied requests
```bash
mockforge serve --proxy --proxy-upstream https://api.example.com
```

**Tunnel Mode**: Capture flows through tunnel
```bash
mockforge tunnel start --local-url http://localhost:8080
```

Requests are automatically grouped into flows based on the configured grouping strategy.

## Best Practices

1. **Use trace_id when available**: Enable OpenTelemetry tracing in your application for best flow grouping
2. **Tag flows immediately**: Tag flows with descriptive names right after recording
3. **Export scenarios**: Version control your scenarios alongside your code
4. **Use strict mode for CI**: Strict mode ensures deterministic test results
5. **Use flex mode for development**: Flex mode allows more flexibility during development

## Limitations (v1)

- Deterministic replay only (probabilistic replay planned for v2)
- Path parameter matching is basic (exact match in strict mode)
- State variable extraction uses heuristics (common patterns like id, user_id, etc.)
- Single scenario per session (multiple scenarios planned for v2)

## Next Steps

- **v2 Features**: Probabilistic replay, multiple scenarios per session, advanced path matching
- **Admin UI**: Visual flow viewer with timeline (planned)
- **Marketplace**: Share scenarios via marketplace (planned)
