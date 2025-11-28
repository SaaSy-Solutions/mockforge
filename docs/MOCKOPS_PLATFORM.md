# MockOps Platform - Implementation Plan

**Pillars:** [Cloud]

**Status:** Planning Phase

## Overview

The MockOps Platform transforms MockForge into a comprehensive orchestration and management system for mock environments at scale. It provides event-driven pipelines, multi-workspace federation, and comprehensive analytics dashboards for enterprise teams.

## Requirements Analysis

### 4.1 Workspace Orchestration Pipelines ("MockOps")

**Concept:** GitHub Actions + mocks - event-driven automation for mock lifecycle management.

**Required Features:**
1. **Schema Change → Auto-Regenerate SDK**
   - Detect OpenAPI/Protobuf schema changes
   - Trigger SDK regeneration for affected languages
   - Update client libraries automatically

2. **New Scenario Published → Auto-Promote to Test → Auto-Notify Teams**
   - Detect new scenario publication
   - Automatically promote to test environment
   - Send notifications to relevant teams (Slack, email, webhooks)

3. **Drift Exceeds Threshold → Auto-Generate Git PR**
   - Monitor drift budgets continuously
   - When threshold exceeded, create Git PR with fixes
   - Include OpenAPI spec updates, fixture updates, SDK regenerations

### 4.2 Multi-Workspace Federation

**Concept:** Compose multiple mock workspaces into one federated "virtual system" for large orgs with microservices.

**Required Features:**
1. **Service Boundary Definition**
   - Define service boundaries and relationships
   - Map services to workspaces
   - Configure inter-service dependencies

2. **Federated Virtual System**
   - Compose multiple workspaces into unified system
   - Single entry point for system-wide scenarios
   - Unified routing across federated services

3. **System-Wide Scenarios**
   - Define scenarios that span multiple services
   - Coordinate state across federated workspaces
   - End-to-end testing scenarios

4. **Per-Service Reality Level Control**
   - Configure reality level per service independently
   - Examples:
     - Auth = real upstream
     - Payments = mock v3
     - Inventory = blended
     - Shipping = chaos-driven

### 4.3 Team Heatmaps & Scenario Coverage

**Concept:** Cloud dashboard providing leadership insight into coverage, risk, and usage.

**Required Metrics:**
1. **Scenario Usage Heatmaps**
   - Which scenarios are used most
   - Usage patterns over time
   - Peak usage times

2. **Persona CI Hit Tracking**
   - Which personas are hit by CI
   - Persona usage frequency
   - Persona coverage gaps

3. **Endpoint Under-Test Detection**
   - Which endpoints are under-tested
   - Test coverage per endpoint
   - Missing test scenarios

4. **Stale Reality Level Detection**
   - Which mocks have stale reality levels
   - Reality level age tracking
   - Recommendations for updates

5. **Drift Percentage Tracking**
   - What percentage of mocks are drifting from real data
   - Drift trends over time
   - Risk assessment based on drift

## Existing Functionality

### ✅ Already Implemented

1. **Promotion Workflow** (`crates/mockforge-collab/src/promotion.rs`)
   - Scenario/persona/config promotion between environments
   - Approval workflow with status tracking
   - GitOps integration for PR creation
   - Pillar-based approval rules

2. **Drift Detection & GitOps** (`crates/mockforge-core/src/drift_gitops/`)
   - Drift budget monitoring
   - Automatic PR generation on threshold violation
   - OpenAPI spec and fixture updates
   - SDK regeneration hooks (configurable)

3. **Multi-Environment Workspaces** (`crates/mockforge-core/src/workspace/mock_environment.rs`)
   - Dev/test/prod environments
   - Per-environment reality level, chaos, drift budgets
   - Environment switching API

4. **Analytics Infrastructure** (`crates/mockforge-analytics/`)
   - Metrics aggregation (minute/hour/day)
   - Protocol/endpoint/workspace filtering
   - Database schema for analytics

5. **Multi-Tenant Workspaces** (`crates/mockforge-core/src/multi_tenant/`)
   - Workspace isolation
   - Path-based routing
   - Per-workspace statistics

### ❌ Missing Functionality

1. **Event-Driven Pipeline System**
   - No pipeline orchestration engine
   - No event triggers (schema change, scenario publish, drift threshold)
   - No pipeline definition DSL/YAML
   - No pipeline execution engine

2. **Multi-Workspace Federation**
   - No federation system
   - No service boundary definitions
   - No virtual system composition
   - No cross-workspace scenario coordination

3. **Comprehensive Coverage Dashboard**
   - Basic analytics exist but not the specific heatmaps
   - No persona CI hit tracking
   - No endpoint under-test detection
   - No stale reality level tracking
   - No drift percentage aggregation

## Implementation Plan

### Phase 1: Workspace Orchestration Pipelines

#### 1.1 Pipeline Engine Core

**New Crate:** `mockforge-pipelines`

**Components:**
- **Pipeline Definition DSL** - YAML-based pipeline definitions
- **Event System** - Event bus for triggering pipelines
- **Pipeline Executor** - Executes pipeline steps
- **Step Types** - Reusable pipeline steps (regenerate SDK, promote, notify, create PR)

**Pipeline Definition Example:**
```yaml
name: schema-change-pipeline
triggers:
  - event: schema.changed
    filters:
      workspace_id: "workspace-123"
      schema_type: ["openapi", "protobuf"]

steps:
  - name: regenerate-sdks
    type: regenerate_sdk
    config:
      languages: ["typescript", "python", "rust"]
      workspace_id: "{{workspace_id}}"

  - name: notify-teams
    type: notify
    config:
      channels: ["#api-team", "#frontend-team"]
      message: "SDKs regenerated for {{workspace_id}}"

  - name: update-ci
    type: update_ci
    config:
      workflow: ".github/workflows/sdk-tests.yml"
```

**Event Types:**
- `schema.changed` - OpenAPI/Protobuf schema modified
- `scenario.published` - New scenario published
- `drift.threshold_exceeded` - Drift budget exceeded
- `promotion.completed` - Promotion completed
- `workspace.created` - New workspace created

**Database Schema:**
```sql
CREATE TABLE pipelines (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    workspace_id UUID,
    org_id UUID,
    definition JSONB NOT NULL,
    enabled BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE pipeline_executions (
    id UUID PRIMARY KEY,
    pipeline_id UUID REFERENCES pipelines(id),
    trigger_event VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL,
    started_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    execution_log JSONB
);

CREATE TABLE pipeline_steps (
    id UUID PRIMARY KEY,
    execution_id UUID REFERENCES pipeline_executions(id),
    step_name VARCHAR(255) NOT NULL,
    step_type VARCHAR(100) NOT NULL,
    status VARCHAR(50) NOT NULL,
    started_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    output JSONB
);
```

#### 1.2 Schema Change Detection

**Enhancement to:** `crates/mockforge-recorder/src/sync.rs`

**Features:**
- Emit `schema.changed` event when sync detects schema changes
- Include change details (added/removed/modified endpoints)
- Trigger pipeline execution

#### 1.3 Auto-Promotion Pipeline

**Enhancement to:** `crates/mockforge-collab/src/promotion.rs`

**Features:**
- Emit `scenario.published` event
- Pipeline step: `auto_promote` - automatically promote to test
- Pipeline step: `notify` - send notifications to teams
- Configurable promotion rules (which scenarios auto-promote)

#### 1.4 SDK Regeneration Pipeline Step

**New Module:** `crates/mockforge-pipelines/src/steps/regenerate_sdk.rs`

**Features:**
- Trigger SDK generation for specified languages
- Update client libraries in repository
- Support for TypeScript, Python, Rust, Go, Java, .NET

**Integration:**
- Use existing SDK generation from `crates/mockforge-sdk/`
- Add pipeline step wrapper

### Phase 2: Multi-Workspace Federation

#### 2.1 Federation Core

**New Crate:** `mockforge-federation`

**Components:**
- **Service Registry** - Define services and their workspace mappings
- **Federation Router** - Route requests to appropriate workspace
- **Virtual System Manager** - Compose workspaces into virtual system
- **Cross-Workspace State** - Coordinate state across workspaces

**Service Definition:**
```yaml
federation:
  name: "e-commerce-platform"
  services:
    - name: "auth"
      workspace_id: "workspace-auth-123"
      base_path: "/auth"
      reality_level: "real"  # Use real upstream

    - name: "payments"
      workspace_id: "workspace-payments-456"
      base_path: "/payments"
      reality_level: "mock_v3"

    - name: "inventory"
      workspace_id: "workspace-inventory-789"
      base_path: "/inventory"
      reality_level: "blended"  # Mix of mock and real

    - name: "shipping"
      workspace_id: "workspace-shipping-012"
      base_path: "/shipping"
      reality_level: "chaos_driven"  # Chaos testing mode
```

**Database Schema:**
```sql
CREATE TABLE federations (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    org_id UUID NOT NULL,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE federation_services (
    id UUID PRIMARY KEY,
    federation_id UUID REFERENCES federations(id),
    service_name VARCHAR(255) NOT NULL,
    workspace_id UUID NOT NULL,
    base_path VARCHAR(255) NOT NULL,
    reality_level VARCHAR(50) NOT NULL,
    config JSONB,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE system_scenarios (
    id UUID PRIMARY KEY,
    federation_id UUID REFERENCES federations(id),
    name VARCHAR(255) NOT NULL,
    description TEXT,
    scenario_definition JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);
```

#### 2.2 Federation Router

**New Module:** `crates/mockforge-federation/src/router.rs`

**Features:**
- Route requests to appropriate workspace based on service mapping
- Handle cross-service calls
- Maintain request context across services
- Support for system-wide scenarios

#### 2.3 Per-Service Reality Level

**Enhancement to:** `crates/mockforge-core/src/workspace/mock_environment.rs`

**Features:**
- Support reality level per service in federation
- Override workspace reality level for federated services
- Support for: `real`, `mock_v3`, `blended`, `chaos_driven`

### Phase 3: Team Heatmaps & Scenario Coverage

#### 3.1 Coverage Analytics

**Enhancement to:** `crates/mockforge-analytics/`

**New Metrics:**
- Scenario usage frequency and patterns
- Persona CI hit tracking
- Endpoint test coverage
- Reality level staleness
- Drift percentage aggregation

**Database Schema Extensions:**
```sql
CREATE TABLE scenario_usage_metrics (
    id UUID PRIMARY KEY,
    scenario_id UUID NOT NULL,
    workspace_id UUID NOT NULL,
    usage_count BIGINT NOT NULL,
    last_used_at TIMESTAMPTZ,
    usage_pattern JSONB,  -- Time-based usage patterns
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE persona_ci_hits (
    id UUID PRIMARY KEY,
    persona_id UUID NOT NULL,
    workspace_id UUID NOT NULL,
    ci_run_id VARCHAR(255),
    hit_count BIGINT NOT NULL,
    hit_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE endpoint_coverage (
    id UUID PRIMARY KEY,
    endpoint VARCHAR(255) NOT NULL,
    workspace_id UUID NOT NULL,
    test_count BIGINT NOT NULL,
    last_tested_at TIMESTAMPTZ,
    coverage_percentage DECIMAL(5,2),
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE reality_level_staleness (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL,
    endpoint VARCHAR(255),
    current_reality_level VARCHAR(50),
    last_updated_at TIMESTAMPTZ,
    staleness_days INTEGER,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE drift_percentage_metrics (
    id UUID PRIMARY KEY,
    workspace_id UUID NOT NULL,
    total_mocks BIGINT NOT NULL,
    drifting_mocks BIGINT NOT NULL,
    drift_percentage DECIMAL(5,2) NOT NULL,
    measured_at TIMESTAMPTZ NOT NULL
);
```

#### 3.2 Dashboard UI

**Enhancement to:** `crates/mockforge-ui/ui/src/components/analytics/`

**New Components:**
- `ScenarioUsageHeatmap.tsx` - Heatmap of scenario usage
- `PersonaCIHits.tsx` - Persona CI hit tracking
- `EndpointCoverage.tsx` - Endpoint test coverage visualization
- `RealityLevelStaleness.tsx` - Stale reality level detection
- `DriftPercentageDashboard.tsx` - Drift percentage tracking

**API Endpoints:**
```rust
GET /api/v2/analytics/scenarios/usage?workspace_id={id}&time_range={range}
GET /api/v2/analytics/personas/ci-hits?workspace_id={id}
GET /api/v2/analytics/endpoints/coverage?workspace_id={id}
GET /api/v2/analytics/reality-levels/staleness?workspace_id={id}
GET /api/v2/analytics/drift/percentage?workspace_id={id}
```

## Implementation Priority

### High Priority (MVP)
1. **Pipeline Engine Core** - Foundation for all automation
2. **Schema Change → SDK Regeneration** - Most requested feature
3. **Scenario Auto-Promotion** - Reduces manual work
4. **Basic Federation** - Service boundary definition and routing

### Medium Priority
1. **Drift Threshold → Auto-PR** - Enhance existing drift GitOps
2. **Coverage Dashboard** - Leadership visibility
3. **System-Wide Scenarios** - Advanced federation features

### Low Priority (Future)
1. **Advanced Federation Features** - Cross-workspace state coordination
2. **Advanced Heatmaps** - Time-based pattern analysis
3. **ML-Based Recommendations** - AI-powered suggestions

## Technical Considerations

### Event System
- Use async event bus (tokio broadcast channel or Redis pub/sub for distributed)
- Event persistence for audit trail
- Event replay capability for debugging

### Pipeline Execution
- Async step execution with timeout handling
- Step retry logic with exponential backoff
- Pipeline cancellation support
- Step output caching

### Federation Performance
- Efficient routing with minimal overhead
- Request context propagation
- Cross-workspace call optimization

### Analytics Performance
- Efficient aggregation queries
- Time-series data optimization
- Dashboard query caching
- Real-time updates via WebSocket

## Integration Points

### Existing Systems
- **Promotion Service** - Emit events, support pipeline steps
- **Drift Detection** - Emit events, enhance GitOps integration
- **Analytics** - Extend metrics, add new aggregations
- **Workspace Management** - Support federation, service mapping

### External Integrations
- **GitHub/GitLab** - PR creation, webhook triggers
- **Slack/Email** - Notification delivery
- **CI/CD Systems** - Pipeline triggers, status updates

## Testing Strategy

### Unit Tests
- Pipeline step execution
- Event emission and handling
- Federation routing logic
- Analytics aggregation

### Integration Tests
- End-to-end pipeline execution
- Federation request routing
- Cross-workspace scenario execution
- Dashboard data accuracy

### E2E Tests
- Complete pipeline workflows
- Federation system scenarios
- Dashboard user workflows

## Documentation Requirements

1. **Pipeline DSL Reference** - Complete pipeline definition guide
2. **Federation Guide** - Setting up and using federated workspaces
3. **Coverage Dashboard Guide** - Understanding metrics and heatmaps
4. **API Documentation** - All new endpoints
5. **Migration Guide** - Upgrading existing workspaces to use pipelines

## Success Metrics

1. **Pipeline Adoption** - % of workspaces using pipelines
2. **Automation Rate** - % of promotions/scenarios handled automatically
3. **Federation Usage** - Number of federated systems
4. **Dashboard Engagement** - Dashboard views and usage
5. **Time Savings** - Reduction in manual mock management tasks
