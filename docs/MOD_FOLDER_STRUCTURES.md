# MOD Folder Structures

**Pillars:** [DevX]

**Version:** 1.0.0
**Last Updated:** 2025-01-27

Recommended folder structures for MOD (Mock-Oriented Development) projects, organized by team size and project type.

## Table of Contents

- [Solo Developer](#solo-developer)
- [Small Team (2-5 developers)](#small-team-2-5-developers)
- [Large Team (6+ developers)](#large-team-6-developers)
- [Monorepo](#monorepo)
- [Microservices](#microservices)
- [Frontend-Focused](#frontend-focused)

## Solo Developer

**Best for:** Individual projects, prototypes, learning MOD

```
my-project/
├── mockforge.yaml          # MOD configuration
├── contracts/              # API contracts
│   ├── api.yaml           # Main API contract
│   └── schemas/           # Shared schemas
├── mocks/                  # Mock definitions
│   └── responses/         # Mock response files
├── scenarios/              # Test scenarios
│   └── user-journeys.yaml
├── personas/              # Persona definitions
│   └── default.yaml
└── README.md
```

**Characteristics:**
- Simple, flat structure
- Single contract file
- Minimal organization
- Quick to set up

## Small Team (2-5 developers)

**Best for:** Small teams, single service, shared ownership

```
my-api/
├── mockforge.yaml          # MOD configuration
├── contracts/              # API contracts
│   ├── v1/                # API version 1
│   │   ├── openapi.yaml
│   │   └── schemas/
│   └── v2/                # API version 2
│       ├── openapi.yaml
│       └── schemas/
├── mocks/                  # Mock definitions
│   ├── endpoints/         # Per-endpoint mocks
│   │   ├── users.yaml
│   │   └── orders.yaml
│   └── scenarios/         # Scenario-based mocks
│       └── checkout-flow.yaml
├── scenarios/              # Test scenarios
│   ├── happy-paths/
│   ├── error-paths/
│   └── edge-cases/
├── personas/              # Persona definitions
│   ├── users/
│   │   ├── premium.yaml
│   │   └── basic.yaml
│   └── orders/
│       └── high-value.yaml
├── tests/                 # Integration tests
│   ├── contract-tests/
│   └── integration-tests/
└── README.md
```

**Characteristics:**
- Versioned contracts
- Organized by feature
- Shared personas
- Team collaboration

## Large Team (6+ developers)

**Best for:** Large teams, multiple services, clear ownership

```
api-platform/
├── mockforge.yaml          # Root MOD configuration
├── contracts/              # All API contracts
│   ├── users-service/
│   │   ├── v1/
│   │   └── v2/
│   ├── orders-service/
│   │   └── v1/
│   └── payments-service/
│       └── v1/
├── mocks/                  # All mock definitions
│   ├── users-service/
│   │   ├── endpoints/
│   │   └── scenarios/
│   ├── orders-service/
│   │   ├── endpoints/
│   │   └── scenarios/
│   └── payments-service/
│       ├── endpoints/
│       └── scenarios/
├── scenarios/              # Cross-service scenarios
│   ├── ecommerce-flow/
│   └── user-onboarding/
├── personas/              # Shared personas
│   ├── users/
│   ├── orders/
│   └── payments/
├── tests/                 # All tests
│   ├── contract-tests/
│   ├── integration-tests/
│   └── e2e-tests/
└── docs/                  # Documentation
    ├── api-design/
    └── mod-guide.md
```

**Characteristics:**
- Service-based organization
- Clear ownership
- Shared resources
- Comprehensive testing

## Monorepo

**Best for:** Monorepo projects, shared codebase, multiple services

```
monorepo/
├── services/
│   ├── users-service/
│   │   ├── mockforge.yaml
│   │   ├── contracts/
│   │   ├── mocks/
│   │   └── tests/
│   ├── orders-service/
│   │   ├── mockforge.yaml
│   │   ├── contracts/
│   │   ├── mocks/
│   │   └── tests/
│   └── payments-service/
│       ├── mockforge.yaml
│       ├── contracts/
│       ├── mocks/
│       └── tests/
├── shared/
│   ├── contracts/         # Shared contracts
│   │   └── common-schemas/
│   ├── personas/          # Shared personas
│   └── scenarios/         # Cross-service scenarios
├── tools/
│   └── mod-scripts/       # MOD automation scripts
└── mockforge.workspace.yaml  # Workspace configuration
```

**Characteristics:**
- Per-service organization
- Shared resources
- Workspace configuration
- Centralized tooling

## Microservices

**Best for:** Microservices architecture, independent services

```
microservices/
├── api-gateway/
│   ├── mockforge.yaml
│   ├── contracts/
│   └── mocks/
├── users-service/
│   ├── mockforge.yaml
│   ├── contracts/
│   ├── mocks/
│   └── tests/
├── orders-service/
│   ├── mockforge.yaml
│   ├── contracts/
│   ├── mocks/
│   └── tests/
├── payments-service/
│   ├── mockforge.yaml
│   ├── contracts/
│   ├── mocks/
│   └── tests/
└── shared/
    ├── contracts/         # Shared contracts
    ├── personas/          # Shared personas
    └── scenarios/         # Cross-service scenarios
```

**Characteristics:**
- Independent services
- Shared contracts
- Cross-service scenarios
- Service-specific mocks

## Frontend-Focused

**Best for:** Frontend teams, API consumer perspective

```
frontend-app/
├── mockforge.yaml          # MOD configuration
├── contracts/              # API contracts (from backend teams)
│   ├── users-api.yaml
│   ├── orders-api.yaml
│   └── payments-api.yaml
├── mocks/                  # Mock definitions
│   ├── local/             # Local development mocks
│   └── scenarios/         # Test scenarios
├── personas/              # Frontend personas
│   └── user-types.yaml
├── tests/                 # Frontend tests
│   ├── component-tests/
│   └── integration-tests/
└── README.md
```

**Characteristics:**
- Consumer-focused
- Contract-driven
- Local development
- Frontend testing

## Folder Structure Best Practices

### 1. Organize by Service/Feature

✅ **Do:**
```
services/
  users-service/
    contracts/
    mocks/
```

❌ **Don't:**
```
contracts/
  users-service/
mocks/
  users-service/
```

### 2. Version Contracts Explicitly

✅ **Do:**
```
contracts/
  v1/
  v2/
```

❌ **Don't:**
```
contracts/
  api.yaml
  api-v2.yaml
```

### 3. Separate Endpoints and Scenarios

✅ **Do:**
```
mocks/
  endpoints/
  scenarios/
```

❌ **Don't:**
```
mocks/
  everything.yaml
```

### 4. Use Shared Resources

✅ **Do:**
```
shared/
  contracts/
  personas/
```

❌ **Don't:**
```
users-service/
  contracts/
orders-service/
  contracts/  # Duplicate
```

### 5. Keep Tests Close to Mocks

✅ **Do:**
```
service/
  mocks/
  tests/
```

❌ **Don't:**
```
mocks/
  service/
tests/
  service/  # Far from mocks
```

## Template Generation

Use MOD CLI to generate folder structures:

```bash
# Generate solo developer structure
mockforge mod init --template solo my-project

# Generate small team structure
mockforge mod init --template small-team my-api

# Generate large team structure
mockforge mod init --template large-team api-platform

# Generate monorepo structure
mockforge mod init --template monorepo my-monorepo

# Generate microservices structure
mockforge mod init --template microservices my-services

# Generate frontend-focused structure
mockforge mod init --template frontend my-frontend
```

## Migration Guide

### From Unorganized to MOD

**Step 1:** Identify existing mocks and contracts

**Step 2:** Organize into MOD structure

**Step 3:** Update references

**Step 4:** Validate structure

**Example:**

```bash
# Before (unorganized)
project/
  mocks.yaml
  api.yaml

# After (MOD structure)
project/
  contracts/
    api.yaml
  mocks/
    endpoints/
      users.yaml
      orders.yaml
```

## Further Reading

- [MOD Philosophy](MOD_PHILOSOPHY.md) — Core MOD principles
- [MOD Guide](MOD_GUIDE.md) — Step-by-step workflow
- [MOD Patterns](MOD_PATTERNS.md) — Common patterns

---

**Choose the structure that fits your team. Start simple, evolve as needed.**
