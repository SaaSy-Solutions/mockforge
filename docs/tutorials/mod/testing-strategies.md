# MOD Testing Strategies

**Pillars:** [DevX][Contracts]

**Duration:** 30 minutes
**Prerequisites:** MOD Getting Started, Testing experience

## Overview

This tutorial demonstrates testing strategies using MOD, from unit tests to end-to-end tests.

## Testing Pyramid with MOD

```
        /\
       /  \  E2E Tests (with mocks)
      /____\
     /      \  Integration Tests (with mocks)
    /________\
   /          \  Unit Tests (with mock responses)
  /____________\
```

## Strategy 1: Unit Testing with Mock Responses

**When:** Testing individual functions/components

**How:** Use mock responses directly in tests

**Example:**

```typescript
// src/services/userService.test.ts
import { getUser } from './userService';

// Mock fetch
global.fetch = jest.fn(() =>
  Promise.resolve({
    ok: true,
    json: async () => ({
      id: 'user_123',
      name: 'Alice',
      email: 'alice@example.com',
    }),
  })
) as jest.Mock;

test('getUser returns user data', async () => {
  const user = await getUser('user_123');
  expect(user.id).toBe('user_123');
  expect(user.name).toBe('Alice');
});
```

## Strategy 2: Integration Testing with Mock Server

**When:** Testing API integration

**How:** Start mock server, test against it

**Example:**

```typescript
// tests/integration/userApi.test.ts
import { startMockServer, stopMockServer } from '../helpers/mockServer';

describe('User API Integration', () => {
  beforeAll(async () => {
    await startMockServer();
  });

  afterAll(async () => {
    await stopMockServer();
  });

  test('GET /api/users returns users', async () => {
    const response = await fetch('http://localhost:3000/api/users');
    const users = await response.json();

    expect(Array.isArray(users)).toBe(true);
    expect(users.length).toBeGreaterThan(0);
  });

  test('GET /api/users/{id} returns user', async () => {
    const response = await fetch('http://localhost:3000/api/users/user_123');
    const user = await response.json();

    expect(user.id).toBe('user_123');
    expect(user.name).toBeDefined();
  });
});
```

## Strategy 3: Contract Testing

**When:** Ensuring implementation matches contract

**How:** Validate implementation against contract

**Example:**

```bash
# Validate implementation
mockforge validate \
  --contract contracts/api.yaml \
  --target http://localhost:8080

# In CI/CD
# .github/workflows/contract-test.yml
name: Contract Tests

on: [pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Validate Contract
        run: |
          mockforge validate \
            --contract contracts/api.yaml \
            --target http://localhost:8080
```

## Strategy 4: Scenario-Based Testing

**When:** Testing user journeys and workflows

**How:** Define scenarios, test against them

**Example:**

```yaml
# scenarios/user-onboarding.yaml
name: User Onboarding Flow
steps:
  - name: Sign Up
    request:
      method: POST
      path: /api/users
      body:
        email: "newuser@example.com"
        password: "password123"
    response:
      status: 201
      body:
        id: "user_new"
        email: "newuser@example.com"

  - name: Verify Email
    request:
      method: POST
      path: /api/users/user_new/verify
      body:
        token: "verification_token"
    response:
      status: 200
      body:
        verified: true

  - name: Complete Profile
    request:
      method: PATCH
      path: /api/users/user_new
      body:
        name: "New User"
    response:
      status: 200
      body:
        id: "user_new"
        name: "New User"
        verified: true
```

```typescript
// tests/scenarios/user-onboarding.test.ts
import { runScenario } from '../helpers/scenarios';

test('user onboarding flow', async () => {
  const result = await runScenario('scenarios/user-onboarding.yaml');
  expect(result.success).toBe(true);
  expect(result.steps).toHaveLength(3);
});
```

## Strategy 5: E2E Testing with Mocks

**When:** Testing complete application flows

**How:** Use mocks for all external services

**Example:**

```typescript
// tests/e2e/checkout.test.ts
import { startMockServer } from '../helpers/mockServer';
import { render, screen, waitFor } from '@testing-library/react';
import { CheckoutPage } from '../../src/pages/CheckoutPage';

describe('Checkout E2E', () => {
  beforeAll(async () => {
    await startMockServer();
  });

  test('complete checkout flow', async () => {
    render(<CheckoutPage />);

    // Add product to cart
    fireEvent.click(screen.getByText('Add to Cart'));

    // Proceed to checkout
    fireEvent.click(screen.getByText('Checkout'));

    // Fill payment form
    fireEvent.change(screen.getByLabelText('Card Number'), {
      target: { value: '4111111111111111' },
    });

    // Submit order
    fireEvent.click(screen.getByText('Place Order'));

    // Verify success
    await waitFor(() => {
      expect(screen.getByText('Order Confirmed')).toBeInTheDocument();
    });
  });
});
```

## Strategy 6: Performance Testing with Mocks

**When:** Testing performance characteristics

**How:** Configure mock latency, test performance

**Example:**

```yaml
# mockforge.yaml
reality:
  level: 3
  latency:
    enabled: true
    base_ms: 100
    jitter_ms: 50
```

```typescript
// tests/performance/api-performance.test.ts
test('API response time is acceptable', async () => {
  const start = Date.now();
  await fetch('http://localhost:3000/api/users');
  const duration = Date.now() - start;

  expect(duration).toBeLessThan(200);  // < 200ms
});
```

## Strategy 7: Error Scenario Testing

**When:** Testing error handling

**How:** Configure mock to return errors

**Example:**

```yaml
# mocks/error-scenarios.yaml
endpoints:
  - path: /api/users/invalid
    method: GET
    response:
      status: 404
      body:
        error: "User not found"
        code: "USER_NOT_FOUND"

  - path: /api/users/error
    method: GET
    response:
      status: 500
      body:
        error: "Internal server error"
        code: "INTERNAL_ERROR"
```

```typescript
// tests/error-handling.test.ts
test('handles 404 error', async () => {
  const response = await fetch('http://localhost:3000/api/users/invalid');
  expect(response.status).toBe(404);

  const error = await response.json();
  expect(error.code).toBe('USER_NOT_FOUND');
});

test('handles 500 error', async () => {
  const response = await fetch('http://localhost:3000/api/users/error');
  expect(response.status).toBe(500);

  const error = await response.json();
  expect(error.code).toBe('INTERNAL_ERROR');
});
```

## Strategy 8: Stateful Testing

**When:** Testing stateful behavior

**How:** Use stateful mocks with session management

**Example:**

```yaml
# mockforge.yaml
reality:
  level: 3
  personas:
    enabled: true
  stateful:
    enabled: true
    session_tracking:
      enabled: true
      method: "cookie"  # or "header", "query"
```

```typescript
// tests/stateful/cart.test.ts
test('cart persists across requests', async () => {
  // Add item to cart
  await fetch('http://localhost:3000/api/cart/items', {
    method: 'POST',
    body: JSON.stringify({ product_id: 'prod_123', quantity: 1 }),
    credentials: 'include',  // Include cookies
  });

  // Get cart
  const response = await fetch('http://localhost:3000/api/cart', {
    credentials: 'include',
  });
  const cart = await response.json();

  expect(cart.items).toHaveLength(1);
  expect(cart.items[0].product_id).toBe('prod_123');
});
```

## Testing Best Practices

### 1. Test with Realistic Data

✅ **Do:**
- Use Smart Personas
- Generate realistic data
- Include edge cases

❌ **Don't:**
- Use placeholder data
- Ignore data relationships
- Skip edge cases

### 2. Test Error Scenarios

✅ **Do:**
- Test 4xx errors
- Test 5xx errors
- Test timeout scenarios

❌ **Don't:**
- Only test happy paths
- Ignore error handling
- Skip edge cases

### 3. Test State Transitions

✅ **Do:**
- Test state changes
- Test lifecycle transitions
- Test session management

❌ **Don't:**
- Test only static state
- Ignore transitions
- Skip stateful behavior

### 4. Validate Contracts

✅ **Do:**
- Validate in CI/CD
- Fail on contract violations
- Check breaking changes

❌ **Don't:**
- Validate only manually
- Skip validation
- Allow contract drift

## Testing Workflow

### 1. Unit Tests

```bash
# Run unit tests
npm test -- --testPathPattern=unit
```

### 2. Integration Tests

```bash
# Start mock server
mockforge serve --config mockforge.yaml

# Run integration tests
npm test -- --testPathPattern=integration
```

### 3. Contract Tests

```bash
# Validate contracts
mockforge validate --contract contracts/
```

### 4. E2E Tests

```bash
# Start mock server
mockforge serve --config mockforge.yaml

# Run E2E tests
npm test -- --testPathPattern=e2e
```

## Further Reading

- [MOD Guide](../../MOD_GUIDE.md) — Complete workflow
- [MOD Patterns](../../MOD_PATTERNS.md) — Testing patterns
- [Contract Testing](../../PROTOCOL_CONTRACTS.md) — Contract validation

---

**MOD enables comprehensive testing at all levels. Test with confidence using realistic mocks.**
