# Natural Language to System Generation

**Pillars:** [AI]

Natural Language to System Generation allows you to describe your product in natural language and MockForge generates an entire backend system including endpoints, personas, lifecycle states, WebSocket topics, failure scenarios, and more.

## Overview

Instead of manually creating APIs, describe your product and get:

- **20-30 REST endpoints** (OpenAPI 3.1 spec)
- **4-5 personas** (based on roles)
- **6-10 lifecycle states** (state machines)
- **WebSocket topics** (if real-time features mentioned)
- **Payment failure scenarios**
- **Surge pricing chaos profiles**
- **Full OpenAPI specification**
- **Mock backend configuration** (mockforge.yaml)
- **GraphQL schema** (optional)
- **Typings** (TypeScript/Go/Rust)
- **CI pipeline templates** (GitHub Actions, GitLab CI)

This becomes **a way to bootstrap an entire startup backend**.

## Quick Start

### Basic Usage

```bash
# Generate system from natural language
mockforge ai generate-system \
  "I'm building a ride-sharing app with drivers, riders, trips, payments, live-location updates, pricing, and surge events."
```

### Via AI Studio

1. Navigate to AI Studio
2. Select "System Generation"
3. Enter your product description
4. Select output formats
5. Generate system

## Example: Ride-Sharing App

### Input

```
I'm building a ride-sharing app with drivers, riders, trips, payments, 
live-location updates, pricing, and surge events.
```

### Generated Output

#### OpenAPI Specification

```yaml
openapi: 3.1.0
info:
  title: Ride-Sharing API
  version: 1.0.0
paths:
  /api/drivers:
    get:
      summary: List drivers
      responses:
        '200':
          description: List of drivers
          content:
            application/json:
              schema:
                type: array
                items:
                  $ref: '#/components/schemas/Driver'
  /api/riders:
    get:
      summary: List riders
      # ... more endpoints
  /api/trips:
    post:
      summary: Create trip
      # ... more endpoints
  /api/payments:
    post:
      summary: Process payment
      # ... more endpoints
```

#### Personas

```yaml
personas:
  - id: driver:premium-001
    name: Premium Driver
    domain: ridesharing
    traits:
      vehicle_type: premium
      rating: 4.9
      total_trips: 500
  - id: rider:frequent-002
    name: Frequent Rider
    domain: ridesharing
    traits:
      ride_frequency: high
      preferred_payment: credit_card
```

#### Lifecycle States

```yaml
lifecycles:
  - entity: trip
    states:
      - requested
      - matched
      - in_progress
      - completed
      - cancelled
    transitions:
      - from: requested
        to: matched
        condition: driver_accepts
      - from: matched
        to: in_progress
        condition: trip_starts
```

#### WebSocket Topics

```yaml
websocket_topics:
  - topic: location_updates
    event_types:
      - driver_location
      - rider_location
  - topic: trip_status
    event_types:
      - trip_requested
      - trip_matched
      - trip_completed
  - topic: surge_alerts
    event_types:
      - surge_started
      - surge_ended
```

#### Chaos Profiles

```yaml
chaos_profiles:
  - name: payment_failure
    type: payment_failure
    config:
      failure_scenarios:
        - insufficient_funds
        - card_declined
        - network_error
  - name: surge_pricing
    type: surge_pricing
    config:
      peak_hours:
        - "08:00-10:00"
        - "17:00-19:00"
      surge_multipliers: [1.5, 2.0, 3.0]
```

## Output Formats

### OpenAPI Specification

Full OpenAPI 3.1 specification with:
- All endpoints
- Request/response schemas
- Authentication
- Error responses

### Personas

4-5 personas based on roles:
- Driver personas
- Rider personas
- Admin personas
- Support personas

### Lifecycle States

6-10 lifecycle states for main entities:
- Trip lifecycle
- Payment lifecycle
- Driver lifecycle
- Rider lifecycle

### WebSocket Topics

If real-time features are mentioned:
- Topic names
- Event types
- Event schemas

### Chaos Profiles

Failure and edge case scenarios:
- Payment failures
- Network errors
- Surge pricing
- Service outages

### CI/CD Templates

GitHub Actions and GitLab CI templates:
- Test workflows
- Deployment workflows
- Integration tests

### GraphQL Schema

Optional GraphQL schema:
- Type definitions
- Queries
- Mutations

### Typings

Type definitions:
- TypeScript
- Go
- Rust
- Python

## Advanced Features

### Versioned Artifacts

Generated artifacts are versioned (v1, v2, etc.) and never mutate existing:

```
generated/
├── v1/
│   ├── openapi.json
│   ├── personas.yaml
│   └── scenarios.yaml
└── v2/
    ├── openapi.json
    ├── personas.yaml
    └── scenarios.yaml
```

### Deterministic Mode

Honors workspace `ai.deterministic_mode` setting:

```yaml
ai:
  deterministic_mode:
    enabled: true
    auto_freeze: true
    freeze_format: yaml
```

When enabled, AI outputs are frozen to deterministic YAML/JSON.

### System Coherence Validation

Ensures all generated components are coherent:
- Personas match endpoints
- Lifecycles match entities
- Scenarios match personas
- WebSocket topics match real-time features

## Configuration

### Enable System Generation

```yaml
# mockforge.yaml
ai_studio:
  system_generation:
    enabled: true
    default_output_formats:
      - openapi
      - personas
      - lifecycles
      - scenarios
```

### Output Format Selection

```bash
# Generate with specific formats
mockforge ai generate-system \
  "Description..." \
  --formats openapi,personas,lifecycles,graphql
```

## Best Practices

1. **Be Specific**: Detailed descriptions yield better results
2. **Mention Roles**: Include user roles (drivers, riders, admins)
3. **Specify Features**: Mention real-time, payments, etc.
4. **Review Generated**: Always review and refine generated artifacts
5. **Iterate**: Generate multiple versions and compare

## Real-World Examples

### E-Commerce Platform

**Input:**
```
E-commerce platform with products, shopping cart, orders, payments, 
inventory management, shipping, and customer reviews.
```

**Generated:**
- 25 REST endpoints
- 5 personas (customer, admin, vendor, support, reviewer)
- 8 lifecycle states (order, payment, shipping)
- WebSocket topics (order_updates, inventory_alerts)
- Payment failure scenarios
- Inventory depletion chaos profiles

### SaaS Platform

**Input:**
```
SaaS platform with users, subscriptions, billing, teams, projects, 
tasks, and real-time collaboration.
```

**Generated:**
- 30 REST endpoints
- 6 personas (admin, owner, member, viewer, billing_admin, support)
- 10 lifecycle states (subscription, project, task)
- WebSocket topics (collaboration_updates, task_updates)
- Billing failure scenarios
- Subscription lifecycle chaos profiles

## Related Documentation

- [AI Studio](llm-studio.md) - AI features overview
- [API Architecture Critique](api-architecture-critique.md) - API analysis
- [Behavioral Simulation](behavioral-simulation.md) - AI behavioral simulation

