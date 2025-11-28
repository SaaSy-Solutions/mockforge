# Multi-Workspace Federation

**Pillars:** [Cloud]

Multi-Workspace Federation enables composing multiple mock workspaces into one federated "virtual system" for large organizations with microservices architectures.

## Overview

Federation allows you to:

- **Define Service Boundaries**: Map services to workspaces
- **Compose Virtual Systems**: Combine multiple workspaces into one system
- **Run System-Wide Scenarios**: Define scenarios that span multiple services
- **Control Reality Per Service**: Set reality level independently per service

## Key Concepts

### Service Boundaries

Services represent individual microservices in your architecture:

```yaml
services:
  - name: auth
    workspace_id: "workspace-auth-123"
    base_path: "/auth"
    reality_level: "real"  # Use real upstream
```

### Federation

A federation is a collection of services that work together:

```yaml
federation:
  name: "e-commerce-platform"
  services:
    - name: auth
      workspace_id: "workspace-auth-123"
      base_path: "/auth"
      reality_level: "real"
    - name: payments
      workspace_id: "workspace-payments-456"
      base_path: "/payments"
      reality_level: "mock_v3"
```

### Virtual System

The federated system appears as a single unified API:

```
/api/auth/*          → Auth service (real)
/api/payments/*      → Payments service (mock v3)
/api/inventory/*    → Inventory service (blended)
/api/shipping/*     → Shipping service (chaos-driven)
```

## Service Reality Levels

Each service can have its own reality level:

### Real

Use real upstream (no mocking):

```yaml
services:
  - name: auth
    reality_level: "real"
```

**Use Case:** Critical services that must be real (auth, payment processing).

### Mock V3

Use mock with reality level 3:

```yaml
services:
  - name: payments
    reality_level: "mock_v3"
```

**Use Case:** Services under development or testing.

### Blended

Mix of mock and real data:

```yaml
services:
  - name: inventory
    reality_level: "blended"
    blend_ratio: 0.5  # 50% real, 50% mock
```

**Use Case:** Gradual migration from mock to real.

### Chaos-Driven

Chaos testing mode:

```yaml
services:
  - name: shipping
    reality_level: "chaos_driven"
```

**Use Case:** Resilience testing, chaos engineering.

## Configuration

### Define Federation

```yaml
# federation.yaml
federation:
  name: "e-commerce-platform"
  description: "Federated e-commerce system"
  services:
    - name: auth
      workspace_id: "workspace-auth-123"
      base_path: "/auth"
      reality_level: "real"
      config:
        upstream_url: "https://auth.example.com"
    
    - name: payments
      workspace_id: "workspace-payments-456"
      base_path: "/payments"
      reality_level: "mock_v3"
      config:
        reality_level: 3
        chaos_enabled: false
    
    - name: inventory
      workspace_id: "workspace-inventory-789"
      base_path: "/inventory"
      reality_level: "blended"
      config:
        blend_ratio: 0.5
        reality_continuum:
          enabled: true
          default_ratio: 0.5
    
    - name: shipping
      workspace_id: "workspace-shipping-012"
      base_path: "/shipping"
      reality_level: "chaos_driven"
      config:
        chaos:
          enabled: true
          error_rate: 0.1
          latency_spike_probability: 0.2
```

### Service Dependencies

Define dependencies between services:

```yaml
services:
  - name: orders
    workspace_id: "workspace-orders-123"
    base_path: "/orders"
    dependencies:
      - payments
      - inventory
      - shipping
```

Dependencies are used for:
- System-wide scenario ordering
- Service startup coordination
- Dependency graph visualization

## System-Wide Scenarios

Define scenarios that span multiple services:

```yaml
system_scenarios:
  - name: end-to-end-checkout
    description: "Complete checkout flow across services"
    steps:
      - service: auth
        endpoint: POST /auth/login
        extract:
          token: "body.token"
      
      - service: inventory
        endpoint: GET /inventory/products/{product_id}
        headers:
          Authorization: "Bearer {{token}}"
        extract:
          product: "body"
      
      - service: payments
        endpoint: POST /payments/charge
        headers:
          Authorization: "Bearer {{token}}"
        body:
          amount: "{{product.price}}"
        extract:
          payment_id: "body.id"
      
      - service: orders
        endpoint: POST /orders
        headers:
          Authorization: "Bearer {{token}}"
        body:
          product_id: "{{product.id}}"
          payment_id: "{{payment_id}}"
```

## Routing

The federation router routes requests to appropriate services:

### Path-Based Routing

Requests are routed based on path prefixes:

```
GET /auth/users/123     → Auth service
GET /payments/charge    → Payments service
GET /inventory/products → Inventory service
```

### Longest Match

The router uses longest match for path matching:

```
/api/v1/payments/charge → Payments service (longer match)
/api/v1/payments         → Payments service
/api/v1                 → Default service
```

## Usage

### Create Federation

```bash
# Create federation from YAML
mockforge federation create federation.yaml

# Or via API
POST /api/v1/federations
{
  "name": "e-commerce-platform",
  "services": [...]
}
```

### Start Federated System

```bash
# Start all services in federation
mockforge federation start e-commerce-platform

# Start specific services
mockforge federation start e-commerce-platform --services auth,payments
```

### Run System-Wide Scenario

```bash
# Run scenario across all services
mockforge federation scenario run e-commerce-platform end-to-end-checkout
```

## Real-World Example

### E-Commerce Platform

```yaml
federation:
  name: "e-commerce-platform"
  services:
    # Auth - Always real (critical service)
    - name: auth
      workspace_id: "workspace-auth"
      base_path: "/auth"
      reality_level: "real"
    
    # Payments - Mock v3 (under development)
    - name: payments
      workspace_id: "workspace-payments"
      base_path: "/payments"
      reality_level: "mock_v3"
    
    # Inventory - Blended (gradual migration)
    - name: inventory
      workspace_id: "workspace-inventory"
      base_path: "/inventory"
      reality_level: "blended"
      config:
        blend_ratio: 0.3  # 30% real, 70% mock
    
    # Shipping - Chaos-driven (resilience testing)
    - name: shipping
      workspace_id: "workspace-shipping"
      base_path: "/shipping"
      reality_level: "chaos_driven"
```

**Result:** Single unified API with different reality levels per service.

## Best Practices

1. **Start Small**: Begin with 2-3 services
2. **Define Boundaries**: Clearly define service boundaries
3. **Use Dependencies**: Document service dependencies
4. **Test Scenarios**: Create system-wide test scenarios
5. **Monitor Routing**: Monitor routing performance

## Related Documentation

- [MockOps Pipelines](mockops-pipelines.md) - Pipeline automation
- [Analytics Dashboard](analytics-dashboard.md) - Usage analytics
- [Cloud Workspaces](cloud-workspaces.md) - Workspace management

