# MOD API-First Tutorial

**Pillars:** [DevX][Contracts]

**Duration:** 30 minutes
**Prerequisites:** MOD Getting Started tutorial, OpenAPI knowledge

## Overview

This tutorial demonstrates building an API using the API-First approach with MOD. You'll learn how to:

1. Design API using contracts
2. Generate mocks for early feedback
3. Iterate on API design
4. Implement backend to match contract
5. Validate and deploy

## Scenario: E-commerce API

We'll build a simple e-commerce API with users, products, and orders.

## Step 1: Design API Contract

Create the main API contract:

```yaml
# contracts/ecommerce-api.yaml
openapi: 3.0.0
info:
  title: E-commerce API
  version: 1.0.0
  description: E-commerce API for products, users, and orders

paths:
  /api/users:
    get:
      summary: List users
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
      summary: Create user
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

  /api/products:
    get:
      summary: List products
      parameters:
        - name: category
          in: query
          schema:
            type: string
      responses:
        '200':
          description: List of products
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/Product'

  /api/orders:
    post:
      summary: Create order
      requestBody:
        required: true
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreateOrderRequest'
      responses:
        '201':
          description: Order created
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Order'

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

    Product:
      type: object
      properties:
        id:
          type: string
        name:
          type: string
        price:
          type: number
        category:
          type: string
      required:
        - id
        - name
        - price

    Order:
      type: object
      properties:
        id:
          type: string
        user_id:
          type: string
        items:
          type: array
          items:
            $ref: '#/components/schemas/OrderItem'
        total:
          type: number
        status:
          type: string
          enum: [pending, processing, completed, cancelled]
        created_at:
          type: string
          format: date-time
      required:
        - id
        - user_id
        - items
        - total
        - status

    OrderItem:
      type: object
      properties:
        product_id:
          type: string
        quantity:
          type: integer
        price:
          type: number
      required:
        - product_id
        - quantity
        - price

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

    CreateOrderRequest:
      type: object
      properties:
        user_id:
          type: string
        items:
          type: array
          items:
            $ref: '#/components/schemas/OrderItem'
      required:
        - user_id
        - items
```

## Step 2: Generate Mock and Review

```bash
# Generate mock from contract
mockforge generate --from-openapi contracts/ecommerce-api.yaml --output mocks/

# Start mock server with Admin UI
mockforge serve --config mockforge.yaml --admin
```

**Review in Admin UI:**
1. Open `http://localhost:3000/__mockforge`
2. Browse endpoints
3. Test each endpoint
4. Review response structures
5. Note any design issues

## Step 3: Iterate on Design

Based on review feedback, update the contract:

```yaml
# Add pagination to list endpoints
paths:
  /api/users:
    get:
      parameters:
        - name: page
          in: query
          schema:
            type: integer
            default: 1
        - name: limit
          in: query
          schema:
            type: integer
            default: 20
      responses:
        '200':
          description: Paginated list of users
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/UserListResponse'

components:
  schemas:
    UserListResponse:
      type: object
      properties:
        data:
          type: array
          items:
            $ref: '#/components/schemas/User'
        pagination:
          $ref: '#/components/schemas/Pagination'

    Pagination:
      type: object
      properties:
        page:
          type: integer
        limit:
          type: integer
        total:
          type: integer
        total_pages:
          type: integer
```

Regenerate mock and review again.

## Step 4: Add Smart Personas

Configure personas for realistic data:

```yaml
# mockforge.yaml
reality:
  level: 3
  personas:
    enabled: true
    personas:
      - name: "premium_customer"
        domain: "ecommerce"
        traits:
          spending_level: "high"
          account_type: "premium"
          loyalty_level: "gold"

      - name: "regular_customer"
        domain: "ecommerce"
        traits:
          spending_level: "medium"
          account_type: "basic"
          loyalty_level: "silver"
```

## Step 5: Create Test Scenarios

Define test scenarios:

```yaml
# scenarios/ecommerce-flow.yaml
name: E-commerce Purchase Flow
description: Complete purchase flow from product browsing to order completion

steps:
  - name: Browse Products
    request:
      method: GET
      path: /api/products?category=electronics
    response:
      status: 200
      body:
        - id: "prod_1"
          name: "Laptop"
          price: 999.99
          category: "electronics"

  - name: Create User
    request:
      method: POST
      path: /api/users
      body:
        name: "Alice"
        email: "alice@example.com"
    response:
      status: 201
      body:
        id: "user_123"
        name: "Alice"
        email: "alice@example.com"

  - name: Create Order
    request:
      method: POST
      path: /api/orders
      body:
        user_id: "user_123"
        items:
          - product_id: "prod_1"
            quantity: 1
            price: 999.99
    response:
      status: 201
      body:
        id: "order_456"
        user_id: "user_123"
        total: 999.99
        status: "pending"
```

## Step 6: Implement Backend

Implement backend to match contract:

```rust
// Backend implementation matches contract exactly
#[get("/api/users")]
async fn get_users(
    Query(params): Query<PaginationParams>
) -> Json<UserListResponse> {
    // Implementation matches contract
}

#[post("/api/users")]
async fn create_user(
    Json(request): Json<CreateUserRequest>
) -> Result<Json<User>, StatusCode> {
    // Implementation matches contract
}
```

## Step 7: Validate and Deploy

```bash
# Validate implementation
mockforge validate \
  --contract contracts/ecommerce-api.yaml \
  --target http://localhost:8080

# Run integration tests
npm test -- --mock-server=http://localhost:3000

# Deploy if validation passes
```

## Best Practices Demonstrated

1. **Contract First** — Defined contract before implementation
2. **Mock Review** — Reviewed mock responses before coding
3. **Iteration** — Iterated on design based on feedback
4. **Personas** — Used Smart Personas for realistic data
5. **Scenarios** — Created test scenarios
6. **Validation** — Validated implementation against contract

## Next Steps

- Add more endpoints
- Create more scenarios
- Use Reality Continuum
- Explore advanced MOD patterns

## Further Reading

- [MOD Guide](../../MOD_GUIDE.md) — Complete workflow
- [MOD Patterns](../../MOD_PATTERNS.md) — Advanced patterns
- [MOD API Review](../../MOD_API_REVIEW.md) — Review process

---

**You've built an API using API-First MOD. Continue exploring advanced MOD patterns.**
