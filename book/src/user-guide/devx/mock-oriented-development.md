# Mock-Oriented Development (MOD)

**Pillars:** [DevX][Reality][Contracts]

Mock-Oriented Development (MOD) is a software development methodology that places mocks at the center of the development workflow. Just as Test-Driven Development (TDD) revolutionized testing, and Infrastructure as Code (IaC) transformed DevOps, MOD transforms how we build and integrate APIs.

## What is MOD?

MOD is not just about using mocks—it's about **thinking mock-first**, **designing with mocks**, and **building confidence through realistic simulation**.

### The MOD Manifesto

1. **Mocks are not afterthoughts** — They are first-class citizens in your development process
2. **Design with mocks** — Use mocks to explore API designs before implementation
3. **Reality matters** — Mocks should feel indistinguishable from real backends
4. **Contracts are sacred** — Mocks enforce and validate API contracts
5. **Iterate fearlessly** — Mocks enable rapid iteration without breaking dependencies
6. **Collaborate through mocks** — Teams work in parallel using shared mock definitions

## Why MOD?

### The Problem MOD Solves

Traditional development workflows create bottlenecks:

- **Frontend teams** wait for backend APIs to be ready
- **Backend teams** build APIs in isolation without early feedback
- **Integration** happens late, revealing design flaws when changes are expensive
- **Testing** relies on fragile, hard-to-maintain fixtures
- **Documentation** is outdated or missing

### The MOD Solution

MOD flips the script:

- **Frontend teams** start immediately with realistic mocks
- **Backend teams** validate designs through mock-driven API reviews
- **Integration** happens continuously through shared mock contracts
- **Testing** uses living, evolving mock scenarios
- **Documentation** is generated from mock definitions

## MOD vs. Other Methodologies

### MOD vs. TDD (Test-Driven Development)

| Aspect | TDD | MOD |
|--------|-----|-----|
| **Focus** | Tests drive implementation | Mocks drive design and integration |
| **When** | During implementation | Before and during implementation |
| **Scope** | Unit/component level | System/integration level |
| **Artifact** | Test code | Mock definitions + contracts |

**MOD complements TDD**: Use TDD for implementation, MOD for integration.

### MOD vs. BDD (Behavior-Driven Development)

| Aspect | BDD | MOD |
|--------|-----|-----|
| **Focus** | Behavior specifications | API contracts and interactions |
| **Language** | Natural language (Gherkin) | API schemas (OpenAPI, gRPC) |
| **Scope** | Feature behavior | System integration |
| **Artifact** | Feature files | Mock scenarios |

**MOD complements BDD**: Use BDD for features, MOD for APIs.

### MOD vs. IaC (Infrastructure as Code)

| Aspect | IaC | MOD |
|--------|-----|-----|
| **Focus** | Infrastructure provisioning | API simulation |
| **Domain** | DevOps/Infrastructure | Development/Integration |
| **Artifact** | Terraform/CloudFormation | Mock configurations |
| **Lifecycle** | Deploy/manage infrastructure | Simulate/validate APIs |

**MOD is like IaC for APIs**: Version-controlled, reproducible, declarative.

## MOD Principles

### 1. Mock-First Design

**Start with mocks, not implementations.**

Before writing backend code, create mock APIs that define:
- Request/response schemas
- Error scenarios
- Edge cases
- Performance characteristics

**Benefits:**
- Early validation of API design
- Frontend can start immediately
- Clear contract definition
- Stakeholder feedback before implementation

### 2. Contract-Driven Development

**Contracts are the source of truth.**

Define API contracts (OpenAPI, gRPC, GraphQL) first:
- Generate mocks from contracts
- Validate implementations against contracts
- Detect contract drift automatically
- Version contracts explicitly

**Benefits:**
- Single source of truth
- Automatic validation
- Breaking change detection
- Clear API evolution

### 3. Reality Progression

**Gradually increase mock realism.**

Start with simple mocks, then add:
- Realistic data generation
- Behavioral patterns
- Error scenarios
- Performance characteristics

**Benefits:**
- Early development with simple mocks
- Gradual complexity as needed
- Production-like testing when ready
- Smooth transition to real backend

### 4. Scenario-Driven Testing

**Test with scenarios, not fixtures.**

Use scenarios that define:
- Multi-step workflows
- State transitions
- Error paths
- Edge cases

**Benefits:**
- Realistic test flows
- Reusable scenarios
- Easy scenario updates
- Living test documentation

### 5. Continuous Integration

**Mocks are part of CI/CD.**

Integrate mocks into:
- Development workflows
- CI/CD pipelines
- Testing strategies
- Documentation generation

**Benefits:**
- Automated validation
- Early error detection
- Consistent environments
- Up-to-date documentation

## MOD Workflow

### 1. Design Phase

```
Define Contract → Create Mock → Review with Team → Iterate
```

1. Define API contract (OpenAPI, gRPC, etc.)
2. Generate initial mock from contract
3. Review mock responses with team
4. Iterate on design based on feedback

### 2. Development Phase

```
Frontend: Use Mock → Backend: Implement Contract → Integration: Validate
```

1. Frontend team uses mock for development
2. Backend team implements to match contract
3. Integration validates implementation against mock

### 3. Testing Phase

```
Unit Tests → Integration Tests → E2E Tests (all with mocks)
```

1. Unit tests use mock responses
2. Integration tests use mock services
3. E2E tests use mock backends

### 4. Review Phase

```
PR Review → Contract Validation → Mock Comparison → Approval
```

1. PR includes contract changes
2. Validate contract changes
3. Compare mock vs. implementation
4. Approve if contract matches

## Getting Started with MOD

### 1. Initialize MOD Project

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
```

### 4. Use Mock in Development

```typescript
// Frontend code uses mock
import { useUser } from './generated/clients/react';

function UserProfile({ userId }) {
  const { data, loading } = useUser(userId);
  // ...
}
```

## MOD Folder Structures

### Solo Developer

```
my-project/
├── mockforge.yaml
├── contracts/
│   ├── api.yaml
│   └── schemas/
├── mocks/
│   └── responses/
├── scenarios/
│   └── user-journeys.yaml
├── personas/
│   └── default.yaml
└── README.md
```

### Small Team (2-5 developers)

```
my-api/
├── mockforge.yaml
├── contracts/
│   ├── v1/
│   │   ├── openapi.yaml
│   │   └── schemas/
│   └── v2/
│       ├── openapi.yaml
│       └── schemas/
├── mocks/
│   ├── endpoints/
│   │   ├── users.yaml
│   │   └── orders.yaml
│   └── scenarios/
│       └── checkout-flow.yaml
├── scenarios/
│   ├── happy-paths/
│   ├── error-paths/
│   └── edge-cases/
└── personas/
    ├── customers.yaml
    └── admins.yaml
```

### Large Team (6+ developers)

```
monorepo/
├── services/
│   ├── auth-service/
│   │   ├── contracts/
│   │   ├── mocks/
│   │   └── scenarios/
│   ├── payment-service/
│   │   ├── contracts/
│   │   ├── mocks/
│   │   └── scenarios/
│   └── order-service/
│       ├── contracts/
│       ├── mocks/
│       └── scenarios/
├── shared/
│   ├── contracts/  # Shared contracts
│   └── personas/   # Shared personas
└── mockforge.yaml  # Root configuration
```

## MOD Patterns

### Pattern 1: Contract-First API Design

1. Design contract (OpenAPI)
2. Generate mock
3. Review with team
4. Implement backend
5. Validate against contract

### Pattern 2: Scenario-Driven Development

1. Define scenario (user journey)
2. Create mocks for scenario
3. Implement frontend using mocks
4. Implement backend to match scenario
5. Test end-to-end with scenario

### Pattern 3: Progressive Realism

1. Start with simple mocks (static responses)
2. Add realistic data (faker generation)
3. Add behavioral patterns (personas)
4. Add error scenarios (chaos)
5. Blend with real backend (reality continuum)

## MOD Success Metrics

Track MOD effectiveness:

- **Time to First Integration** — How quickly can teams integrate?
- **Contract Drift** — How often do implementations diverge from contracts?
- **Frontend Blocking** — How often is frontend blocked by backend?
- **API Review Time** — How long do API reviews take?
- **Integration Bugs** — How many bugs are found during integration?

## Best Practices

1. **Start Early**: Create mocks before implementation
2. **Version Contracts**: Use semantic versioning for contracts
3. **Keep Mocks Updated**: Update mocks as contracts evolve
4. **Use Scenarios**: Test with scenarios, not just fixtures
5. **Document Decisions**: Document why mocks are configured certain ways

## Related Documentation

- [Getting Started](../../getting-started/getting-started.md) - Quick start guide
- [Smart Personas](smart-personas.md) - Persona system
- [Reality Continuum](reality-continuum.md) - Reality progression
- [Scenarios](scenario-state-machines.md) - Scenario system
- [Contracts](../../docs/PILLARS.md#contracts) - Contract management

