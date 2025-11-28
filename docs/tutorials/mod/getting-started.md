# MOD Getting Started Tutorial

**Pillars:** [DevX]

**Duration:** 15 minutes
**Prerequisites:** MockForge installed, basic API knowledge

## Overview

This tutorial walks you through creating your first MOD project from scratch. You'll learn how to:

1. Initialize a MOD project
2. Define an API contract
3. Generate a mock from the contract
4. Use the mock in development
5. Validate your implementation

## Step 1: Initialize MOD Project

```bash
# Create project directory
mkdir my-first-mod-project
cd my-first-mod-project

# Initialize MOD project
mockforge mod init

# This creates:
# - mockforge.yaml (configuration)
# - contracts/ (API contracts)
# - mocks/ (mock definitions)
# - scenarios/ (test scenarios)
# - personas/ (persona definitions)
```

## Step 2: Define Your First Contract

Create an API contract for a simple users API:

```bash
# Create contract file
cat > contracts/users-api.yaml <<EOF
openapi: 3.0.0
info:
  title: Users API
  version: 1.0.0
  description: Simple users API for MOD tutorial

paths:
  /api/users:
    get:
      summary: List all users
      responses:
        '200':
          description: List of users
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/User'

    post:
      summary: Create a new user
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreateUserRequest'
      responses:
        '201':
          description: User created
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/User'

  /api/users/{id}:
    get:
      summary: Get user by ID
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
                $ref: '#/components/schemas/User'
        '404':
          description: User not found

components:
  schemas:
    User:
      type: object
      properties:
        id:
          type: string
        name:
          type: string
        email:
          type: string
        created_at:
          type: string
          format: date-time
      required:
        - id
        - name
        - email

    CreateUserRequest:
      type: object
      properties:
        name:
          type: string
        email:
          type: string
      required:
        - name
        - email
EOF
```

## Step 3: Generate Mock from Contract

```bash
# Generate mock from contract
mockforge generate --from-openapi contracts/users-api.yaml --output mocks/

# Start mock server
mockforge serve --config mockforge.yaml --admin
```

The mock server is now running at `http://localhost:3000` with Admin UI at `http://localhost:3000/__mockforge`.

## Step 4: Test Your Mock

```bash
# Test GET /api/users
curl http://localhost:3000/api/users

# Test GET /api/users/{id}
curl http://localhost:3000/api/users/user_123

# Test POST /api/users
curl -X POST http://localhost:3000/api/users \
  -H "Content-Type: application/json" \
  -d '{"name": "Alice", "email": "alice@example.com"}'
```

## Step 5: Use Mock in Development

Create a simple frontend that uses the mock:

```typescript
// src/api/users.ts
const API_BASE = process.env.API_BASE || 'http://localhost:3000';

export async function getUsers() {
  const response = await fetch(`${API_BASE}/api/users`);
  return response.json();
}

export async function getUser(id: string) {
  const response = await fetch(`${API_BASE}/api/users/${id}`);
  if (!response.ok) {
    throw new Error('User not found');
  }
  return response.json();
}

export async function createUser(user: { name: string; email: string }) {
  const response = await fetch(`${API_BASE}/api/users`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(user),
  });
  return response.json();
}
```

## Step 6: Implement Backend

Implement the backend to match the contract:

```rust
// Backend implementation (Rust example)
use axum::{Json, extract::Path};

#[derive(Serialize, Deserialize)]
struct User {
    id: String,
    name: String,
    email: String,
    created_at: String,
}

async fn get_users() -> Json<Vec<User>> {
    // Implementation matches contract
    Json(vec![
        User {
            id: "user_1".to_string(),
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
            created_at: "2025-01-27T00:00:00Z".to_string(),
        },
    ])
}

async fn get_user(Path(id): Path<String>) -> Result<Json<User>, StatusCode> {
    // Implementation matches contract
    Ok(Json(User {
        id: id.clone(),
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
        created_at: "2025-01-27T00:00:00Z".to_string(),
    }))
}
```

## Step 7: Validate Implementation

```bash
# Validate backend against contract
mockforge validate \
  --contract contracts/users-api.yaml \
  --target http://localhost:8080

# Should pass if implementation matches contract
```

## Step 8: Compare Mock vs. Implementation

```bash
# Compare mock and implementation
mockforge mod review \
  --contract contracts/users-api.yaml \
  --mock http://localhost:3000 \
  --implementation http://localhost:8080

# Review differences and fix if needed
```

## Next Steps

- Add more endpoints to your contract
- Use Smart Personas for consistent data
- Create test scenarios
- Explore MOD patterns

## Troubleshooting

### Mock server won't start

**Solution:**
- Check if port 3000 is available
- Verify `mockforge.yaml` configuration
- Check logs for errors

### Contract validation fails

**Solution:**
- Verify contract syntax
- Check OpenAPI spec version
- Ensure implementation matches contract

### Mock responses don't match expectations

**Solution:**
- Customize mock responses
- Use Smart Personas
- Configure reality level

## Further Reading

- [MOD Guide](../../MOD_GUIDE.md) — Complete MOD workflow
- [MOD Patterns](../../MOD_PATTERNS.md) — Common patterns
- [MOD Philosophy](../../MOD_PHILOSOPHY.md) — Core principles

---

**Congratulations! You've completed your first MOD project. Continue exploring MOD patterns and practices.**
