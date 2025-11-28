# Book Documentation Update Plan

**Date**: 2025-01-27  
**Status**: Planning updates for newly verified features

## Overview

Based on the Feature Verification Report, the following features need to be documented in the book. Many are fully implemented but missing from the documentation.

---

## Features Requiring New Documentation

### 1. [Reality] Features

#### 1.1 Reality Profiles Marketplace
**Status**: Missing from book  
**Priority**: High  
**Location**: `user-guide/advanced-features/reality-profiles-marketplace.md`

**Content Needed**:
- Overview of pre-tuned reality profile packs
- E-Commerce Peak Season Pack
- Fintech Fraud Pack
- Healthcare HL7/Insurance Edge Cases Pack
- IoT Device Fleet Chaos Pack
- How to install and use packs
- Creating custom packs
- Integration with scenarios and personas

**Reference**: `crates/mockforge-scenarios/src/reality_profile/packs.rs`

---

#### 1.2 Behavioral Economics Engine
**Status**: Missing from book  
**Priority**: High  
**Location**: `user-guide/advanced-features/behavioral-economics.md`

**Content Needed**:
- Overview of behavioral economics engine
- How mocks react to pressure, load, pricing, fraud suspicion, customer segments
- Example rules:
  - "Cart conversion drops if latency > 400ms"
  - "Bank declines transactions if prior balance checks failed"
  - "User churn increases after multiple 500s"
- Configuration examples
- Declarative vs scriptable rules

**Reference**: `crates/mockforge-core/src/behavioral_economics/`

---

#### 1.3 Synthetic → Recorded Drift Learning
**Status**: Missing from book  
**Priority**: Medium  
**Location**: `user-guide/advanced-features/drift-learning.md`

**Content Needed**:
- Overview of drift learning system
- How mocks learn from traffic patterns
- Persona behavior adaptation
- Traffic pattern mirroring
- Configuration options
- Learning modes (behavioral, statistical, hybrid)

**Reference**: `crates/mockforge-data/src/drift_learning.rs`

---

### 2. [Contracts] Features

#### 2.1 API Change Forecasting
**Status**: Missing from book  
**Priority**: High  
**Location**: `user-guide/contracts/api-change-forecasting.md`

**Content Needed**:
- Overview of change forecasting
- Predicting likely future breaks based on historical patterns
- Pattern analysis (seasonal patterns, volatility)
- Statistical modeling
- Multi-window forecasting (30/90/180 days)
- CLI commands and API usage

**Reference**: `crates/mockforge-core/src/contract_drift/forecasting/`

---

#### 2.2 Semantic Drift Notifications
**Status**: Missing from book  
**Priority**: High  
**Location**: `user-guide/contracts/semantic-drift.md`

**Content Needed**:
- Overview of semantic drift detection
- Detecting meaning changes (not just structure)
- Description change detection
- Enum narrowing detection
- Soft-breaking changes
- Nullable → non-nullable changes
- Error code removals
- LLM-powered semantic analysis

**Reference**: `crates/mockforge-core/src/ai_contract_diff/semantic_analyzer.rs`

---

#### 2.3 Contract Threat Modeling
**Status**: Missing from book  
**Priority**: High  
**Location**: `user-guide/contracts/threat-modeling.md`

**Content Needed**:
- Overview of contract threat modeling
- PII exposure detection
- DoS risk analysis (unbounded arrays)
- Error leakage detection (stack traces)
- Schema design analysis
- AI-powered remediation suggestions
- Multi-level assessment (workspace/service/endpoint)

**Reference**: `crates/mockforge-core/src/contract_drift/threat_modeling/`

---

### 3. [DevX] Features

#### 3.1 Zero-Config Mode (Runtime Daemon)
**Status**: Missing from book  
**Priority**: High  
**Location**: `user-guide/devx/zero-config-mode.md`

**Content Needed**:
- Overview of zero-config mode
- Runtime daemon functionality
- Auto-detection of 404 responses
- Automatic mock creation
- Type generation
- Client stub generation
- OpenAPI schema updates
- Scenario setup
- Configuration options

**Reference**: `crates/mockforge-runtime-daemon/`

---

#### 3.2 DevTools Browser Integration
**Status**: Partially covered (ForgeConnect SDK exists)  
**Priority**: Medium  
**Location**: Update `user-guide/forgeconnect-sdk.md` or create `user-guide/devx/devtools-integration.md`

**Content Needed**:
- DevTools panel features
- "Mock this endpoint" functionality
- Reverse-injection into workspace
- Live response modification
- Persona/scenario toggling in DevTools
- Integration with runtime daemon

**Reference**: `browser-extension/`

---

#### 3.3 Snapshot Diff Between Environments
**Status**: Missing from book  
**Priority**: Medium  
**Location**: `user-guide/devx/snapshot-diff.md`

**Content Needed**:
- Overview of snapshot diff feature
- Comparing test vs prod mock behavior
- Persona A vs Persona B comparison
- Reality 0.1 vs Reality 0.9 comparison
- Side-by-side visualization
- API usage

**Reference**: `crates/mockforge-http/src/handlers/snapshot_diff.rs`

---

### 4. [Cloud] Features

#### 4.1 Workspace Orchestration Pipelines (MockOps)
**Status**: Missing from book  
**Priority**: High  
**Location**: `user-guide/cloud/mockops-pipelines.md`

**Content Needed**:
- Overview of MockOps pipelines
- Event-driven automation
- Schema change → auto-regenerate SDK
- Scenario published → auto-promote to test → notify teams
- Drift threshold exceeded → auto-generate Git PR
- Pipeline definition (YAML)
- Available events and steps
- Integration examples

**Reference**: `docs/MOCKOPS_USER_GUIDE.md`, `crates/mockforge-pipelines/`

---

#### 4.2 Multi-Workspace Federation
**Status**: Missing from book  
**Priority**: High  
**Location**: `user-guide/cloud/federation.md`

**Content Needed**:
- Overview of multi-workspace federation
- Service boundary definition
- Composing workspaces into virtual systems
- System-wide scenarios
- Per-service reality level control
- Federation configuration
- Routing and coordination

**Reference**: `crates/mockforge-federation/`, `docs/MOCKOPS_PLATFORM.md`

---

#### 4.3 Team Heatmaps & Scenario Coverage
**Status**: Missing from book  
**Priority**: Medium  
**Location**: `user-guide/cloud/analytics-dashboard.md`

**Content Needed**:
- Overview of analytics dashboard
- Scenario usage heatmaps
- Persona CI hit tracking
- Endpoint coverage analysis
- Reality level staleness detection
- Drift percentage tracking
- Leadership insights

**Reference**: `crates/mockforge-ui/ui/src/components/analytics/`

---

### 5. [AI] Features

#### 5.1 LLM-Driven API Architecture Critique
**Status**: Missing from book  
**Priority**: High  
**Location**: `user-guide/ai/api-architecture-critique.md`

**Content Needed**:
- Overview of API architecture critique
- Anti-pattern detection
- Redundancy detection
- Naming quality assessment
- Emotional tone analysis
- Restructuring recommendations
- Usage examples
- UI integration

**Reference**: `crates/mockforge-core/src/ai_studio/api_critique.rs`

---

#### 5.2 NL → Entire System Generation
**Status**: Partially covered (LLM Studio exists)  
**Priority**: Medium  
**Location**: Update `user-guide/llm-studio.md` or create `user-guide/ai/system-generation.md`

**Content Needed**:
- Complete system generation from natural language
- Example: "I'm building a ride-sharing app..."
- Generated components:
  - 20-30 endpoints
  - 4-5 personas
  - 6-10 lifecycle states
  - WebSocket topics
  - Payment failure scenarios
  - Surge pricing chaos profiles
  - Full OpenAPI spec
  - GraphQL schema
  - Typings
  - CI pipeline templates
- Versioned artifacts
- Deterministic mode integration

**Reference**: `crates/mockforge-core/src/ai_studio/system_generator.rs`

---

#### 5.3 AI Behavioral Simulation Engine
**Status**: Missing from book  
**Priority**: High  
**Location**: `user-guide/ai/behavioral-simulation.md`

**Content Needed**:
- Overview of behavioral simulation
- Narrative agents
- Reacting to app state
- Forming intentions (shop, browse, buy, abandon)
- Responding to errors
- Multi-step interactions
- Behavior policies
- Usage examples

**Reference**: `crates/mockforge-core/src/ai_studio/behavioral_simulator.rs`

---

### 6. Cross-Pillar Features

#### 6.1 MockForge "World State" Engine
**Status**: Missing from book  
**Priority**: Medium  
**Location**: `user-guide/advanced-features/world-state-engine.md`

**Content Needed**:
- Overview of world state engine
- Unified state visualization
- Aggregating state from all subsystems
- Graph visualization
- Real-time updates
- Time travel
- Query interface
- Export capabilities

**Reference**: `crates/mockforge-world-state/`

---

#### 6.2 Mock-Oriented Development (MOD)
**Status**: Missing from book  
**Priority**: High  
**Location**: `user-guide/devx/mock-oriented-development.md` or new section

**Content Needed**:
- MOD philosophy and manifesto
- MOD vs TDD, BDD, IaC
- MOD principles
- MOD workflow
- MOD patterns
- MOD folder structures
- MOD API review process
- Getting started with MOD

**Reference**: `docs/MOD_PHILOSOPHY.md`, `docs/MOD_GUIDE.md`, `docs/MOD_PATTERNS.md`

---

#### 6.3 Performance Mode (Load Simulation)
**Status**: Missing from book  
**Priority**: Medium  
**Location**: `user-guide/advanced-features/performance-mode.md`

**Content Needed**:
- Overview of performance mode
- Running scenarios at n RPS
- Simulating bottlenecks
- Recording latencies
- Observing response changes under load
- Not true load testing - realistic behavior simulation
- Configuration and usage

**Reference**: `crates/mockforge-performance/`

---

## Updates to Existing Documentation

### Update `user-guide/advanced-features.md`
- Add links to new feature documentation
- Update feature list

### Update `getting-started/getting-started.md`
- Add mentions of new features in appropriate sections
- Update feature highlights

### Update `SUMMARY.md`
- Add new sections for:
  - Reality Profiles Marketplace
  - Behavioral Economics
  - API Change Forecasting
  - Semantic Drift
  - Contract Threat Modeling
  - Zero-Config Mode
  - Snapshot Diff
  - MockOps Pipelines
  - Federation
  - Analytics Dashboard
  - API Architecture Critique
  - Behavioral Simulation
  - World State Engine
  - MOD
  - Performance Mode

### Update `reference/changelog.md`
- Document all new features in changelog format

---

## Implementation Priority

### High Priority (Core Features)
1. Reality Profiles Marketplace
2. Behavioral Economics Engine
3. API Change Forecasting
4. Semantic Drift Notifications
5. Contract Threat Modeling
6. Zero-Config Mode
7. MockOps Pipelines
8. Federation
9. API Architecture Critique
10. Behavioral Simulation
11. MOD

### Medium Priority (Supporting Features)
1. Drift Learning
2. DevTools Integration (update existing)
3. Snapshot Diff
4. Analytics Dashboard
5. NL System Generation (update existing)
6. World State Engine
7. Performance Mode

---

## File Structure Recommendations

```
book/src/user-guide/
├── advanced-features/
│   ├── reality-profiles-marketplace.md (NEW)
│   ├── behavioral-economics.md (NEW)
│   ├── drift-learning.md (NEW)
│   ├── world-state-engine.md (NEW)
│   └── performance-mode.md (NEW)
├── contracts/
│   ├── api-change-forecasting.md (NEW)
│   ├── semantic-drift.md (NEW)
│   └── threat-modeling.md (NEW)
├── devx/
│   ├── zero-config-mode.md (NEW)
│   ├── snapshot-diff.md (NEW)
│   └── mock-oriented-development.md (NEW)
├── cloud/
│   ├── mockops-pipelines.md (NEW)
│   ├── federation.md (NEW)
│   └── analytics-dashboard.md (NEW)
└── ai/
    ├── api-architecture-critique.md (NEW)
    ├── behavioral-simulation.md (NEW)
    └── system-generation.md (NEW or update llm-studio.md)
```

---

## Next Steps

1. Create new documentation files for high-priority features
2. Update SUMMARY.md to include new sections
3. Update existing documentation to reference new features
4. Add examples and code snippets
5. Cross-reference related features
6. Update changelog

---

**Estimated Documentation Effort**: 
- High Priority: ~11 new documents
- Medium Priority: ~7 new/updated documents
- Total: ~18 documentation updates

