# MOD for Microservices

**Pillars:** [DevX][Contracts]

**Duration:** 45 minutes
**Prerequisites:** MOD Getting Started, Microservices architecture knowledge

## Overview

This tutorial demonstrates using MOD in a microservices architecture to coordinate development across multiple services.

## The Microservices Challenge

**Problem:** Multiple services need to coordinate, but development happens in parallel.

**MOD Solution:** Use shared contracts and mocks to coordinate services.

## Architecture

We'll build a simple e-commerce system with three services:

- **Users Service** — User management
- **Products Service** — Product catalog
- **Orders Service** — Order processing

## Step 1: Define Service Contracts

### Users Service Contract

```yaml
# contracts/users-service.yaml
openapi: 3.0.0
info:
  title: Users Service
  version: 1.0.0

paths:
  /api/users/{id}:
    get:
      responses:
        '200':
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/User'
```

### Products Service Contract

```yaml
# contracts/products-service.yaml
openapi: 3.0.0
info:
  title: Products Service
  version: 1.0.0

paths:
  /api/products/{id}:
    get:
      responses:
        '200':
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Product'
```

### Orders Service Contract

```yaml
# contracts/orders-service.yaml
openapi: 3.0.0
info:
  title: Orders Service
  version: 1.0.0

paths:
  /api/orders:
    post:
      requestBody:
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/CreateOrderRequest'
      responses:
        '201':
          content:
            application/json:
              schema:
                $ref: '#/components/schemas/Order'
```

## Step 2: Create Shared Contracts

Define shared schemas that services reference:

```yaml
# contracts/shared-schemas.yaml
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

    Product:
      type: object
      properties:
        id:
          type: string
        name:
          type: string
        price:
          type: number

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
```

## Step 3: Generate Mocks for Each Service

```bash
# Generate mock for users service
mockforge generate \
  --from-openapi contracts/users-service.yaml \
  --output mocks/users-service/

# Generate mock for products service
mockforge generate \
  --from-openapi contracts/products-service.yaml \
  --output mocks/products-service/

# Generate mock for orders service
mockforge generate \
  --from-openapi contracts/orders-service.yaml \
  --output mocks/orders-service/
```

## Step 4: Configure Multi-Service Workspace

```yaml
# mockforge.yaml
workspaces:
  - name: users-service
    port: 3001
    endpoints:
      - path: /api/users/*
        method: GET
        response:
          from_mock: mocks/users-service/

  - name: products-service
    port: 3002
    endpoints:
      - path: /api/products/*
        method: GET
        response:
          from_mock: mocks/products-service/

  - name: orders-service
    port: 3003
    endpoints:
      - path: /api/orders/*
        method: POST
        response:
          from_mock: mocks/orders-service/
```

## Step 5: Use Unified State

Configure unified state across services:

```yaml
# mockforge.yaml
core:
  consistency:
    enabled: true
    unified_state:
      workspace_id: "ecommerce-platform"
      persona_graph:
        enabled: true

# All services share same persona data
# GET /api/users/user_123 → same user data
# GET /api/orders?user_id=user_123 → same user's orders
```

## Step 6: Create Cross-Service Scenarios

Define scenarios that span multiple services:

```yaml
# scenarios/cross-service-purchase.yaml
name: Cross-Service Purchase Flow
description: Purchase flow across users, products, and orders services

steps:
  - name: Get User (Users Service)
    service: users-service
    request:
      method: GET
      path: /api/users/user_123
    response:
      status: 200
      body:
        id: "user_123"
        name: "Alice"
        email: "alice@example.com"

  - name: Get Product (Products Service)
    service: products-service
    request:
      method: GET
      path: /api/products/prod_456
    response:
      status: 200
      body:
        id: "prod_456"
        name: "Laptop"
        price: 999.99

  - name: Create Order (Orders Service)
    service: orders-service
    request:
      method: POST
      path: /api/orders
      body:
        user_id: "user_123"
        items:
          - product_id: "prod_456"
            quantity: 1
    response:
      status: 201
      body:
        id: "order_789"
        user_id: "user_123"
        total: 999.99
```

## Step 7: Coordinate Development

### Team 1: Users Service

```bash
# Team 1 works on users service
cd users-service/

# Use mock for other services
export PRODUCTS_API=http://localhost:3002
export ORDERS_API=http://localhost:3003

# Develop users service
npm run dev
```

### Team 2: Products Service

```bash
# Team 2 works on products service
cd products-service/

# Use mock for other services
export USERS_API=http://localhost:3001
export ORDERS_API=http://localhost:3003

# Develop products service
npm run dev
```

### Team 3: Orders Service

```bash
# Team 3 works on orders service
cd orders-service/

# Use mock for other services
export USERS_API=http://localhost:3001
export PRODUCTS_API=http://localhost:3002

# Develop orders service
npm run dev
```

## Step 8: Integration Testing

Test integration across services:

```typescript
// tests/integration/cross-service.test.ts
describe('Cross-Service Integration', () => {
  it('should create order with user and product', async () => {
    // Get user from users service
    const user = await fetch('http://localhost:3001/api/users/user_123');

    // Get product from products service
    const product = await fetch('http://localhost:3002/api/products/prod_456');

    // Create order in orders service
    const order = await fetch('http://localhost:3003/api/orders', {
      method: 'POST',
      body: JSON.stringify({
        user_id: 'user_123',
        items: [{ product_id: 'prod_456', quantity: 1 }],
      }),
    });

    expect(order.status).toBe(201);
  });
});
```

## Step 9: Contract Validation

Validate each service against its contract:

```bash
# Validate users service
mockforge validate \
  --contract contracts/users-service.yaml \
  --target http://localhost:8081

# Validate products service
mockforge validate \
  --contract contracts/products-service.yaml \
  --target http://localhost:8082

# Validate orders service
mockforge validate \
  --contract contracts/orders-service.yaml \
  --target http://localhost:8083
```

## Best Practices for Microservices MOD

### 1. Shared Contracts

✅ **Do:**
- Define shared schemas
- Version contracts explicitly
- Share contracts via Git

❌ **Don't:**
- Duplicate schemas
- Ignore contract versions
- Keep contracts isolated

### 2. Service Isolation

✅ **Do:**
- Each service has its own mock
- Services can develop independently
- Test services in isolation

❌ **Don't:**
- Share mock servers
- Couple services
- Skip isolation testing

### 3. Cross-Service Scenarios

✅ **Do:**
- Define cross-service scenarios
- Test end-to-end flows
- Validate integration

❌ **Don't:**
- Test services in isolation only
- Ignore integration
- Skip scenario testing

### 4. Unified State

✅ **Do:**
- Use unified state for consistency
- Share personas across services
- Maintain relationships

❌ **Don't:**
- Isolate service state
- Ignore relationships
- Skip consistency

## Advanced: Service Mesh Integration

For service mesh architectures:

```yaml
# mockforge.yaml
workspaces:
  - name: users-service
    service_mesh:
      enabled: true
      mesh_type: "istio"  # or "linkerd", "consul"
      virtual_service:
        hosts:
          - users-service
        routes:
          - match:
              - uri:
                  prefix: "/api/users"
            route:
              - destination:
                  host: users-service-mock
```

## Troubleshooting

### Problem: Services can't communicate

**Solution:**
- Check service ports
- Verify network configuration
- Test service connectivity

### Problem: State inconsistency

**Solution:**
- Enable unified state
- Use Smart Personas
- Configure persona graph

### Problem: Contract drift

**Solution:**
- Validate contracts regularly
- Use contract testing
- Fail builds on drift

## Further Reading

- [MOD Guide](../../MOD_GUIDE.md) — Complete workflow
- [MOD Patterns](../../MOD_PATTERNS.md) — Advanced patterns
- [Protocol Abstraction](../../PROTOCOL_ABSTRACTION.md) — Multi-protocol support

---

**MOD enables microservices teams to develop in parallel while maintaining integration. Start coordinating your services with MOD today.**
