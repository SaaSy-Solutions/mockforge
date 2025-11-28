# Mock-Oriented Development (MOD) Guide

**Pillars:** [DevX][Reality][Contracts]

**Version:** 1.0.0
**Last Updated:** 2025-01-27

A comprehensive guide to implementing Mock-Oriented Development in your projects.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [MOD Workflow](#mod-workflow)
- [Integration with Existing Workflows](#integration-with-existing-workflows)
- [Best Practices](#best-practices)
- [Common Patterns](#common-patterns)
- [Anti-Patterns](#anti-patterns)
- [Troubleshooting](#troubleshooting)

## Overview

MOD (Mock-Oriented Development) is a methodology that places mocks at the center of your development workflow. This guide walks you through implementing MOD step-by-step.

### Prerequisites

- MockForge installed (`cargo install mockforge-cli` or `npm install -g mockforge-cli`)
- Basic understanding of API design (OpenAPI, gRPC, etc.)
- Familiarity with your team's development workflow

## Quick Start

### 1. Initialize a MOD Project

```bash
# Initialize a new MOD project
mockforge mod init my-api-project

# This creates:
# - mockforge.yaml (MOD configuration)
# - contracts/ (API contract definitions)
# - mocks/ (Mock definitions)
# - scenarios/ (Test scenarios)
# - personas/ (Persona definitions)
```

### 2. Define Your First Contract

```yaml
# contracts/users-api.yaml
openapi: 3.0.0
info:
  title: Users API
  version: 1.0.0
paths:
  /api/users/{id}:
    get:
      parameters:
        - name: id
          in: path
          required: true
          schema:
            type: string
      responses:
        '200':
          description: User details
          content:
            application/json:
              schema:
                type: object
                properties:
                  id:
                    type: string
                  name:
                    type: string
                  email:
                    type: string
```

### 3. Generate Mock from Contract

```bash
# Generate mock from OpenAPI contract
mockforge generate --from-openapi contracts/users-api.yaml --output mocks/

# Start mock server
mockforge serve --config mockforge.yaml
```

### 4. Use Mock in Development

```typescript
// Frontend code
const API_BASE = process.env.API_BASE || 'http://localhost:3000';

async function getUser(id: string) {
  const response = await fetch(`${API_BASE}/api/users/${id}`);
  return response.json();
}
```

### 5. Implement Backend to Match Contract

```rust
// Backend implementation
#[get("/api/users/{id}")]
async fn get_user(id: Path<String>) -> Json<User> {
    // Implementation matches contract
    Json(User { id, name: "...", email: "..." })
}
```

### 6. Validate Implementation

```bash
# Validate backend against contract
mockforge validate --contract contracts/users-api.yaml --target http://localhost:8080
```

## MOD Workflow

### Phase 1: Design & Contract Definition

**Goal:** Define API contracts before implementation

**Steps:**

1. **Design API** — Work with stakeholders to design API
2. **Write Contract** — Create OpenAPI/gRPC contract
3. **Generate Mock** — Generate mock from contract
4. **Review Mock** — Review mock responses with team
5. **Iterate** — Refine contract based on feedback

**Tools:**
- OpenAPI Editor
- `mockforge generate`
- MockForge Admin UI

**Deliverables:**
- API contract (OpenAPI/gRPC)
- Mock server
- Mock responses

### Phase 2: Parallel Development

**Goal:** Enable parallel frontend/backend development

**Steps:**

1. **Frontend Starts** — Use mock for development
2. **Backend Starts** — Implement to match contract
3. **Continuous Validation** — Validate backend against contract
4. **Integration Testing** — Test integration with mocks

**Tools:**
- MockForge mock server
- Contract validation
- Integration test framework

**Deliverables:**
- Frontend implementation
- Backend implementation
- Integration tests

### Phase 3: Integration & Validation

**Goal:** Validate implementation matches contract

**Steps:**

1. **Contract Validation** — Validate backend against contract
2. **Mock Comparison** — Compare mock vs. implementation
3. **Integration Testing** — Run integration tests
4. **Fix Discrepancies** — Fix any contract violations

**Tools:**
- `mockforge validate`
- Contract testing tools
- Integration test suite

**Deliverables:**
- Validated implementation
- Integration test results
- Contract compliance report

### Phase 4: Review & Approval

**Goal:** Ensure API changes are reviewed

**Steps:**

1. **PR Creation** — Create PR with contract + implementation
2. **Contract Review** — Review contract changes
3. **Mock Comparison** — Compare mock vs. implementation
4. **Approval** — Approve if contract matches

**Tools:**
- `mockforge mod review`
- PR review tools
- Contract diff tools

**Deliverables:**
- Reviewed PR
- Approved changes
- Updated documentation

## Integration with Existing Workflows

### Git Workflow

**MOD integrates with Git:**

```bash
# Pre-commit hook validates contracts
.git/hooks/pre-commit:
  mockforge validate --contract contracts/

# CI validates contracts on PR
.github/workflows/ci.yml:
  - name: Validate Contracts
    run: mockforge validate --contract contracts/
```

### CI/CD Pipeline

**MOD in CI/CD:**

```yaml
# .github/workflows/mod.yml
name: MOD Validation

on: [pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Validate Contracts
        run: mockforge validate --contract contracts/
      - name: Run Tests with Mocks
        run: npm test -- --mock-server=http://localhost:3000
```

### API Review Process

**MOD in API reviews:**

1. **Contract First** — Review contract before implementation
2. **Mock Preview** — Preview mock responses
3. **Implementation Review** — Review implementation against contract
4. **Validation** — Automatically validate contract compliance

See [MOD_API_REVIEW.md](MOD_API_REVIEW.md) for detailed API review process.

## Best Practices

### 1. Contract as Source of Truth

✅ **Do:**
- Define contracts first
- Generate mocks from contracts
- Validate implementations against contracts

❌ **Don't:**
- Write contracts after implementation
- Manually maintain mocks
- Ignore contract validation

### 2. Version Contracts Explicitly

✅ **Do:**
- Version contracts (v1, v2, etc.)
- Use semantic versioning
- Document breaking changes

❌ **Don't:**
- Make breaking changes without versioning
- Mix contract versions
- Forget to update version numbers

### 3. Realistic Mock Data

✅ **Do:**
- Use Smart Personas for consistency
- Generate realistic data
- Include edge cases

❌ **Don't:**
- Use placeholder data
- Ignore data relationships
- Skip error scenarios

### 4. Scenario-Based Testing

✅ **Do:**
- Organize tests by scenarios
- Test user journeys
- Include error paths

❌ **Don't:**
- Test endpoints in isolation
- Ignore state transitions
- Skip edge cases

### 5. Continuous Validation

✅ **Do:**
- Validate in CI/CD
- Run validation on every PR
- Fail builds on contract violations

❌ **Don't:**
- Validate only manually
- Skip validation in CI
- Allow contract drift

## Common Patterns

### Pattern 1: Contract-First API Design

**Problem:** API design happens during implementation, causing rework.

**Solution:** Define contract first, generate mock, iterate on design.

**Example:**

```bash
# 1. Define contract
cat > contracts/api.yaml <<EOF
openapi: 3.0.0
paths:
  /api/users:
    get:
      responses:
        '200':
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/User'
EOF

# 2. Generate mock
mockforge generate --from-openapi contracts/api.yaml

# 3. Review mock responses
mockforge serve --admin

# 4. Iterate on contract based on feedback
```

### Pattern 2: Frontend-Backend Parallel Development

**Problem:** Frontend blocked waiting for backend.

**Solution:** Frontend uses mock, backend implements contract.

**Example:**

```typescript
// Frontend uses mock
const API_URL = process.env.MOCK_MODE
  ? 'http://localhost:3000'  // Mock server
  : 'https://api.example.com'; // Real API

// Backend implements contract
#[get("/api/users")]
async fn get_users() -> Json<Vec<User>> {
    // Implementation matches contract
}
```

### Pattern 3: Contract-Driven Testing

**Problem:** Tests break when implementation changes.

**Solution:** Tests validate against contract, not implementation.

**Example:**

```typescript
// Test validates contract
test('GET /api/users returns valid schema', async () => {
  const response = await fetch('http://localhost:3000/api/users');
  const data = await response.json();

  // Validate against OpenAPI schema
  validateAgainstSchema(data, userListSchema);
});
```

### Pattern 4: Gradual Reality Progression

**Problem:** Mocks too simple or too complex.

**Solution:** Start simple, increase realism over time.

**Example:**

```yaml
# Phase 1: Static mocks
reality:
  level: 1  # Static stubs

# Phase 2: Dynamic data
reality:
  level: 2  # Light simulation
  personas:
    enabled: true

# Phase 3: Realistic behavior
reality:
  level: 3  # Moderate realism
  personas:
    enabled: true
  latency:
    enabled: true
```

### Pattern 5: Scenario-Based Development

**Problem:** Testing individual endpoints misses integration issues.

**Solution:** Organize development around scenarios.

**Example:**

```yaml
# scenarios/user-signup.yaml
name: User Signup Flow
steps:
  - name: Create Account
    request:
      method: POST
      path: /api/users
      body:
        email: "user@example.com"
    response:
      status: 201
      body:
        id: "user_123"
        email: "user@example.com"

  - name: Verify Email
    request:
      method: POST
      path: /api/users/user_123/verify
    response:
      status: 200
```

## Anti-Patterns

### ❌ Anti-Pattern 1: Mock After Implementation

**Problem:** Mocks created after backend is done.

**Why it's bad:**
- Frontend blocked during development
- No early design validation
- Contract drift likely

**Solution:** Create mocks first, before implementation.

### ❌ Anti-Pattern 2: Ignoring Contract Validation

**Problem:** No validation that implementation matches contract.

**Why it's bad:**
- Contract drift goes undetected
- Breaking changes not caught
- Integration issues in production

**Solution:** Validate contracts in CI/CD.

### ❌ Anti-Pattern 3: Overly Complex Mocks

**Problem:** Mocks try to replicate entire backend.

**Why it's bad:**
- Hard to maintain
- Slow to develop
- Defeats purpose of mocks

**Solution:** Start simple, increase realism gradually.

### ❌ Anti-Pattern 4: Mock Data Doesn't Match Real Data

**Problem:** Mock responses don't reflect real API behavior.

**Why it's bad:**
- Frontend works with mock, breaks with real API
- Integration surprises
- Wasted development time

**Solution:** Use Reality Continuum to blend mock and real data.

### ❌ Anti-Pattern 5: Mocks Not Versioned

**Problem:** Mocks don't track API versions.

**Why it's bad:**
- Breaking changes not tracked
- Version confusion
- Integration issues

**Solution:** Version contracts and mocks explicitly.

## Troubleshooting

### Problem: Mock responses don't match real API

**Solution:**
- Use Reality Continuum to blend mock and real data
- Record real API responses
- Update mock based on real behavior

### Problem: Contract validation fails

**Solution:**
- Check contract syntax
- Validate against OpenAPI/gRPC spec
- Ensure implementation matches contract

### Problem: Frontend works with mock, breaks with real API

**Solution:**
- Increase mock realism (reality level)
- Use Smart Personas for consistency
- Test with Reality Continuum

### Problem: Mock server too slow

**Solution:**
- Reduce reality level
- Disable unnecessary features
- Use static responses for simple cases

## Next Steps

- [MOD Patterns](MOD_PATTERNS.md) — Detailed patterns and examples
- [MOD Folder Structures](MOD_FOLDER_STRUCTURES.md) — Project organization
- [MOD API Review](MOD_API_REVIEW.md) — Review process
- [MOD Tutorials](tutorials/mod/) — Step-by-step tutorials

---

**Ready to embrace MOD? Start with `mockforge mod init` and begin your mock-first journey.**
